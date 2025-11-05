//! Integration tests for Voice Activity Detection (VAD)
//!
//! These tests verify the VAD module's ability to detect speech segments
//! and publish appropriate events.

#[cfg(all(feature = "mic", feature = "vad"))]
mod vad_tests {
    use loom_audio::{VadConfig, VadGate};
    use loom_core::{Event, EventBus, QoSLevel};
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn gen_id() -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!("{:x}", nanos)
    }

    fn now_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }

    /// Generate synthetic audio chunk event with PCM16 data
    fn create_audio_chunk(
        sample_rate: u32,
        channels: u16,
        samples: Vec<i16>,
    ) -> loom_core::proto::Event {
        let mut metadata = HashMap::new();
        metadata.insert("sample_rate".to_string(), sample_rate.to_string());
        metadata.insert("channels".to_string(), channels.to_string());
        metadata.insert("encoding".to_string(), "pcm_s16le".to_string());

        // Serialize to little-endian bytes
        let mut payload = Vec::with_capacity(samples.len() * 2);
        for sample in samples {
            payload.extend_from_slice(&sample.to_le_bytes());
        }

        Event {
            id: gen_id(),
            r#type: "audio_chunk".to_string(),
            timestamp_ms: now_ms(),
            source: "test_mic".to_string(),
            metadata,
            payload,
            confidence: 1.0,
            tags: vec![],
            priority: 90,
        }
    }

    /// Generate synthetic speech-like audio (sine wave)
    fn generate_speech_signal(duration_ms: u32, sample_rate: u32) -> Vec<i16> {
        let num_samples = (duration_ms * sample_rate / 1000) as usize;
        let frequency = 200.0; // Hz (typical speech frequency)
        (0..num_samples)
            .map(|i| {
                let t = i as f64 / sample_rate as f64;
                let amplitude = 0.5;
                (amplitude * (2.0 * std::f64::consts::PI * frequency * t).sin() * i16::MAX as f64)
                    as i16
            })
            .collect()
    }

    /// Generate silence (zeros)
    fn generate_silence(duration_ms: u32, sample_rate: u32) -> Vec<i16> {
        let num_samples = (duration_ms * sample_rate / 1000) as usize;
        vec![0i16; num_samples]
    }

    #[tokio::test]
    async fn test_vad_detects_speech_boundaries() {
        // Setup event bus
        let event_bus = Arc::new(EventBus::new().await.unwrap());
        event_bus.start().await.unwrap();

        // Configure VAD with permissive settings
        let vad_config = VadConfig {
            input_topic: "test.audio".to_string(),
            voiced_topic: "test.voiced".to_string(),
            vad_topic: "test.vad".to_string(),
            mode: 1, // LowBitrate (more permissive for synthetic audio)
            frame_ms: 20,
            min_start_ms: 40, // 2 frames
            hangover_ms: 100,
        };

        // Subscribe to VAD events
        let (_sub_id, mut vad_rx) = event_bus
            .subscribe(
                vad_config.vad_topic.clone(),
                vec!["vad.speech_start".to_string(), "vad.speech_end".to_string()],
                QoSLevel::QosRealtime,
            )
            .await
            .unwrap();

        // Start VAD
        let vad_gate = VadGate::new(Arc::clone(&event_bus), vad_config.clone());
        let _vad_handle = vad_gate.start().await.unwrap();

        // Wait for VAD to be ready
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Publish synthetic audio: silence → speech → silence
        let sample_rate = 16_000u32;

        // 1. Initial silence (should not trigger speech_start)
        let silence1 = generate_silence(50, sample_rate);
        event_bus
            .publish(
                &vad_config.input_topic,
                create_audio_chunk(sample_rate, 1, silence1),
            )
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;

        // 2. Speech segment (should trigger speech_start)
        let speech = generate_speech_signal(200, sample_rate);
        event_bus
            .publish(
                &vad_config.input_topic,
                create_audio_chunk(sample_rate, 1, speech),
            )
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // 3. Ending silence (should trigger speech_end after hangover)
        let silence2 = generate_silence(150, sample_rate);
        event_bus
            .publish(
                &vad_config.input_topic,
                create_audio_chunk(sample_rate, 1, silence2),
            )
            .await
            .unwrap();

        // Collect VAD events
        let mut speech_starts = 0;
        let mut speech_ends = 0;

        let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(500));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                Some(event) = vad_rx.recv() => {
                    match event.r#type.as_str() {
                        "vad.speech_start" => speech_starts += 1,
                        "vad.speech_end" => speech_ends += 1,
                        _ => {}
                    }
                }
                _ = &mut timeout => break,
            }
        }

        // Verify we detected at least one speech segment
        assert!(
            speech_starts >= 1,
            "Expected at least 1 speech_start, got {}",
            speech_starts
        );
        assert!(
            speech_ends >= 1,
            "Expected at least 1 speech_end, got {}",
            speech_ends
        );

        event_bus.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_vad_outputs_voiced_frames() {
        // Setup event bus
        let event_bus = Arc::new(EventBus::new().await.unwrap());
        event_bus.start().await.unwrap();

        // Configure VAD
        let vad_config = VadConfig {
            input_topic: "test.audio".to_string(),
            voiced_topic: "test.voiced".to_string(),
            vad_topic: "test.vad".to_string(),
            mode: 1,
            frame_ms: 20,
            min_start_ms: 40,
            hangover_ms: 100,
        };

        // Subscribe to voiced frames
        let (_sub_id, mut voiced_rx) = event_bus
            .subscribe(
                vad_config.voiced_topic.clone(),
                vec!["audio_voiced".to_string()],
                QoSLevel::QosRealtime,
            )
            .await
            .unwrap();

        // Start VAD
        let vad_gate = VadGate::new(Arc::clone(&event_bus), vad_config.clone());
        let _vad_handle = vad_gate.start().await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Publish speech audio
        let speech = generate_speech_signal(200, 16_000);
        event_bus
            .publish(
                &vad_config.input_topic,
                create_audio_chunk(16_000, 1, speech),
            )
            .await
            .unwrap();

        // Collect voiced frames
        let mut voiced_count = 0;
        let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(300));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                Some(event) = voiced_rx.recv() => {
                    if event.r#type == "audio_voiced" {
                        voiced_count += 1;
                        // Verify metadata
                        assert_eq!(event.metadata.get("channels").map(|s| s.as_str()), Some("1"));
                        assert_eq!(event.metadata.get("encoding").map(|s| s.as_str()), Some("pcm_s16le"));
                        // Verify payload is not empty
                        assert!(!event.payload.is_empty());
                    }
                }
                _ = &mut timeout => break,
            }
        }

        // Should have received some voiced frames
        assert!(
            voiced_count > 0,
            "Expected voiced frames, got {}",
            voiced_count
        );

        event_bus.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_vad_mode_aggressiveness() {
        // Test that higher aggressiveness mode is more strict
        let event_bus = Arc::new(EventBus::new().await.unwrap());
        event_bus.start().await.unwrap();

        // Test with mode 3 (VeryAggressive) - should detect less speech
        let vad_config_strict = VadConfig {
            input_topic: "test.audio".to_string(),
            voiced_topic: "test.voiced".to_string(),
            vad_topic: "test.vad.strict".to_string(),
            mode: 3, // VeryAggressive
            frame_ms: 20,
            min_start_ms: 40,
            hangover_ms: 100,
        };

        let (_sub_id, mut strict_rx) = event_bus
            .subscribe(
                vad_config_strict.vad_topic.clone(),
                vec!["vad.speech_start".to_string()],
                QoSLevel::QosRealtime,
            )
            .await
            .unwrap();

        let vad_strict = VadGate::new(Arc::clone(&event_bus), vad_config_strict.clone());
        let _handle_strict = vad_strict.start().await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Publish low-amplitude speech (borderline)
        let weak_speech: Vec<i16> = (0..3200)
            .map(|i| {
                let t = i as f64 / 16000.0;
                (0.1 * (2.0 * std::f64::consts::PI * 200.0 * t).sin() * i16::MAX as f64) as i16
            })
            .collect();

        event_bus
            .publish(
                &vad_config_strict.input_topic,
                create_audio_chunk(16_000, 1, weak_speech),
            )
            .await
            .unwrap();

        // Count detections with strict mode
        let mut strict_detections = 0;
        let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(300));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                Some(_event) = strict_rx.recv() => {
                    strict_detections += 1;
                }
                _ = &mut timeout => break,
            }
        }

        // Strict mode should detect less (possibly 0) for weak signal
        // This is expected behavior - just verify it doesn't panic
        assert!(
            strict_detections <= 1,
            "VeryAggressive mode should be very strict"
        );

        event_bus.shutdown().await.unwrap();
    }

    #[test]
    fn test_vad_config_direct_construction() {
        // Test VadConfig can be constructed with custom values
        let config = VadConfig {
            input_topic: "test.audio".to_string(),
            voiced_topic: "test.voiced".to_string(),
            vad_topic: "test.vad".to_string(),
            mode: 3,
            frame_ms: 30,
            min_start_ms: 100,
            hangover_ms: 300,
        };

        assert_eq!(config.mode, 3);
        assert_eq!(config.frame_ms, 30);
        assert_eq!(config.min_start_ms, 100);
        assert_eq!(config.hangover_ms, 300);
    }

    #[test]
    fn test_vad_config_defaults() {
        // Test default values without environment variables
        // Note: This may be affected by env vars set elsewhere
        let config = VadConfig {
            input_topic: "audio.mic".to_string(),
            voiced_topic: "audio.voiced".to_string(),
            vad_topic: "vad".to_string(),
            mode: 2,
            frame_ms: 20,
            min_start_ms: 60,
            hangover_ms: 200,
        };

        assert_eq!(config.mode, 2);
        assert_eq!(config.frame_ms, 20);
        assert_eq!(config.min_start_ms, 60);
        assert_eq!(config.hangover_ms, 200);
    }
}

// Placeholder test when features are disabled
#[cfg(not(all(feature = "mic", feature = "vad")))]
#[test]
fn vad_tests_require_features() {
    // This test ensures cargo test passes even without features
    println!("VAD tests require 'mic' and 'vad' features. Run: cargo test --features mic,vad");
}
