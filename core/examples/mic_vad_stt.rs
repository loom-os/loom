//! Microphone capture with Voice Activity Detection (VAD) and Speech-to-Text (STT)
//!
//! This example demonstrates the full speech pipeline:
//! 1. Capturing audio from the default microphone
//! 2. Running VAD to detect speech segments
//! 3. Transcribing speech segments using whisper.cpp
//! 4. Publishing transcript.final events
//!
//! Prerequisites:
//! - Linux: sudo apt-get install -y libasound2-dev pkg-config
//! - whisper.cpp: https://github.com/ggerganov/whisper.cpp
//!   git clone https://github.com/ggerganov/whisper.cpp
//!   cd whisper.cpp
//!   make
//!   bash ./models/download-ggml-model.sh base.en  # English-only (default)
//!
//! Configuration (via environment variables):
//! - MIC_DEVICE: Optional substring to match input device name
//! - MIC_CHUNK_MS: Chunk duration in milliseconds (default: 20)
//! - VAD_MODE: Aggressiveness 0-3 (default: 2)
//! - VAD_MIN_START_MS: Min consecutive voiced duration (default: 60ms)
//! - VAD_HANGOVER_MS: Hangover duration after last voice (default: 200ms)
//! - WHISPER_BIN: Path to whisper.cpp executable (default: "whisper")
//! - WHISPER_MODEL_PATH: Path to whisper model file (default: "ggml-base.en.bin")
//! - WHISPER_LANG: Language code (default: "en")
//! - WHISPER_EXTRA_ARGS: Extra whisper args, comma-separated (e.g., "--threads,4")
//!
//! Run:
//!   WHISPER_BIN=./whisper.cpp/build/bin/whisper-cli \
//!   WHISPER_MODEL_PATH=./whisper.cpp/models/ggml-base.en.bin \
//!   WHISPER_LANG=en \
//!   cargo run -p loom-core --example mic_vad_stt --features mic,vad,stt
//!
//! For Chinese (or other languages), switch to the multilingual model and set language:
//!   WHISPER_BIN=./whisper.cpp/build/bin/whisper-cli \
//!   WHISPER_MODEL_PATH=./whisper.cpp/models/ggml-base.bin \
//!   WHISPER_LANG=zh \
//!   cargo run -p loom-core --example mic_vad_stt --features mic,vad,stt

use loom_core::audio::{MicConfig, MicSource, SttConfig, SttEngine, VadConfig, VadGate};
#[cfg(feature = "wake")]
use loom_core::audio::{WakeWordConfig, WakeWordDetector};
use loom_core::{Event, EventBus, QoSLevel, Result};
use std::sync::Arc;
use tracing::info;

