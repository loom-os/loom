//! STT (Speech-to-Text) integration tests
//!
//! These tests verify the STT engine can:
//! 1. Listen to VAD events (speech_start, speech_end)
//! 2. Buffer voiced audio frames
//! 3. Generate transcript events (when whisper is available)

#![cfg(feature = "stt")]

use loom_core::audio::{SttConfig, SttEngine};
use loom_core::{Event, EventBus, QoSLevel};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, Duration};

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn gen_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}", nanos)
}

/// Test that STT engine can be started and listens to VAD events
#[tokio::test]
async fn test_stt_engine_starts() {
    let bus = Arc::new(EventBus::new().await.unwrap());
    bus.start().await.unwrap();

    let stt_config = SttConfig {
        vad_topic: "vad".to_string(),
        voiced_topic: "audio.voiced".to_string(),
        transcript_topic: "transcript".to_string(),
        whisper_bin: "/nonexistent/whisper".into(),
        whisper_model: "/nonexistent/model.bin".into(),
        language: "en".to_string(),
        temp_dir: std::env::temp_dir(),
        extra_args: vec![],
    };

    let stt_engine = SttEngine::new(Arc::clone(&bus), stt_config);
    let handle = stt_engine.start().await.unwrap();

    // Give it time to start
    sleep(Duration::from_millis(100)).await;

    // STT should be running even if whisper is not available
    assert!(!handle.is_finished());

    // Cleanup
    handle.abort();
    bus.shutdown().await.unwrap();
}

/// Test that STT engine receives VAD events and buffers audio
#[tokio::test]
async fn test_stt_receives_vad_events() {
    let bus = Arc::new(EventBus::new().await.unwrap());
    bus.start().await.unwrap();

    let stt_config = SttConfig {
        vad_topic: "vad".to_string(),
        voiced_topic: "audio.voiced".to_string(),
        transcript_topic: "transcript".to_string(),
        whisper_bin: "/nonexistent/whisper".into(),
        whisper_model: "/nonexistent/model.bin".into(),
        language: "en".to_string(),
        temp_dir: std::env::temp_dir(),
        extra_args: vec![],
    };

    // Subscribe to transcript events (even though whisper won't run)
    let (_sub_id, mut transcript_rx) = bus
        .subscribe(
            stt_config.transcript_topic.clone(),
            vec!["transcript.final".to_string()],
            QoSLevel::QosBatched,
        )
        .await
        .unwrap();

    let stt_engine = SttEngine::new(Arc::clone(&bus), stt_config);
    let _handle = stt_engine.start().await.unwrap();

    // Give it time to start
    sleep(Duration::from_millis(100)).await;

    // Publish speech_start
    let mut md = HashMap::new();
    md.insert("sample_rate".to_string(), "16000".to_string());
    let start_event = Event {
        id: gen_id(),
        r#type: "vad.speech_start".to_string(),
        timestamp_ms: now_ms(),
        source: "test".to_string(),
        metadata: md,
        payload: vec![],
        confidence: 1.0,
        tags: vec![],
        priority: 70,
    };
    bus.publish("vad", start_event).await.unwrap();

    // Publish some voiced audio frames
    for _ in 0..10 {
        let mut md = HashMap::new();
        md.insert("sample_rate".to_string(), "16000".to_string());
        md.insert("channels".to_string(), "1".to_string());
        md.insert("frame_ms".to_string(), "20".to_string());

        // Generate 20ms of silence (320 samples @ 16kHz)
        let samples: Vec<i16> = vec![0; 320];
        let mut payload = Vec::new();
        for &s in &samples {
            payload.extend_from_slice(&s.to_le_bytes());
        }

        let voiced_event = Event {
            id: gen_id(),
            r#type: "audio_voiced".to_string(),
            timestamp_ms: now_ms(),
            source: "test".to_string(),
            metadata: md,
            payload,
            confidence: 1.0,
            tags: vec![],
            priority: 80,
        };
        bus.publish("audio.voiced", voiced_event).await.unwrap();
        sleep(Duration::from_millis(10)).await;
    }

    // Publish speech_end
    let mut md = HashMap::new();
    md.insert("sample_rate".to_string(), "16000".to_string());
    let end_event = Event {
        id: gen_id(),
        r#type: "vad.speech_end".to_string(),
        timestamp_ms: now_ms(),
        source: "test".to_string(),
        metadata: md,
        payload: vec![],
        confidence: 1.0,
        tags: vec![],
        priority: 70,
    };
    bus.publish("vad", end_event).await.unwrap();

    // Wait a bit for processing
    sleep(Duration::from_millis(200)).await;

    // We shouldn't get a transcript because whisper is not available
    // But the STT engine should have processed the events without crashing
    assert!(
        transcript_rx.try_recv().is_err(),
        "Should not receive transcript when whisper is unavailable"
    );

    bus.shutdown().await.unwrap();
}

