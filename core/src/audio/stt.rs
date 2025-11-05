use crate::audio::utils::{gen_id, now_ms};
use crate::{event::EventBus, proto::Event, QoSLevel, Result};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

/// STT (Speech-to-Text) configuration
#[derive(Clone, Debug)]
pub struct SttConfig {
    /// Input topic to subscribe to VAD events (`vad.speech_start`, `vad.speech_end`)
    pub vad_topic: String,
    /// Input topic to subscribe to voiced audio frames (`audio_voiced`)
    pub voiced_topic: String,
    /// Output topic to publish transcript events (`transcript.final`)
    pub transcript_topic: String,
    /// Path to whisper.cpp executable (e.g., "./whisper.cpp/main")
    pub whisper_bin: PathBuf,
    /// Path to whisper model file (e.g., "./models/ggml-base.en.bin")
    pub whisper_model: PathBuf,
    /// Language for transcription (e.g., "en", "zh", "auto")
    pub language: String,
    /// Temporary directory for WAV files
    pub temp_dir: PathBuf,
    /// Additional whisper.cpp arguments (e.g., ["--threads", "4"])
    pub extra_args: Vec<String>,
}

impl Default for SttConfig {
    fn default() -> Self {
        let whisper_bin = std::env::var("WHISPER_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("whisper"));
        let whisper_model = std::env::var("WHISPER_MODEL_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("ggml-base.en.bin"));
        // Default to English-only model + language for best OOTB speed/accuracy.
        // Switch to multilingual by pointing WHISPER_MODEL_PATH to ggml-base.bin and
        // optionally setting WHISPER_LANG=zh (or appropriate language code).
        let language = std::env::var("WHISPER_LANG").unwrap_or_else(|_| "en".to_string());
        let temp_dir = std::env::var("STT_TEMP_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir());

        // Parse extra args from env (comma-separated)
        let extra_args = std::env::var("WHISPER_EXTRA_ARGS")
            .ok()
            .map(|s| {
                s.split(',')
                    .map(|arg| arg.trim().to_string())
                    .filter(|arg| !arg.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        Self {
            vad_topic: std::env::var("STT_VAD_TOPIC").unwrap_or_else(|_| "vad".into()),
            voiced_topic: std::env::var("STT_VOICED_TOPIC")
                .unwrap_or_else(|_| "audio.voiced".into()),
            transcript_topic: std::env::var("STT_TRANSCRIPT_TOPIC")
                .unwrap_or_else(|_| "transcript".into()),
            whisper_bin,
            whisper_model,
            language,
            temp_dir,
            extra_args,
        }
    }
}

pub struct SttEngine {
    bus: Arc<EventBus>,
    cfg: SttConfig,
}

impl SttEngine {
    pub fn new(bus: Arc<EventBus>, cfg: SttConfig) -> Self {
        Self { bus, cfg }
    }

    pub async fn start(self) -> Result<JoinHandle<()>> {
        let bus = Arc::clone(&self.bus);
        let cfg = self.cfg.clone();

        // Validate whisper binary exists
        if !cfg.whisper_bin.exists() {
            warn!(
                "⚠️  Whisper binary not found at {:?}. STT will be skipped.",
                cfg.whisper_bin
            );
            warn!("   Please set WHISPER_BIN environment variable or install whisper.cpp.");
            warn!("   See: https://github.com/ggerganov/whisper.cpp");
        } else {
            info!("✓ Found whisper binary at {:?}", cfg.whisper_bin);
        }

        if !cfg.whisper_model.exists() {
            warn!(
                "⚠️  Whisper model not found at {:?}. STT will be skipped.",
                cfg.whisper_model
            );
            warn!("   Please set WHISPER_MODEL_PATH or download a model.");
        } else {
            info!("✓ Found whisper model at {:?}", cfg.whisper_model);
        }

        let handle = tokio::spawn(async move {
            if let Err(e) = run_stt(bus, cfg).await {
                error!("SttEngine stopped with error: {}", e);
            }
        });
        Ok(handle)
    }
}

// now_ms and gen_id are provided by audio::utils

/// Utterance buffer for audio frames between speech_start and speech_end
#[derive(Debug)]
struct Utterance {
    frames: Vec<Vec<i16>>,
    sample_rate: u32,
    start_time_ms: i64,
}

impl Utterance {
    fn new(sample_rate: u32) -> Self {
        Self {
            frames: Vec::new(),
            sample_rate,
            start_time_ms: now_ms(),
        }
    }

    fn add_frame(&mut self, frame: Vec<i16>) {
        self.frames.push(frame);
    }

    fn total_samples(&self) -> usize {
        self.frames.iter().map(|f| f.len()).sum()
    }

    fn duration_ms(&self) -> u64 {
        let total_samples = self.total_samples();
        if self.sample_rate == 0 {
            return 0;
        }
        (total_samples as u64 * 1000) / self.sample_rate as u64
    }

    fn to_pcm(&self) -> Vec<i16> {
        let mut pcm = Vec::with_capacity(self.total_samples());
        for frame in &self.frames {
            pcm.extend_from_slice(frame);
        }
        pcm
    }
}

async fn run_stt(bus: Arc<EventBus>, cfg: SttConfig) -> Result<()> {
    // Check if dependencies are available
    let has_whisper = cfg.whisper_bin.exists() && cfg.whisper_model.exists();
    if !has_whisper {
        warn!("Whisper dependencies not available, STT disabled");
        // Keep running but just consume events without processing
    }

    // Subscribe to VAD events (speech_start, speech_end)
    let (_vad_sub_id, mut vad_rx) = bus
        .subscribe(
            cfg.vad_topic.clone(),
            vec!["vad.speech_start".to_string(), "vad.speech_end".to_string()],
            QoSLevel::QosRealtime,
        )
        .await?;

    // Subscribe to voiced audio frames
    let (_voiced_sub_id, mut voiced_rx) = bus
        .subscribe(
            cfg.voiced_topic.clone(),
            vec!["audio_voiced".to_string()],
            QoSLevel::QosRealtime,
        )
        .await?;

    // State: current utterance being recorded
    let utterance = Arc::new(Mutex::new(Option::<Utterance>::None));

    // Spawn a task to handle VAD events
    let bus_vad = Arc::clone(&bus);
    let cfg_vad = cfg.clone();
    let utterance_vad = Arc::clone(&utterance);
    let vad_task = tokio::spawn(async move {
        while let Some(ev) = vad_rx.recv().await {
            match ev.r#type.as_str() {
                "vad.speech_start" => {
                    let sample_rate: u32 = ev
                        .metadata
                        .get("sample_rate")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(16_000);

                    info!("🎤 Speech started (sample_rate={}Hz)", sample_rate);
                    let mut utt = utterance_vad.lock().await;
                    *utt = Some(Utterance::new(sample_rate));
                }
                "vad.speech_end" => {
                    info!("🎤 Speech ended, processing utterance...");
                    let mut utt = utterance_vad.lock().await;
                    if let Some(utterance) = utt.take() {
                        // Process the utterance
                        if has_whisper {
                            if let Err(e) = process_utterance(&bus_vad, &cfg_vad, utterance).await {
                                error!("Failed to process utterance: {}", e);
                            }
                        } else {
                            debug!(
                                "Skipping STT processing (whisper not available), {} samples buffered",
                                utterance.total_samples()
                            );
                        }
                    } else {
                        warn!("speech_end received but no active utterance");
                    }
                }
                _ => {}
            }
        }
    });

    // Spawn a task to handle voiced audio frames
    let utterance_voiced = Arc::clone(&utterance);
    let voiced_task = tokio::spawn(async move {
        while let Some(ev) = voiced_rx.recv().await {
            if ev.r#type == "audio_voiced" {
                // Decode payload as i16 samples
                if ev.payload.len() % 2 != 0 {
                    warn!(
                        "audio_voiced payload length ({}) is not even, expected i16 samples (2 bytes each)",
                        ev.payload.len()
                    );
                    continue;
                }

                let mut samples: Vec<i16> = Vec::with_capacity(ev.payload.len() / 2);
                for chunk in ev.payload.chunks_exact(2) {
                    let s = i16::from_le_bytes([chunk[0], chunk[1]]);
                    samples.push(s);
                }

                // Add to current utterance if active
                let mut utt = utterance_voiced.lock().await;
                if let Some(utterance) = utt.as_mut() {
                    utterance.add_frame(samples);
                }
            }
        }
    });

    // Wait for both tasks
    let _ = tokio::try_join!(vad_task, voiced_task);

    Ok(())
}

async fn process_utterance(
    bus: &Arc<EventBus>,
    cfg: &SttConfig,
    utterance: Utterance,
) -> Result<()> {
    let duration = utterance.duration_ms();
    let samples = utterance.total_samples();

    info!(
        "Processing utterance: {} samples, {}ms @ {}Hz",
        samples, duration, utterance.sample_rate
    );

    // Skip very short utterances (< 200ms)
    if duration < 200 {
        info!("Utterance too short, skipping transcription");
        return Ok(());
    }

    // Convert to PCM
    let pcm = utterance.to_pcm();

    // Write to temporary WAV file
    let wav_path = cfg.temp_dir.join(format!("utterance_{}.wav", gen_id()));
    if let Err(e) = write_wav_file(&wav_path, &pcm, utterance.sample_rate, 1) {
        error!("Failed to write WAV file: {}", e);
        return Ok(());
    }

    info!("💾 Wrote WAV file: {:?} ({} samples)", wav_path, pcm.len());

    // Call whisper.cpp
    let transcript = match transcribe_with_whisper(cfg, &wav_path).await {
        Ok(text) => text,
        Err(e) => {
            error!("Transcription failed: {}", e);
            // Clean up WAV file (unless debug mode)
            if std::env::var("STT_KEEP_WAV").is_err() {
                let _ = std::fs::remove_file(&wav_path);
            }
            return Ok(());
        }
    };

    // Clean up WAV file (unless debug mode)
    let keep_wav = std::env::var("STT_KEEP_WAV").is_ok();
    if keep_wav {
        info!("🔍 Kept WAV file for debugging: {:?}", wav_path);
    } else {
        let _ = std::fs::remove_file(&wav_path);
    }

    // Publish transcript event
    if !transcript.is_empty() {
        info!("📝 Transcript: {}", transcript);

        let mut metadata = HashMap::new();
        metadata.insert("sample_rate".to_string(), utterance.sample_rate.to_string());
        metadata.insert("duration_ms".to_string(), duration.to_string());
        metadata.insert("language".to_string(), cfg.language.clone());
        metadata.insert("text".to_string(), transcript.clone());

        let event = Event {
            id: gen_id(),
            r#type: "transcript.final".to_string(),
            timestamp_ms: now_ms(),
            source: "stt".to_string(),
            metadata,
            payload: transcript.into_bytes(),
            confidence: 1.0,
            tags: vec![],
            priority: 70,
        };

        if let Err(e) = bus.publish(&cfg.transcript_topic, event).await {
            error!("Failed to publish transcript event: {}", e);
        }
    } else {
        info!("Empty transcript, skipping publish");
    }

    Ok(())
}

async fn transcribe_with_whisper(cfg: &SttConfig, wav_path: &PathBuf) -> Result<String> {
    // Build whisper command
    // Example: ./whisper.cpp/main -m ./models/ggml-base.en.bin -f input.wav -l en --no-timestamps
    let mut cmd = Command::new(&cfg.whisper_bin);
    cmd.arg("-m").arg(&cfg.whisper_model);
    cmd.arg("-f").arg(wav_path);

    if !cfg.language.is_empty() && cfg.language != "auto" {
        cmd.arg("-l").arg(&cfg.language);
    }

    // Add flags for cleaner output
    cmd.arg("--no-timestamps");
    cmd.arg("--no-prints");

    // Add extra args from config
    for arg in &cfg.extra_args {
        cmd.arg(arg);
    }

    debug!("Running whisper command: {:?}", cmd);

    // Run command and capture output
    let output = tokio::task::spawn_blocking(move || cmd.output())
        .await
        .map_err(|e| crate::LoomError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?
        .map_err(|e| crate::LoomError::IoError(e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(crate::LoomError::IoError(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Whisper failed with status {}: {}", output.status, stderr),
        )));
    }

    // Parse output - whisper.cpp writes transcript to stdout
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Log full output for debugging
    debug!("Whisper stdout: {}", stdout);
    debug!("Whisper stderr: {}", stderr);

    let transcript = stdout
        .lines()
        .filter(|line| {
            // Filter out progress/status lines, keep actual transcript
            !line.starts_with('[')
                && !line.trim().is_empty()
                && !line.contains("whisper_")
                && !line.contains("load time")
                && !line.contains("system_info")
        })
        .map(|line| line.trim())
        .collect::<Vec<_>>()
        .join(" ");

    Ok(transcript)
}

/// Write PCM samples to a WAV file
fn write_wav_file(
    path: &PathBuf,
    samples: &[i16],
    sample_rate: u32,
    channels: u16,
) -> std::io::Result<()> {
    let mut file = std::fs::File::create(path)?;

    // WAV header
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * channels as u32 * (bits_per_sample as u32 / 8);
    let block_align = channels * (bits_per_sample / 8);
    let data_size = (samples.len() * 2) as u32;
    let file_size = 36 + data_size;

    // RIFF header
    file.write_all(b"RIFF")?;
    file.write_all(&file_size.to_le_bytes())?;
    file.write_all(b"WAVE")?;

    // fmt subchunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?; // Subchunk1Size (16 for PCM)
    file.write_all(&1u16.to_le_bytes())?; // AudioFormat (1 = PCM)
    file.write_all(&channels.to_le_bytes())?;
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&block_align.to_le_bytes())?;
    file.write_all(&bits_per_sample.to_le_bytes())?;

    // data subchunk
    file.write_all(b"data")?;
    file.write_all(&data_size.to_le_bytes())?;

    // Write PCM data
    for &sample in samples {
        file.write_all(&sample.to_le_bytes())?;
    }

    Ok(())
}
