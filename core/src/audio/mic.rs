use crate::{event::EventBus, proto::Event, LoomError, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
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

async fn run_capture_loop(event_bus: Arc<EventBus>, mut config: MicConfig) -> Result<()> {
    // Choose host and input device
    let host = cpal::default_host();

    // Enumerate devices and optionally filter by name
    let input_device = if let Some(ref needle) = config.device_name {
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

    let input_device = input_device.ok_or_else(|| LoomError::EventBusError("No input device available".into()))?;
    let device_name = input_device.name().unwrap_or_else(|_| "unknown".into());

    // Resolve supported configs and pick the best matching one
    let supported_configs = input_device
        .supported_input_configs()
        .map_err(|e| LoomError::EventBusError(format!("failed to query supported input configs: {}", e)))?;

    // Preferred sample rates in order
    let preferred_rates = [16_000u32, 48_000u32, 32_000u32, 8_000u32];

    // Find a config that matches our desired channels and one of the preferred rates
    let mut chosen_config: Option<cpal::SupportedStreamConfig> = None;
    for cfg_range in supported_configs {
        if cfg_range.channels() != config.channels {
            continue;
        }
        let sample_format = cfg_range.sample_format();
        for &rate in &preferred_rates {
            if cfg_range.min_sample_rate().0 <= rate && cfg_range.max_sample_rate().0 >= rate {
                chosen_config = Some(cpal::SupportedStreamConfig::new(
                    cfg_range.channels(),
                    cpal::SampleRate(rate),
                    cfg_range.buffer_size().clone(),
                    sample_format,
                ));
                break;
            }
        }
        if chosen_config.is_some() {
            break;
        }
    }

    // If still none, fall back to default input config
    let chosen_config = match chosen_config {
        Some(c) => c,
        None => {
            warn!("Falling back to default input config; may not match requested sample rate/channels");
            input_device
                .default_input_config()
                .map_err(|e| LoomError::EventBusError(format!("failed to get default input config: {}", e)))?
        }
    };

    let actual_rate = chosen_config.sample_rate().0;
    let actual_channels = chosen_config.channels();
    if actual_rate != config.sample_rate_hz || actual_channels != config.channels {
        warn!(
            "Mic using rate={}Hz channels={} (requested {}Hz/{}ch)",
            actual_rate, actual_channels, config.sample_rate_hz, config.channels
        );
        // Update to actual for chunk sizing/metadata
        config.sample_rate_hz = actual_rate;
        config.channels = actual_channels;
    } else {
        info!(
            "Mic configured rate={}Hz channels={} device=\"{}\"",
            actual_rate, actual_channels, device_name
        );
    }

    // Create an async channel to pass samples from the audio callback to the publisher task
    // Capacity sized to a few chunks worth of samples
    let samples_per_chunk = ((config.sample_rate_hz as u64) * (config.chunk_ms as u64) / 1000) as usize * (config.channels as usize);
    let (tx, mut rx) = mpsc::channel::<Vec<i16>>(64);

    // Build input stream with proper sample type handling
    let stream_config: cpal::StreamConfig = chosen_config.clone().into();

    let err_fn = |err| {
        error!("cpal input stream error: {}", err);
    };

    // We aggregate samples in the audio thread into a buffer and send fixed-size chunks over mpsc
    let mut callback_acc: Vec<i16> = Vec::with_capacity(samples_per_chunk * 2);
    let tx_clone = tx.clone();

    let stream = match chosen_config.sample_format() {
        cpal::SampleFormat::I16 => build_input_stream::<i16>(&input_device, &stream_config, err_fn, move |data: &[i16]| {
            push_and_ship(data, &mut callback_acc, samples_per_chunk, &tx_clone);
        })?,
        cpal::SampleFormat::U16 => build_input_stream::<u16>(&input_device, &stream_config, err_fn, move |data: &[u16]| {
            let converted: Vec<i16> = data.iter().map(|&s| u16_to_i16(s)).collect();
            push_and_ship(&converted, &mut callback_acc, samples_per_chunk, &tx_clone);
        })?,
        cpal::SampleFormat::F32 => build_input_stream::<f32>(&input_device, &stream_config, err_fn, move |data: &[f32]| {
            let converted: Vec<i16> = data.iter().map(|&s| f32_to_i16(s)).collect();
            push_and_ship(&converted, &mut callback_acc, samples_per_chunk, &tx_clone);
        })?,
        other => {
            return Err(LoomError::EventBusError(format!("Unsupported sample format: {:?}", other)));
        }
    };

    stream
        .play()
        .map_err(|e| LoomError::EventBusError(format!("failed to start input stream: {}", e)))?;

    // Publisher loop: assemble events from received chunks
    let encoding = "pcm_s16le".to_string();
    let mut metadata_base: HashMap<String, String> = HashMap::new();
    metadata_base.insert("sample_rate".into(), config.sample_rate_hz.to_string());
    metadata_base.insert("channels".into(), config.channels.to_string());
    metadata_base.insert("device".into(), device_name.clone());
    metadata_base.insert("encoding".into(), encoding.clone());
    metadata_base.insert("chunk_ms".into(), config.chunk_ms.to_string());

    info!(
        "MicSource started: device=\"{}\" topic='{}' chunk={}ms rate={}Hz ch={}",
        device_name, config.topic, config.chunk_ms, config.sample_rate_hz, config.channels
    );

    // Keep a small buffer in case mpsc delivers partial multiples
    while let Some(mut buf) = rx.recv().await {
        // buf length should be exactly samples_per_chunk, but we’ll handle any length by slicing
        let mut offset = 0usize;
        while offset < buf.len() {
            let remain = buf.len() - offset;
            let take = remain.min(samples_per_chunk);
            let chunk = &buf[offset..offset + take];
            offset += take;

            // Serialize to little-endian bytes
            let mut payload = Vec::with_capacity(chunk.len() * 2);
            for sample in chunk.iter() {
                payload.extend_from_slice(&sample.to_le_bytes());
            }

            let mut metadata = metadata_base.clone();
            metadata.insert("frame_samples".into(), (chunk.len() as u32).to_string());

            let event = Event {
                id: gen_id(),
                r#type: "audio_chunk".into(),
                timestamp_ms: now_ms(),
                source: config.source.clone(),
                metadata,
                payload,
                confidence: 1.0,
                tags: vec![],
                priority: 90, // high priority
            };

            // Publish; ignore result value (delivered count)
            if let Err(e) = event_bus.publish(&config.topic, event).await {
                warn!("Failed to publish audio_chunk: {}", e);
            }
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
    T: cpal::Sample + cpal::FromSample<T> + Send + 'static,
    F: FnMut(&[T]) + Send + 'static,
{
    device
        .build_input_stream(config, move |data: &[T], _| on_data(data), err_fn, None)
        .map_err(|e| LoomError::EventBusError(format!("failed to build input stream: {}", e)))
}

fn push_and_ship(data: &[i16], acc: &mut Vec<i16>, chunk_samples: usize, tx: &mpsc::Sender<Vec<i16>>) {
    acc.extend_from_slice(data);
    while acc.len() >= chunk_samples {
        let chunk: Vec<i16> = acc.drain(..chunk_samples).collect();
        // Best-effort send; drop if the channel is full to keep realtime behavior
        let _ = tx.try_send(chunk);
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
