mod config;
use config::VoiceAgentConfig;
use loom_audio::{MicSource, SttEngine, VadGate, WakeWordDetector};
use loom_core::context::{PromptBundle, TokenBudget};
use loom_core::proto::QoSLevel;
use loom_core::Loom;
use serde_json::json;
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info};

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

    // Initialize Loom runtime essentials (event bus, tool registry, built-ins)
    let mut loom = Loom::new().await?;
    loom.start().await?;

    let bus = Arc::clone(&loom.event_bus);
    let registry = Arc::clone(&loom.tool_registry);

    // Load configuration (defaults + env + optional TOML overlay)
    let cfg = VoiceAgentConfig::load();

    // 1) Mic capture → audio.mic (audio_chunk)
    let mic = MicSource::new(Arc::clone(&bus), cfg.mic.clone());
    let mic_handle = mic.start().await?;

    // 2) VAD gating → vad (speech_start/end) and audio.voiced (audio_voiced)
    let vad = VadGate::new(Arc::clone(&bus), cfg.vad.clone());
    let vad_handle = vad.start().await?;

    // 3) STT utterance segmentation via whisper.cpp → transcript (transcript.final)
    let stt = SttEngine::new(Arc::clone(&bus), cfg.stt.clone());
    let stt_handle = stt.start().await?;

    // 4) Wake word on transcripts → wake (wake_word_detected) + query (user.query)
    let wake = WakeWordDetector::new(Arc::clone(&bus), cfg.wake.clone());
    let wake_handle = wake.start().await?;

    // Register local TTS capability as a Tool (moved to loom-audio)
    {
        let tts_cfg = cfg.build_tts_config();
        let tts = loom_audio::TtsSpeakProvider::new(Arc::clone(&bus), Some(tts_cfg));
        registry.register(Arc::new(tts)).await;
    }

    // 5) Subscribe to user queries → call LLM → TTS
    let (_sub_id, mut query_rx) = bus
        .subscribe(
            cfg.query_topic.clone(),
            vec!["user.query".to_string()],
            QoSLevel::QosBatched,
        )
        .await?;
    let llm_system = cfg.llm.system_prompt.clone();

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

            // Invoke LLM via tool registry (llm.generate)
            let args = json!({
                "bundle": bundle,
                "budget": budget,
                "headers": cfg.llm_headers(),
            });

            let res = registry.call("llm.generate", args).await;
            let reply_text = match res {
                Ok(val) => val
                    .get("text")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
                Err(e) => {
                    error!(target = "voice_agent", error = %e, "llm.generate call failed");
                    String::new()
                }
            };

            if reply_text.is_empty() {
                continue;
            }

            info!(target = "voice_agent", assistant = %reply_text, "🗣️  Speaking reply");

            // Invoke local TTS (tts.speak) via tool registry
            let tts_args = json!({
                "text": reply_text,
                "voice": cfg.tts_headers().get("voice").cloned().unwrap_or_default(),
                "rate": cfg.tts_headers().get("rate").and_then(|s| s.parse::<f32>().ok()),
                "volume": cfg.tts_headers().get("volume").and_then(|s| s.parse::<f32>().ok()),
            });
            match registry.call("tts.speak", tts_args).await {
                Ok(_) => {}
                Err(e) => {
                    error!(target = "voice_agent", error = %e, "TTS invocation failed");
                }
            }
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
