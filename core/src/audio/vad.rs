use crate::audio::utils::{gen_id, now_ms};
use crate::{event::EventBus, proto::Event, QoSLevel, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{error, warn};

/// Voice Activity Detection (VAD) configuration
#[derive(Clone, Debug)]
pub struct VadConfig {
    /// Input topic to subscribe (expects `audio_chunk` events)
    pub input_topic: String,
    /// Topic to publish voiced audio frames (event type: `audio_voiced`)
    pub voiced_topic: String,
    /// Topic to publish VAD boundary events: `vad.speech_start`/`vad.speech_end`
    pub vad_topic: String,
    /// Aggressiveness mode 0..=3 (higher = more aggressive = more strict speech)
    pub mode: i32,
    /// Frame size in milliseconds (allowed: 10, 20, 30). Default: 20
    pub frame_ms: u32,
    /// Minimum consecutive voiced duration to declare speech start (ms)
    pub min_start_ms: u32,
    /// Hangover duration after last voiced frame to declare speech end (ms)
    pub hangover_ms: u32,
}

impl Default for VadConfig {
    fn default() -> Self {
        let mode = std::env::var("VAD_MODE")
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .map(|m| m.clamp(0, 3))
            .unwrap_or(2);
        let frame_ms = std::env::var("VAD_FRAME_MS")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|&v| v == 10 || v == 20 || v == 30)
            .unwrap_or(20);
        let min_start_ms = std::env::var("VAD_MIN_START_MS")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(60);
        let hangover_ms = std::env::var("VAD_HANGOVER_MS")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(200);
        Self {
            input_topic: std::env::var("VAD_INPUT_TOPIC").unwrap_or_else(|_| "audio.mic".into()),
            voiced_topic: std::env::var("VAD_VOICED_TOPIC")
                .unwrap_or_else(|_| "audio.voiced".into()),
            vad_topic: std::env::var("VAD_TOPIC").unwrap_or_else(|_| "vad".into()),
            mode,
            frame_ms,
            min_start_ms,
            hangover_ms,
        }
    }
}

pub struct VadGate {
    bus: Arc<EventBus>,
    cfg: VadConfig,
}

impl VadGate {
    pub fn new(bus: Arc<EventBus>, cfg: VadConfig) -> Self {
        Self { bus, cfg }
    }

    pub async fn start(self) -> Result<JoinHandle<()>> {
        let bus = Arc::clone(&self.bus);
        let cfg = self.cfg.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = run_vad(bus, cfg).await {
                error!("VadGate stopped with error: {}", e);
            }
        });
        Ok(handle)
    }
}

// now_ms and gen_id are provided by audio::utils

