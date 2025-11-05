//! Microphone capture EventSource using cpal.
//!
//! Linux build note: you need ALSA development headers for `cpal`.
//! On Debian/Ubuntu:
//!   sudo apt-get update && sudo apt-get install -y libasound2-dev pkg-config
//! Then run the example with:
//!   cargo run -p loom-core --example mic_capture --features mic
use crate::audio::utils::{gen_id, now_ms};
use crate::{event::EventBus, proto::Event, LoomError, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

/// Configuration for microphone capture
#[derive(Clone, Debug)]
pub struct MicConfig {
    /// Desired sample rate; will try 16_000 first, then fall back to common rates (48k/32k/8k)
    pub sample_rate_hz: u32,
    /// Desired channels; default mono
    pub channels: u16,
    /// Chunk size in milliseconds for emitted audio_chunk events
    pub chunk_ms: u32,
    /// Optional input device name substring to match
    pub device_name: Option<String>,
    /// Event topic to publish to (e.g., "audio.mic")
    pub topic: String,
    /// Event source name (e.g., "mic.primary")
    pub source: String,
}

impl Default for MicConfig {
    fn default() -> Self {
        let chunk_ms = std::env::var("MIC_CHUNK_MS")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(20);
        let topic = std::env::var("MIC_TOPIC").unwrap_or_else(|_| "audio.mic".to_string());
        let source = std::env::var("MIC_SOURCE").unwrap_or_else(|_| "mic.primary".to_string());
        let device_name = std::env::var("MIC_DEVICE").ok();
        Self {
            sample_rate_hz: 16_000,
            channels: 1,
            chunk_ms,
            device_name,
            topic,
            source,
        }
    }
}

/// Microphone event source: captures audio and publishes `audio_chunk` events
pub struct MicSource {
    event_bus: Arc<EventBus>,
    config: MicConfig,
}

impl MicSource {
    pub fn new(event_bus: Arc<EventBus>, config: MicConfig) -> Self {
        Self { event_bus, config }
    }

    /// Start the microphone capture loop. Returns a handle to the background task.
    pub async fn start(self) -> Result<JoinHandle<()>> {
        let cfg = self.config.clone();
        let event_bus = Arc::clone(&self.event_bus);

        // Spawn an async task to run capture and publish
        let handle = tokio::spawn(async move {
            if let Err(e) = run_capture_loop(event_bus, cfg).await {
                error!("MicSource stopped with error: {}", e);
            }
        });
        Ok(handle)
    }
}

// now_ms and gen_id are provided by audio::utils

struct AudioPacket {
    samples: Vec<i16>,
    sample_rate_hz: u32,
    channels: u16,
    device_name: String,
}

async fn run_capture_loop(event_bus: Arc<EventBus>, config: MicConfig) -> Result<()> {
    // Channel to receive audio chunks (with metadata) from the producer thread
    let (tx, mut rx) = mpsc::channel::<AudioPacket>(64);

    // Spawn a producer thread that owns the CPAL stream (non-Send)
    let cfg_for_thread = config.clone();
    std::thread::spawn(move || {
        // Choose host and input device
        let host = cpal::default_host();

        // Optional: Log available input devices and their basic capabilities
        if std::env::var("MIC_LOG_DEVICES").is_ok() {
            match host.input_devices() {
                Ok(devices) => {
                    info!("Listing input devices (set MIC_DEVICE to choose by substring):");
                    for dev in devices {
                        match dev.name() {
                            Ok(name) => {
                                let mut caps = String::new();
                                if let Ok(mut cfgs) = dev.supported_input_configs() {
                                    // Just summarize min..max sample rates and max channels seen
                                    let mut min_sr = u32::MAX;
                                    let mut max_sr = 0u32;
                                    let mut max_ch = 0u16;
                                    while let Some(r) = cfgs.next() {
                                        let min = r.min_sample_rate().0;
                                        let max = r.max_sample_rate().0;
                                        let ch = r.channels();
                                        if min < min_sr {
                                            min_sr = min;
                                        }
                                        if max > max_sr {
                                            max_sr = max;
                                        }
                                        if ch > max_ch {
                                            max_ch = ch;
                                        }
                                    }
                                    if min_sr != u32::MAX {
                                        caps = format!(
                                            " ({}-{} Hz, up to {}ch)",
                                            min_sr, max_sr, max_ch
                                        );
                                    }
                                }
                                info!(" • {}{}", name, caps);
                            }
                            Err(_) => info!(" • <unnamed device>"),
                        }
                    }
                }
                Err(e) => warn!("Failed to list input devices: {}", e),
            }
        }

        // Enumerate devices and optionally filter by name
        let input_device = if let Some(ref needle) = cfg_for_thread.device_name {
            let mut found: Option<cpal::Device> = None;
            match host.input_devices() {
                Ok(devices) => {
                    for dev in devices {
                        if let Ok(name) = dev.name() {
                            if name.to_lowercase().contains(&needle.to_lowercase()) {
                                found = Some(dev);
                                info!("Selected input device by MIC_DEVICE='{}': {}", needle, name);
                                break;
                            }
                        }
                    }
                }
                Err(e) => warn!("Failed to list input devices: {}", e),
            }
            found.or_else(|| host.default_input_device())
        } else {
            host.default_input_device()
        };

        let input_device = match input_device {
            Some(d) => d,
            None => {
                error!("No input device available");
                return;
            }
        };
        let device_name = input_device.name().unwrap_or_else(|_| "unknown".into());

        // Resolve supported configs and pick the best matching one
        let supported_configs = match input_device.supported_input_configs() {
            Ok(c) => c,
            Err(e) => {
                error!("failed to query supported input configs: {}", e);
                return;
            }
        };

        // Build candidate list across supported configs, preferring:
        // 1) Requested sample rate if available, otherwise 48k, 32k, 16k, 8k
        // 2) Higher-quality sample formats: F32 > I16 > U16 > U8
        // 3) Requested channel count; if unavailable, allow 2ch and downmix later
        let preferred_rates_primary = [
            cfg_for_thread.sample_rate_hz,
            48_000u32,
            32_000u32,
            16_000u32,
            8_000u32,
        ];

        #[derive(Clone)]
        struct Candidate {
            cfg: cpal::SupportedStreamConfig,
            rate: u32,
            channels: u16,
            fmt: cpal::SampleFormat,
            rate_rank: usize,
            fmt_rank: usize,
            ch_penalty: usize,
        }

        /// Return a relative "quality" rank for input sample formats when picking
        /// a capture configuration.
        ///
        /// Why F32 > I16 > U16 > U8?
        /// - F32 (3): Float input provides the most headroom and avoids device-side
        ///   clipping/AGC effects. Even though our pipeline normalizes to i16 for
        ///   events, capturing as f32 minimizes quantization, lets us clamp/scale
        ///   deterministically, and generally yields cleaner VAD/STT inputs. The
        ///   f32→i16 conversion cost is negligible compared to VAD/whisper.
        /// - I16 (2): Matches our internal PCM16 pipeline and is widely supported
        ///   with good fidelity on many devices/drivers.
        /// - U16 (1) / U8 (0): Unsigned PCM has a DC offset and reduced effective
        ///   dynamic range for speech. It requires re-centering and tends to come
        ///   from lower-quality paths. We handle conversion, but prefer signed/float
        ///   formats when available.
        ///
        /// Note: This ranking only influences capture selection. Regardless of the
        /// input format, we convert to i16 samples before emitting `audio_chunk`.
        fn fmt_rank(fmt: cpal::SampleFormat) -> usize {
            match fmt {
                cpal::SampleFormat::F32 => 3,
                cpal::SampleFormat::I16 => 2,
                cpal::SampleFormat::U16 => 1,
                cpal::SampleFormat::U8 => 0,
                _ => 0,
            }
        }

        let mut candidates: Vec<Candidate> = Vec::new();
        for cfg_range in supported_configs {
            let fmt = cfg_range.sample_format();
            let ch = cfg_range.channels();
            for (rank, &rate) in preferred_rates_primary.iter().enumerate() {
                if cfg_range.min_sample_rate().0 <= rate && cfg_range.max_sample_rate().0 >= rate {
                    let ch_penalty = if ch == cfg_for_thread.channels {
                        0
                    } else if ch == 2 {
                        1
                    } else {
                        2
                    };
                    candidates.push(Candidate {
                        cfg: cpal::SupportedStreamConfig::new(
                            ch,
                            cpal::SampleRate(rate),
                            cfg_range.buffer_size().clone(),
                            fmt,
                        ),
                        rate,
                        channels: ch,
                        fmt,
                        rate_rank: rank,
                        fmt_rank: fmt_rank(fmt),
                        ch_penalty,
                    });
                }
            }
        }

        // Sort by best quality: higher fmt_rank, lower ch_penalty, lower rate_rank
        candidates.sort_by(|a, b| {
            b.fmt_rank
                .cmp(&a.fmt_rank)
                .then(a.ch_penalty.cmp(&b.ch_penalty))
                .then(a.rate_rank.cmp(&b.rate_rank))
        });

        let chosen_config = if let Some(best) = candidates.first() {
            best.cfg.clone()
        } else {
            match input_device.default_input_config() {
                Ok(c) => c,
                Err(e) => {
                    error!("failed to get default input config: {}", e);
                    return;
                }
            }
        };

        let actual_rate = chosen_config.sample_rate().0;
        let actual_channels = chosen_config.channels();
        let fmt = chosen_config.sample_format();
        let fmt_str = match fmt {
            cpal::SampleFormat::F32 => "f32",
            cpal::SampleFormat::I16 => "i16",
            cpal::SampleFormat::U16 => "u16",
            cpal::SampleFormat::U8 => "u8",
            other => {
                warn!(
                    "Using uncommon sample format: {:?}. Audio quality may be degraded. Consider configuring your device to use F32 or I16 format.",
                    other
                );
                "other"
            }
        };
        if actual_rate != cfg_for_thread.sample_rate_hz
            || actual_channels != cfg_for_thread.channels
        {
            warn!(
                "Mic using rate={}Hz channels={} fmt={} (requested {}Hz/{}ch)",
                actual_rate,
                actual_channels,
                fmt_str,
                cfg_for_thread.sample_rate_hz,
                cfg_for_thread.channels
            );
        } else {
            info!(
                "Mic configured rate={}Hz channels={} device=\"{}\" fmt={}",
                actual_rate, actual_channels, device_name, fmt_str
            );
        }

        let samples_per_chunk = ((actual_rate as u64) * (cfg_for_thread.chunk_ms as u64) / 1000)
            as usize
            * (actual_channels as usize);

        // Build input stream with proper sample type handling
        let stream_config: cpal::StreamConfig = chosen_config.clone().into();

        let err_fn = |err| {
            error!("cpal input stream error: {}", err);
        };

        // Accumulator lives on this thread
        let mut callback_acc: Vec<i16> = Vec::with_capacity(samples_per_chunk * 2);
        let tx_clone = tx.clone();
        let dev_name_for_cb = device_name.clone();

        let build = || -> std::result::Result<cpal::Stream, LoomError> {
            match chosen_config.sample_format() {
                cpal::SampleFormat::I16 => build_input_stream::<i16, _>(
                    &input_device,
                    &stream_config,
                    err_fn,
                    move |data: &[i16]| {
                        let tx_inner = tx_clone.clone();
                        let dev_name = dev_name_for_cb.clone();
                        emit_chunks(data, &mut callback_acc, samples_per_chunk, |chunk| {
                            let _ = tx_inner.try_send(AudioPacket {
                                samples: chunk,
                                sample_rate_hz: actual_rate,
                                channels: actual_channels,
                                device_name: dev_name.clone(),
                            });
                        });
                    },
                ),
                cpal::SampleFormat::U8 => build_input_stream::<u8, _>(
                    &input_device,
                    &stream_config,
                    err_fn,
                    move |data: &[u8]| {
                        let converted: Vec<i16> = data.iter().map(|&s| u8_to_i16(s)).collect();
                        let tx_inner = tx_clone.clone();
                        let dev_name = dev_name_for_cb.clone();
                        emit_chunks(&converted, &mut callback_acc, samples_per_chunk, |chunk| {
                            let _ = tx_inner.try_send(AudioPacket {
                                samples: chunk,
                                sample_rate_hz: actual_rate,
                                channels: actual_channels,
                                device_name: dev_name.clone(),
                            });
                        });
                    },
                ),
                cpal::SampleFormat::U16 => build_input_stream::<u16, _>(
                    &input_device,
                    &stream_config,
                    err_fn,
                    move |data: &[u16]| {
                        let converted: Vec<i16> = data.iter().map(|&s| u16_to_i16(s)).collect();
                        let tx_inner = tx_clone.clone();
                        let dev_name = dev_name_for_cb.clone();
                        emit_chunks(&converted, &mut callback_acc, samples_per_chunk, |chunk| {
                            let _ = tx_inner.try_send(AudioPacket {
                                samples: chunk,
                                sample_rate_hz: actual_rate,
                                channels: actual_channels,
                                device_name: dev_name.clone(),
                            });
                        });
                    },
                ),
                cpal::SampleFormat::F32 => build_input_stream::<f32, _>(
                    &input_device,
                    &stream_config,
                    err_fn,
                    move |data: &[f32]| {
                        let converted: Vec<i16> = data.iter().map(|&s| f32_to_i16(s)).collect();
                        let tx_inner = tx_clone.clone();
                        let dev_name = dev_name_for_cb.clone();
                        emit_chunks(&converted, &mut callback_acc, samples_per_chunk, |chunk| {
                            let _ = tx_inner.try_send(AudioPacket {
                                samples: chunk,
                                sample_rate_hz: actual_rate,
                                channels: actual_channels,
                                device_name: dev_name.clone(),
                            });
                        });
                    },
                ),
                other => {
                    return Err(LoomError::EventBusError(format!(
                        "Unsupported sample format: {:?}",
                        other
                    )));
                }
            }
        };

        let stream = match build() {
            Ok(s) => s,
            Err(e) => {
                error!("failed to build input stream: {}", e);
                return;
            }
        };

        if let Err(e) = stream.play() {
            error!("failed to start input stream: {}", e);
            return;
        }

        info!(
            "MicSource started: device=\"{}\" chunk={}ms rate={}Hz ch={}",
            device_name, cfg_for_thread.chunk_ms, actual_rate, actual_channels
        );

        // Keep thread alive while stream runs; callbacks send packets via mpsc
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    });

    // Consumer: assemble and publish events
    while let Some(pkt) = rx.recv().await {
        // Serialize to little-endian bytes
        let mut payload = Vec::with_capacity(pkt.samples.len() * 2);
        for sample in pkt.samples.iter() {
            payload.extend_from_slice(&sample.to_le_bytes());
        }

        let mut metadata: HashMap<String, String> = HashMap::new();
        metadata.insert("sample_rate".into(), pkt.sample_rate_hz.to_string());
        metadata.insert("channels".into(), pkt.channels.to_string());
        metadata.insert("device".into(), pkt.device_name.clone());
        metadata.insert("encoding".into(), "pcm_s16le".into());
        metadata.insert(
            "frame_samples".into(),
            (payload.len() as u32 / 2).to_string(),
        );

        let event = Event {
            id: gen_id(),
            r#type: "audio_chunk".into(),
            timestamp_ms: now_ms(),
            source: config.source.clone(),
            metadata,
            payload,
            confidence: 1.0,
            tags: vec![],
            priority: 90,
        };

        if let Err(e) = event_bus.publish(&config.topic, event).await {
            warn!("Failed to publish audio_chunk: {}", e);
        }
    }

    Ok(())
}

fn build_input_stream<T, F>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    err_fn: fn(cpal::StreamError),
    mut on_data: F,
) -> Result<cpal::Stream>
where
    T: cpal::Sample + cpal::FromSample<T> + cpal::SizedSample + Send + 'static,
    F: FnMut(&[T]) + Send + 'static,
{
    device
        .build_input_stream(config, move |data: &[T], _| on_data(data), err_fn, None)
        .map_err(|e| LoomError::EventBusError(format!("failed to build input stream: {}", e)))
}

fn emit_chunks<F: FnMut(Vec<i16>)>(
    data: &[i16],
    acc: &mut Vec<i16>,
    chunk_samples: usize,
    mut emit: F,
) {
    acc.extend_from_slice(data);
    while acc.len() >= chunk_samples {
        let chunk: Vec<i16> = acc.drain(..chunk_samples).collect();
        emit(chunk);
    }
}

#[inline]
fn f32_to_i16(s: f32) -> i16 {
    let s = s.clamp(-1.0, 1.0);
    (s * i16::MAX as f32) as i16
}

#[inline]
fn u16_to_i16(s: u16) -> i16 {
    // Map 0..=65535 to -32768..=32767
    (s as i32 - 32768) as i16
}

#[inline]
fn u8_to_i16(s: u8) -> i16 {
    // Map 0..=255 unsigned to -32768..=32767 by centering at 128 and scaling
    (s as i16 - 128) << 8
}