/// Simple event logger that prints speech and transcript events
fn handle_event(event: &Event) {
    const UNKNOWN: &str = "?";

    match event.r#type.as_str() {
        "vad.speech_start" => {
            let mode = event
                .metadata
                .get("mode")
                .map(|s| s.as_str())
                .unwrap_or(UNKNOWN);
            let rate = event
                .metadata
                .get("sample_rate")
                .map(|s| s.as_str())
                .unwrap_or(UNKNOWN);
            info!("🎤 SPEECH START (mode={}, rate={}Hz)", mode, rate);
        }
        "vad.speech_end" => {
            info!("🤫 SPEECH END");
        }
        "transcript.final" => {
            let text = event.metadata.get("text").map(|s| s.as_str()).unwrap_or("");
            let duration = event
                .metadata
                .get("duration_ms")
                .map(|s| s.as_str())
                .unwrap_or(UNKNOWN);
            let lang = event
                .metadata
                .get("language")
                .map(|s| s.as_str())
                .unwrap_or(UNKNOWN);
            info!(
                "📝 TRANSCRIPT [{}ms, lang={}]: \"{}\"",
                duration, lang, text
            );
        }
        _ => {}
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    info!("🚀 Starting Mic + VAD + STT example...");
    info!("");

    // Create event bus
    let event_bus = Arc::new(EventBus::new().await?);
    event_bus.start().await?;

    // Configure microphone (16kHz mono, 20ms chunks by default)
    let mic_config = MicConfig {
        sample_rate_hz: 16_000,
        channels: 1,
        chunk_ms: std::env::var("MIC_CHUNK_MS")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(20),
        device_name: std::env::var("MIC_DEVICE").ok(),
        topic: "audio.mic".to_string(),
        source: "mic.primary".to_string(),
    };

    info!("📊 Mic config: {:?}", mic_config);

    // Configure VAD
    let vad_config = VadConfig::default();
    info!("📊 VAD config: {:?}", vad_config);

    // Configure STT
    let stt_config = SttConfig::default();
    info!("📊 STT config: {:?}", stt_config);
    info!("");

    // Subscribe to VAD events
    let (_sub_id, mut vad_rx) = event_bus
        .subscribe(
            vad_config.vad_topic.clone(),
            vec!["vad.speech_start".to_string(), "vad.speech_end".to_string()],
            QoSLevel::QosRealtime,
        )
        .await?;

    // Subscribe to transcript events
    let (_sub_id2, mut transcript_rx) = event_bus
        .subscribe(
            stt_config.transcript_topic.clone(),
            vec!["transcript.final".to_string()],
            QoSLevel::QosBatched,
        )
        .await?;

    // Start microphone capture
    let mic_source = MicSource::new(Arc::clone(&event_bus), mic_config);
    let _mic_handle = mic_source.start().await?;
    info!("✅ Microphone started");

    // Start VAD gate
    let vad_gate = VadGate::new(Arc::clone(&event_bus), vad_config);
    let _vad_handle = vad_gate.start().await?;
    info!("✅ VAD gate started");

    // Start STT engine
    let stt_engine = SttEngine::new(Arc::clone(&event_bus), stt_config);
    let _stt_handle = stt_engine.start().await?;
    info!("✅ STT engine started");

    // Optionally start Wake Word detector (transcript-based)
    #[cfg(feature = "wake")]
    {
        let wake = WakeWordDetector::new(Arc::clone(&event_bus), WakeWordConfig::default());
        let _wake_handle = wake.start().await?;
        info!("✅ Wake detector started");
    }

    // Print the actual device used (from first audio_chunk metadata)
    {
        let (sub_id_dev, mut rx_dev) = event_bus
            .subscribe(
                "audio.mic".into(),
                vec!["audio_chunk".to_string()],
                QoSLevel::QosRealtime,
            )
            .await?;
        if let Some(ev) = rx_dev.recv().await {
            let dev = ev.metadata.get("device").cloned().unwrap_or_default();
            let rate = ev.metadata.get("sample_rate").cloned().unwrap_or_default();
            let ch = ev.metadata.get("channels").cloned().unwrap_or_default();
            info!(
                "🎛️  Mic device in use: \"{}\" ({} Hz, {} ch)",
                dev, rate, ch
            );
        }
        // Drop subscription
        let _ = event_bus.unsubscribe(&sub_id_dev).await;
    }
    info!("");
    info!("🎙️  Listening... speak into your microphone!");
    info!("   Your speech will be transcribed in real-time.");
    info!("   (Press Ctrl+C to stop)");
    info!("");

    // Process events
    #[cfg(not(feature = "wake"))]
    {
        tokio::select! {
            _ = async {
                while let Some(event) = vad_rx.recv().await {
                    handle_event(&event);
                }
            } => {},
            _ = async {
                while let Some(event) = transcript_rx.recv().await {
                    handle_event(&event);
                }
            } => {},
            _ = tokio::signal::ctrl_c() => {
                info!("Received Ctrl+C, shutting down...");
            }
        }
    }

    #[cfg(feature = "wake")]
    {
        // Subscribe to wake + query topics
        let (_w_id, mut w_rx) = event_bus
            .subscribe(
                "wake".into(),
                vec!["wake_word_detected".into()],
                QoSLevel::QosRealtime,
            )
            .await?;
        let (_q_id, mut q_rx) = event_bus
            .subscribe(
                "query".into(),
                vec!["user.query".into()],
                QoSLevel::QosRealtime,
            )
            .await?;

        tokio::select! {
            _ = async {
                while let Some(event) = vad_rx.recv().await {
                    handle_event(&event);
                }
            } => {},
            _ = async {
                while let Some(event) = transcript_rx.recv().await {
                    handle_event(&event);
                }
            } => {},
            _ = async {
                while let Some(event) = w_rx.recv().await {
                    let phrase = event.metadata.get("phrase").cloned().unwrap_or_default();
                    let sid = event.metadata.get("session_id").cloned().unwrap_or_default();
                    info!("🔔 WAKE: phrase='{}' session={}", phrase, sid);
                }
            } => {},
            _ = async {
                while let Some(event) = q_rx.recv().await {
                    let text = event.metadata.get("text").cloned().unwrap_or_default();
                    let sid = event.metadata.get("session_id").cloned().unwrap_or_default();
                    info!("💬 QUERY ({}): {}", sid, text);
                }
            } => {},
            _ = tokio::signal::ctrl_c() => {
                info!("Received Ctrl+C, shutting down...");
            }
        }
    }

    event_bus.shutdown().await?;
    info!("✅ Shutdown complete");

    Ok(())
}
