//! Microphone capture with Voice Activity Detection (VAD)
//!
//! This example demonstrates:
//! 1. Capturing audio from the default microphone
//! 2. Running VAD to detect speech segments
//! 3. Publishing vad.speech_start, vad.speech_end, and audio_voiced events
//!
//! Prerequisites:
//! - Linux: sudo apt-get install -y libasound2-dev pkg-config
//!
//! Configuration (via environment variables):
//! - MIC_DEVICE: Optional substring to match input device name
//! - MIC_CHUNK_MS: Chunk duration in milliseconds (default: 20)
//! - VAD_MODE: Aggressiveness 0-3 (default: 2, Aggressive)
//!   - 0 = Quality (most permissive)
//!   - 1 = LowBitrate
//!   - 2 = Aggressive
//!   - 3 = VeryAggressive (most strict)
//! - VAD_MIN_START_MS: Min consecutive voiced duration to trigger speech_start (default: 60)
//! - VAD_HANGOVER_MS: Hangover duration after last voice to trigger speech_end (default: 200)
//!
//! Run:
//!   cargo run -p loom-core --example mic_vad --features mic,vad
//!
//! Or with custom settings:
//!   VAD_MODE=3 VAD_MIN_START_MS=100 cargo run -p loom-core --example mic_vad --features mic,vad

use loom_core::audio::{MicConfig, MicSource, VadConfig, VadGate};
use loom_core::{Event, EventBus, QoSLevel, Result};
use std::sync::Arc;
use tracing::info;

/// Simple event logger that prints speech boundary events
fn handle_vad_event(event: &Event) {
    match event.r#type.as_str() {
        "vad.speech_start" => {
            let unknown = "?".to_string();
            let mode = event.metadata.get("mode").unwrap_or(&unknown);
            let rate = event.metadata.get("sample_rate").unwrap_or(&unknown);
            info!(
                "🎤 SPEECH START (mode={}, rate={}Hz) @ {}ms",
                mode, rate, event.timestamp_ms
            );
        }
        "vad.speech_end" => {
            let unknown = "?".to_string();
            let mode = event.metadata.get("mode").unwrap_or(&unknown);
            let rate = event.metadata.get("sample_rate").unwrap_or(&unknown);
            info!(
                "🤫 SPEECH END (mode={}, rate={}Hz) @ {}ms",
                mode, rate, event.timestamp_ms
            );
        }
        "audio_voiced" => {
            // Count voiced frames (optional, can comment out if too verbose)
            let frame_ms = event
                .metadata
                .get("frame_ms")
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0);
            info!(
                "🔊 VOICED frame ({}ms) @ {}ms",
                frame_ms, event.timestamp_ms
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

    info!("Starting Mic + VAD example...");

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

    info!("Mic config: {:?}", mic_config);

    // Configure VAD
    let vad_config = VadConfig::default();
    info!("VAD config: {:?}", vad_config);

    // Subscribe to VAD events (speech boundaries and voiced frames)
    let (_sub_id, mut vad_rx) = event_bus
        .subscribe(
            vad_config.vad_topic.clone(),
            vec!["vad.speech_start".to_string(), "vad.speech_end".to_string()],
            QoSLevel::QosRealtime,
        )
        .await?;

    let (_sub_id2, mut voiced_rx) = event_bus
        .subscribe(
            vad_config.voiced_topic.clone(),
            vec!["audio_voiced".to_string()],
            QoSLevel::QosRealtime,
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

    info!("🎙️  Listening... speak into your microphone!");
    info!("   (Press Ctrl+C to stop)");

    // Process events
    tokio::select! {
        _ = async {
            while let Some(event) = vad_rx.recv().await {
                handle_vad_event(&event);
            }
        } => {},
        _ = async {
            while let Some(event) = voiced_rx.recv().await {
                handle_vad_event(&event);
            }
        } => {},
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down...");
        }
    }

    event_bus.shutdown().await?;
    info!("✅ Shutdown complete");

    Ok(())
}