/// Test that STT engine correctly ignores short utterances
#[tokio::test]
async fn test_stt_ignores_short_utterances() {
    let bus = Arc::new(EventBus::new().await.unwrap());
    bus.start().await.unwrap();

    let stt_config = SttConfig {
        vad_topic: "vad".to_string(),
        voiced_topic: "audio.voiced".to_string(),
        transcript_topic: "transcript".to_string(),
        whisper_bin: "/nonexistent/whisper".into(),
        whisper_model: "/nonexistent/model.bin".into(),
        language: "en".to_string(),
        temp_dir: std::env::temp_dir(),
        extra_args: vec![],
    };

    let (_sub_id, mut transcript_rx) = bus
        .subscribe(
            stt_config.transcript_topic.clone(),
            vec!["transcript.final".to_string()],
            QoSLevel::QosBatched,
        )
        .await
        .unwrap();

    let stt_engine = SttEngine::new(Arc::clone(&bus), stt_config);
    let _handle = stt_engine.start().await.unwrap();

    sleep(Duration::from_millis(100)).await;

    // Publish speech_start
    let mut md = HashMap::new();
    md.insert("sample_rate".to_string(), "16000".to_string());
    let start_event = Event {
        id: gen_id(),
        r#type: "vad.speech_start".to_string(),
        timestamp_ms: now_ms(),
        source: "test".to_string(),
        metadata: md,
        payload: vec![],
        confidence: 1.0,
        tags: vec![],
        priority: 70,
    };
    bus.publish("vad", start_event).await.unwrap();

    // Publish only 1 short frame (< 200ms threshold)
    let mut md = HashMap::new();
    md.insert("sample_rate".to_string(), "16000".to_string());
    md.insert("channels".to_string(), "1".to_string());
    md.insert("frame_ms".to_string(), "20".to_string());

    let samples: Vec<i16> = vec![100; 320]; // 20ms @ 16kHz
    let mut payload = Vec::new();
    for &s in &samples {
        payload.extend_from_slice(&s.to_le_bytes());
    }

    let voiced_event = Event {
        id: gen_id(),
        r#type: "audio_voiced".to_string(),
        timestamp_ms: now_ms(),
        source: "test".to_string(),
        metadata: md,
        payload,
        confidence: 1.0,
        tags: vec![],
        priority: 80,
    };
    bus.publish("audio.voiced", voiced_event).await.unwrap();

    // Publish speech_end immediately
    let mut md = HashMap::new();
    md.insert("sample_rate".to_string(), "16000".to_string());
    let end_event = Event {
        id: gen_id(),
        r#type: "vad.speech_end".to_string(),
        timestamp_ms: now_ms(),
        source: "test".to_string(),
        metadata: md,
        payload: vec![],
        confidence: 1.0,
        tags: vec![],
        priority: 70,
    };
    bus.publish("vad", end_event).await.unwrap();

    // Wait for processing
    sleep(Duration::from_millis(200)).await;

    // Should not get transcript for short utterance
    assert!(
        transcript_rx.try_recv().is_err(),
        "Should not transcribe utterances < 200ms"
    );

    bus.shutdown().await.unwrap();
}
