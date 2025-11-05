use loom_audio::{MicConfig, SttConfig, VadConfig, WakeWordConfig};
use loom_core::context::{PromptBundle, TokenBudget};
use loom_core::proto::{ActionCall, QoSLevel};
use loom_core::{ActionBroker, EventBus, Loom};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Logging / tracing
    let filter = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "info,loom_core=info,voice_agent=info".to_string());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .init();

    info!(
        target = "voice_agent",
        "Starting Voice Agent demo: Mic → VAD → STT → Wake → LLM → TTS"
    );

    // Initialize Loom runtime essentials (event bus, action broker, built-ins)
    let mut loom = Loom::new().await?;
    loom.start().await?;

    let bus = Arc::clone(&loom.event_bus);
    let broker = Arc::clone(&loom.action_broker);

    // 1) Mic capture → audio.mic (audio_chunk)
    let mic_cfg = MicConfig::default();
    let mic = loom_audio::MicSource::new(Arc::clone(&bus), mic_cfg);
    let mic_handle = mic.start().await?;

    // 2) VAD gating → vad (speech_start/end) and audio.voiced (audio_voiced)
    let vad_cfg = VadConfig::default();
    let vad = loom_audio::VadGate::new(Arc::clone(&bus), vad_cfg);
    let vad_handle = vad.start().await?;

    // 3) STT utterance segmentation via whisper.cpp → transcript (transcript.final)
    let stt_cfg = SttConfig::default();
    let stt = loom_audio::SttEngine::new(Arc::clone(&bus), stt_cfg);
    let stt_handle = stt.start().await?;

    // 4) Wake word on transcripts → wake (wake_word_detected) + query (user.query)
    let wake_cfg = WakeWordConfig::default();
    let wake = loom_audio::WakeWordDetector::new(Arc::clone(&bus), wake_cfg);
    let wake_handle = wake.start().await?;

    // Register local TTS capability provider (moved to loom-audio)
    {
        let tts = loom_audio::TtsSpeakProvider::new(Arc::clone(&bus), None);
        broker.register_provider(Arc::new(tts));
    }

    // 5) Subscribe to user queries → call LLM → TTS
    let (_sub_id, mut query_rx) = bus
        .subscribe(
            std::env::var("QUERY_TOPIC").unwrap_or_else(|_| "query".to_string()),
            vec!["user.query".to_string()],
            QoSLevel::QosBatched,
        )
        .await?;

    let llm_system = std::env::var("VOICE_SYSTEM_PROMPT").unwrap_or_else(|_| {
        "You are Loom's helpful and concise voice assistant. Answer briefly and clearly.".into()
    });

    // Spawn task to process queries
    let broker_task = tokio::spawn(async move {
        while let Some(ev) = query_rx.recv().await {
            let text = ev
                .metadata
                .get("text")
                .cloned()
                .unwrap_or_else(|| String::from_utf8_lossy(&ev.payload).to_string());
            if text.trim().is_empty() {
                continue;
            }
            info!(target = "voice_agent", user_query = %text, "➡️  Received user.query");

            // Assemble a minimal PromptBundle
            let bundle = PromptBundle {
                system: llm_system.clone(),
                instructions: text.clone(),
                tools_json_schema: None,
                context_docs: vec![],
                history: vec![],
            };
            let budget = TokenBudget {
                max_input_tokens: 2048,
                max_output_tokens: 256,
            };

            // Invoke LLM via action broker (llm.generate)
            let call_id = format!("call_{}", current_millis());
            let payload = json!({
                "bundle": bundle,
                "budget": budget,
            })
            .to_string()
            .into_bytes();

            let mut headers = HashMap::new();
            // Allow dynamic overrides via env
            if let Ok(model) = std::env::var("VLLM_MODEL") {
                if !model.is_empty() {
                    headers.insert("model".into(), model);
                }
            }
            if let Ok(base) = std::env::var("VLLM_BASE_URL") {
                if !base.is_empty() {
                    headers.insert("base_url".into(), base);
                }
            }
            if let Ok(t) = std::env::var("VLLM_TEMPERATURE") {
                headers.insert("temperature".into(), t);
            }
            if let Ok(rt) = std::env::var("REQUEST_TIMEOUT_MS") {
                headers.insert("request_timeout_ms".into(), rt);
            }

            let call = ActionCall {
                id: call_id.clone(),
                capability: "llm.generate".to_string(),
                version: "0.1.0".to_string(),
                payload,
                headers,
                timeout_ms: 60_000, // generous budget
                correlation_id: ev.id.clone(),
                qos: QoSLevel::QosBatched as i32,
            };

            let res = broker.invoke(call).await;
            let reply_text = match res {
                Ok(r) if r.status == (loom_core::proto::ActionStatus::ActionOk as i32) => {
                    let v: Result<serde_json::Value, _> = serde_json::from_slice(&r.output);
                    match v {
                        Ok(val) => val.get("text")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .to_string(),
                        Err(e) => {
                            warn!(
                                target = "voice_agent",
                                error = %e,
                                raw_output = ?r.output,
                                "Failed to deserialize LLM response"
                            );
                            String::new()
                        }
                    }
                }
                Ok(r) => {
                    warn!(
                        target = "voice_agent",
                        status = r.status,
                        "llm.generate returned non-OK status"
                    );
                    String::new()
                }
                Err(e) => {
                    error!(target = "voice_agent", error = %e, "llm.generate invoke failed");
                    String::new()
                }
            };

            if reply_text.is_empty() {
                continue;
            }

            info!(target = "voice_agent", assistant = %reply_text, "🗣️  Speaking reply");

            // Invoke local TTS (tts.speak). It was registered when building Loom with feature "tts".
            let tts_call = ActionCall {
                id: format!("call_{}", current_millis()),
                capability: "tts.speak".to_string(),
                version: "0.1.0".to_string(),
                // Safe to unwrap: serializing a simple JSON object with a string field should never fail.
                payload: serde_json::to_vec(&json!({"text": reply_text})).unwrap(),
                headers: tts_headers_from_env(),
                timeout_ms: 30_000,
                correlation_id: ev.id.clone(),
                qos: QoSLevel::QosRealtime as i32,
            };
            let _ = broker.invoke(tts_call).await;
        }
    });

    // Ctrl+C handler to shutdown gracefully
    let shutdown = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    tokio::select! {
        _ = shutdown => {
            info!(target = "voice_agent", "Shutting down...");
        }
    }

    // Stop background tasks (they are long-running but will exit on process end)
    let _ = mic_handle.abort();
    let _ = vad_handle.abort();
    let _ = stt_handle.abort();
    let _ = wake_handle.abort();
    let _ = broker_task.abort();

    loom.shutdown().await.ok();
    Ok(())
}

fn current_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time before UNIX_EPOCH")
        .as_millis() as i64
}

fn tts_headers_from_env() -> HashMap<String, String> {
    let mut h = HashMap::new();
    if let Ok(v) = std::env::var("TTS_VOICE") {
        h.insert("voice".into(), v);
    }
    if let Ok(v) = std::env::var("TTS_RATE") {
        h.insert("rate".into(), v);
    }
    if let Ok(v) = std::env::var("TTS_VOLUME") {
        h.insert("volume".into(), v);
    }
    if let Ok(v) = std::env::var("TTS_SAMPLE_RATE") {
        h.insert("sample_rate".into(), v);
    }
    if let Ok(v) = std::env::var("TTS_PLAYER") {
        h.insert("player".into(), v);
    }
    h
}