async fn run_vad(bus: Arc<EventBus>, cfg: VadConfig) -> Result<()> {
    use webrtc_vad::{SampleRate, Vad, VadMode};

    // Subscribe to audio chunks (realtime)
    let (_sub_id, mut rx) = bus
        .subscribe(
            cfg.input_topic.clone(),
            vec!["audio_chunk".to_string()],
            QoSLevel::QosRealtime,
        )
        .await?;

    // State
    let mut in_speech = false;
    let frame_ms = cfg.frame_ms;

    // Counters (in frames)
    let mut consec_voiced = 0usize;
    let min_start_frames = (cfg.min_start_ms.max(frame_ms) + frame_ms - 1) / frame_ms;
    let mut hangover_left = 0isize;
    let hangover_frames = (cfg.hangover_ms + frame_ms - 1) / frame_ms;

    // Buffer the most recent speech frames prior to "speech_start" so we don't
    // lose the beginning of the utterance. We only keep voiced frames since the
    // last non-speech. On transition to speech, we will flush these frames first.
    use std::collections::VecDeque;
    let mut pre_speech_buffer: VecDeque<Vec<i16>> = VecDeque::new();

    while let Some(ev) = rx.recv().await {
        // Parse metadata
        let rate: u32 = ev
            .metadata
            .get("sample_rate")
            .and_then(|s| s.parse().ok())
            .unwrap_or(16_000);
        let channels: u16 = ev
            .metadata
            .get("channels")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        if !matches!(rate, 8000 | 16000 | 32000 | 48000) {
            warn!("VAD: unsupported sample_rate={}Hz; skipping", rate);
            continue;
        }

        // Decode payload as little-endian i16
        if ev.payload.len() % 2 != 0 {
            warn!(
                "VAD: uneven payload length for audio_chunk: {} bytes",
                ev.payload.len()
            );
            continue;
        }
        let mut samples: Vec<i16> = Vec::with_capacity(ev.payload.len() / 2);
        let mut it = ev.payload.chunks_exact(2);
        for b in &mut it {
            let s = i16::from_le_bytes([b[0], b[1]]);
            samples.push(s);
        }

        // Downmix to mono if needed (simple average across channels)
        let mono: Vec<i16> = if channels == 1 {
            samples
        } else {
            let mut m = Vec::with_capacity(samples.len() / channels as usize);
            let ch = channels as usize;
            for frame in samples.chunks_exact(ch) {
                let mut acc: i32 = 0;
                for &s in frame.iter().take(ch) {
                    acc += s as i32;
                }
                m.push((acc / ch as i32) as i16);
            }
            m
        };

        // Segment into VAD frames (10/20/30ms)
        let frame_len = (rate as usize) * (frame_ms as usize) / 1000;
        if frame_len == 0 {
            continue;
        }

        // Process all frames with VAD first (blocking, no await)
        // Use a struct to store decision + frame data
        struct FrameDecision {
            is_speech: bool,
            frame_data: Vec<i16>,
        }

        let decisions: Vec<FrameDecision> = {
            // Construct a VAD instance in a separate scope to ensure it's dropped
            let mut vad = Vad::new();
            let mode_variant = match cfg.mode {
                0 => VadMode::Quality,
                1 => VadMode::LowBitrate,
                2 => VadMode::Aggressive,
                3 => VadMode::VeryAggressive,
                _ => VadMode::Aggressive,
            };
            vad.set_mode(mode_variant);
            // Configure VAD with current sample rate
            let sample_rate = match rate {
                8000 => SampleRate::Rate8kHz,
                16000 => SampleRate::Rate16kHz,
                32000 => SampleRate::Rate32kHz,
                48000 => SampleRate::Rate48kHz,
                _ => SampleRate::Rate16kHz,
            };
            let _ = vad.set_sample_rate(sample_rate);

            // Collect VAD results and frames for all chunks
            let mut decisions: Vec<FrameDecision> = Vec::new();
            for frame in mono.chunks_exact(frame_len) {
                let is_speech = match vad.is_voice_segment(frame) {
                    Ok(b) => b,
                    Err(e) => {
                        warn!("VAD error: {:?}", e);
                        false
                    }
                };
                decisions.push(FrameDecision {
                    is_speech,
                    frame_data: frame.to_vec(),
                });
            }
            decisions
            // VAD is dropped here when the block exits
        };

        // Now process decisions and publish events (with await)
        for decision in decisions {
            let is_speech = decision.is_speech;

            if is_speech {
                consec_voiced += 1;
                hangover_left = hangover_frames as isize;
                // Accumulate only voiced frames until we enter speech
                if !in_speech {
                    pre_speech_buffer.push_back(decision.frame_data.clone());
                    // Cap at min_start_frames to avoid unbounded growth
                    if pre_speech_buffer.len() > min_start_frames as usize {
                        let excess = pre_speech_buffer.len() - min_start_frames as usize;
                        pre_speech_buffer.drain(0..excess);
                    }
                }
            } else {
                consec_voiced = 0;
                if in_speech && hangover_left > 0 {
                    hangover_left -= 1;
                }
                // Reset buffer on non-speech when not in active speech
                if !in_speech {
                    pre_speech_buffer.clear();
                }
            }

            // Transition logic
            if !in_speech {
                if consec_voiced >= min_start_frames as usize {
                    in_speech = true;
                    // Emit start
                    let mut md: HashMap<String, String> = HashMap::new();
                    md.insert("frame_ms".into(), frame_ms.to_string());
                    md.insert("mode".into(), cfg.mode.to_string());
                    md.insert("sample_rate".into(), rate.to_string());
                    let start = Event {
                        id: gen_id(),
                        r#type: "vad.speech_start".into(),
                        timestamp_ms: now_ms(),
                        source: "vad".into(),
                        metadata: md,
                        payload: vec![],
                        confidence: 1.0,
                        tags: vec![],
                        priority: 70,
                    };
                    if let Err(e) = bus.publish(&cfg.vad_topic, start).await {
                        warn!("Failed to publish vad.speech_start: {}", e);
                    }

                    // Immediately flush buffered pre-speech voiced frames so the
                    // start of the utterance is preserved. These frames are
                    // serialized and published as audio_voiced events with the
                    // same metadata used in the in-speech path.
                    while let Some(frame) = pre_speech_buffer.pop_front() {
                        let mut md = HashMap::new();
                        md.insert("sample_rate".into(), rate.to_string());
                        md.insert("channels".into(), "1".into());
                        md.insert("encoding".into(), "pcm_s16le".into());
                        md.insert("frame_ms".into(), frame_ms.to_string());

                        let mut payload = Vec::with_capacity(frame.len() * 2);
                        for &s in &frame {
                            payload.extend_from_slice(&s.to_le_bytes());
                        }

                        let voiced_ev = Event {
                            id: gen_id(),
                            r#type: "audio_voiced".into(),
                            timestamp_ms: now_ms(),
                            source: "vad".into(),
                            metadata: md,
                            payload,
                            confidence: 1.0,
                            tags: vec![],
                            priority: 80,
                        };
                        if let Err(e) = bus.publish(&cfg.voiced_topic, voiced_ev).await {
                            warn!("Failed to publish audio_voiced (pre-roll): {}", e);
                        }
                    }
                }
            } else {
                // in_speech
                // Forward voiced frames
                if is_speech {
                    let mut md = HashMap::new();
                    md.insert("sample_rate".into(), rate.to_string());
                    md.insert("channels".into(), "1".into());
                    md.insert("encoding".into(), "pcm_s16le".into());
                    md.insert("frame_ms".into(), frame_ms.to_string());

                    // Serialize frame to bytes
                    let mut payload = Vec::with_capacity(decision.frame_data.len() * 2);
                    for &s in &decision.frame_data {
                        payload.extend_from_slice(&s.to_le_bytes());
                    }

                    let voiced_ev = Event {
                        id: gen_id(),
                        r#type: "audio_voiced".into(),
                        timestamp_ms: now_ms(),
                        source: "vad".into(),
                        metadata: md,
                        payload,
                        confidence: 1.0,
                        tags: vec![],
                        priority: 80,
                    };
                    if let Err(e) = bus.publish(&cfg.voiced_topic, voiced_ev).await {
                        warn!("Failed to publish audio_voiced: {}", e);
                    }
                }

                // End if hangover expired
                if hangover_left <= 0 && !is_speech {
                    in_speech = false;
                    let mut md: HashMap<String, String> = HashMap::new();
                    md.insert("frame_ms".into(), frame_ms.to_string());
                    md.insert("mode".into(), cfg.mode.to_string());
                    md.insert("sample_rate".into(), rate.to_string());
                    let end = Event {
                        id: gen_id(),
                        r#type: "vad.speech_end".into(),
                        timestamp_ms: now_ms(),
                        source: "vad".into(),
                        metadata: md,
                        payload: vec![],
                        confidence: 1.0,
                        tags: vec![],
                        priority: 70,
                    };
                    if let Err(e) = bus.publish(&cfg.vad_topic, end).await {
                        warn!("Failed to publish vad.speech_end: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}
