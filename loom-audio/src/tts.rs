//! Text-to-Speech (TTS) capability provider under the audio module
//!
//! Provides a native Action capability `tts.speak` that synthesizes speech using
//! local CLI engines with graceful degradation:
//! - Prefer Piper (higher quality, requires voice model)
//! - Fallback to espeak-ng (widely available)
//! - If neither present, logs the text and returns OK
//!
//! Headers supported on ActionCall:
//! - voice: string (piper voice model path or name; espeak voice code)
//! - rate:  float (0.5–2.0, default 1.0)
//! - volume: float (0.5–2.0, default 1.0)
//! - sample_rate: u32 (output WAV sample rate, default 16000)
//! - player: string (aplay|paplay|ffplay), optional preference
//!
//! Env overrides:
//! - PIPER_BIN, PIPER_VOICE, PIPER_VOICE_DIR
//! - ESPEAK_BIN
//! - TTS_TIMEOUT_MS, TTS_TEMP_DIR, TTS_TOPIC
//!
//! Emits observability events on `tts` topic by default:
//! - tts.start, tts.done, tts.error

use crate::utils::{gen_id, now_ms};
use async_trait::async_trait;
use loom_core::messaging::EventBus;
use loom_core::proto::Event;
use loom_core::tools::{Tool, ToolResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use tokio::task;
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn};

#[derive(Clone, Debug)]
pub struct TtsSpeakProviderConfig {
    pub temp_dir: PathBuf,
    pub topic: String,
    pub timeout_ms: u64,
    pub default_sample_rate: u32,
    pub piper_bin: Option<PathBuf>,
    pub piper_voice: Option<PathBuf>,
    pub piper_voice_dir: Option<PathBuf>,
    pub espeak_bin: Option<PathBuf>,
}

impl Default for TtsSpeakProviderConfig {
    fn default() -> Self {
        let temp_dir = std::env::var("TTS_TEMP_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir());
        let topic = std::env::var("TTS_TOPIC").unwrap_or_else(|_| "tts".to_string());
        let timeout_ms = std::env::var("TTS_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(20_000);
        let default_sample_rate = 16_000u32;

        let piper_bin = get_from_env_or_path("PIPER_BIN", "piper");
        let piper_voice = std::env::var("PIPER_VOICE").ok().map(PathBuf::from);
        let piper_voice_dir = std::env::var("PIPER_VOICE_DIR").ok().map(PathBuf::from);
        let espeak_bin =
            get_from_env_or_path("ESPEAK_BIN", "espeak-ng").or_else(|| get_from_path("espeak"));

        Self {
            temp_dir,
            topic,
            timeout_ms,
            default_sample_rate,
            piper_bin,
            piper_voice,
            piper_voice_dir,
            espeak_bin,
        }
    }
}

fn get_from_env_or_path(env_key: &str, default_bin: &str) -> Option<PathBuf> {
    if let Ok(p) = std::env::var(env_key) {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }
    get_from_path(default_bin)
}

fn get_from_path(bin: &str) -> Option<PathBuf> {
    // If a path-like string is provided, respect it directly
    if bin.contains(std::path::MAIN_SEPARATOR) {
        let p = PathBuf::from(bin);
        return if p.exists() { Some(p) } else { None };
    }

    // Search PATH portably
    if let Some(paths_os) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths_os) {
            let candidate = dir.join(bin);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}

pub struct TtsSpeakProvider {
    bus: Arc<EventBus>,
    cfg: TtsSpeakProviderConfig,
}

impl TtsSpeakProvider {
    pub fn new(bus: Arc<EventBus>, cfg: Option<TtsSpeakProviderConfig>) -> Self {
        let cfg = cfg.unwrap_or_default();
        // Log detected engines once
        if let Some(ref p) = cfg.piper_bin {
            info!(target = "tts", bin = ?p, "Detected Piper binary");
        }
        if let Some(ref e) = cfg.espeak_bin {
            info!(target = "tts", bin = ?e, "Detected espeak-ng binary");
        }
        Self { bus, cfg }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct SpeakPayload {
    #[serde(default)]
    text: String,
    #[serde(default)]
    voice: String,
    #[serde(default)]
    rate: Option<f32>,
    #[serde(default)]
    volume: Option<f32>,
    #[serde(default)]
    sample_rate: Option<u32>,
    #[serde(default)]
    player: Option<String>,
}

#[async_trait]
impl Tool for TtsSpeakProvider {
    fn name(&self) -> String {
        "tts.speak".to_string()
    }

    fn description(&self) -> String {
        "Synthesizes speech from text using local TTS engines (Piper or espeak-ng)".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "text": { "type": "string", "description": "Text to synthesize" },
                "voice": { "type": "string", "description": "Voice model or name" },
                "rate": { "type": "number", "description": "Speech rate (0.5-2.0)" },
                "volume": { "type": "number", "description": "Volume (0.5-2.0)" },
                "sample_rate": { "type": "integer", "description": "Output sample rate" },
                "player": { "type": "string", "description": "Audio player preference" }
            },
            "required": ["text"]
        })
    }

    async fn call(&self, arguments: serde_json::Value) -> ToolResult<serde_json::Value> {
        let payload: SpeakPayload = serde_json::from_value(arguments)
            .map_err(|e| loom_core::ToolError::InvalidArguments(e.to_string()))?;

        let text = payload.text;
        if text.trim().is_empty() {
            return Ok(serde_json::json!({ "status": "ok", "message": "empty text" }));
        }

        let voice = payload.voice;
        let rate = payload.rate.unwrap_or(1.0).clamp(0.5, 2.0);
        let volume = payload.volume.unwrap_or(1.0).clamp(0.5, 2.0);
        let sample_rate = payload.sample_rate.unwrap_or(self.cfg.default_sample_rate);
        let player_pref = payload.player;

        let engine = select_engine(&self.cfg, &voice);
        let player = select_player(player_pref.as_deref());

        // start event
        let mut meta = HashMap::new();
        meta.insert("engine".to_string(), engine.clone());
        meta.insert("voice".to_string(), voice.clone());
        meta.insert("rate".to_string(), rate.to_string());
        meta.insert("volume".to_string(), volume.to_string());
        meta.insert("sample_rate".to_string(), sample_rate.to_string());
        meta.insert("player".to_string(), player.clone().unwrap_or_default());

        let start_event = Event {
            id: gen_id(),
            r#type: "tts.start".to_string(),
            timestamp_ms: now_ms(),
            source: "tts".to_string(),
            metadata: meta.clone(),
            payload: text.as_bytes().to_vec(),
            confidence: 1.0,
            tags: vec![],
            priority: 50,
        };
        let _ = self.bus.publish(&self.cfg.topic, start_event).await;

        // If no engine detected, degrade gracefully
        if engine == "none" {
            warn!(target = "tts", "No TTS engine detected. Printing only.");
            let mut meta_done = meta.clone();
            meta_done.insert("no_engine".into(), "true".into());
            let ev = Event {
                id: gen_id(),
                r#type: "tts.done".to_string(),
                timestamp_ms: now_ms(),
                source: "tts".to_string(),
                metadata: meta_done,
                payload: text.as_bytes().to_vec(),
                confidence: 1.0,
                tags: vec![],
                priority: 50,
            };
            let _ = self.bus.publish(&self.cfg.topic, ev).await;

            return Ok(serde_json::json!({
                "engine": "none",
                "printed": true,
                "voice": voice,
                "rate": rate,
                "volume": volume,
                "sample_rate": sample_rate,
                "player": player,
            }));
        }

        // Execute synthesis + playback in blocking task
        let cfg = self.cfg.clone();
        let bus = Arc::clone(&self.bus);
        let topic = cfg.topic.clone();
        let t0 = now_ms();
        let meta_for_timeout = meta.clone();

        let join = task::spawn_blocking(move || {
            let wav_path = cfg.temp_dir.join(format!("tts_{}.wav", gen_id()));
            let synthesis_ms: i64;
            let playback_ms: i64;

            let synth_start = now_ms();
            let synth_ok = match engine.as_str() {
                "piper" => synth_with_piper(&cfg, &voice, rate, sample_rate, &text, &wav_path),
                "espeak-ng" => {
                    synth_with_espeak(&cfg, &voice, rate, volume, sample_rate, &text, &wav_path)
                }
                _ => Ok(()),
            };
            synthesis_ms = now_ms() - synth_start;

            if let Err(err) = synth_ok {
                let mut meta_err = meta.clone();
                meta_err.insert("error".to_string(), err.to_string());
                let ev = Event {
                    id: gen_id(),
                    r#type: "tts.error".to_string(),
                    timestamp_ms: now_ms(),
                    source: "tts".to_string(),
                    metadata: meta_err,
                    payload: Vec::new(),
                    confidence: 0.0,
                    tags: vec![],
                    priority: 50,
                };
                let _ = tokio::runtime::Handle::current().block_on(bus.publish(&topic, ev));
                return Err(loom_core::ToolError::ExecutionFailed(err.to_string()));
            }

            // Post-process volume for Piper only
            if engine == "piper" && (volume - 1.0).abs() > f32::EPSILON {
                if let Err(e) = scale_wav_pcm16_inplace(&wav_path, volume) {
                    warn!(target = "tts", error = %e, "Failed to scale volume for WAV");
                }
            }

            // Playback
            let play_start = now_ms();
            if wav_path.exists() {
                if let Some(bin) = player.as_ref().and_then(|name| get_from_path(name)) {
                    let _ = play_wav_with(&bin, &wav_path);
                } else {
                    if let Some(bin) = get_from_path("aplay")
                        .or_else(|| get_from_path("paplay"))
                        .or_else(|| get_from_path("ffplay"))
                    {
                        let _ = play_wav_with(&bin, &wav_path);
                    } else {
                        info!(target = "tts", path = ?wav_path, "No audio player found; kept WAV on disk");
                    }
                }
            }
            playback_ms = now_ms() - play_start;

            let duration_ms = now_ms() - t0;
            let mut meta_done = meta.clone();
            meta_done.insert("synthesis_ms".into(), synthesis_ms.to_string());
            meta_done.insert("playback_ms".into(), playback_ms.to_string());
            meta_done.insert("total_ms".into(), duration_ms.to_string());
            meta_done.insert("wav_path".into(), wav_path.to_string_lossy().to_string());

            let ev = Event {
                id: gen_id(),
                r#type: "tts.done".to_string(),
                timestamp_ms: now_ms(),
                source: "tts".to_string(),
                metadata: meta_done,
                payload: Vec::new(),
                confidence: 1.0,
                tags: vec![],
                priority: 50,
            };
            let _ = tokio::runtime::Handle::current().block_on(bus.publish(&topic, ev));

            Ok(serde_json::json!({
                "engine": engine,
                "voice": voice,
                "rate": rate,
                "volume": volume,
                "sample_rate": sample_rate,
                "player": player,
                "wav_path": wav_path.to_string_lossy(),
            }))
        });

        // Apply internal timeout
        match timeout(Duration::from_millis(self.cfg.timeout_ms), join).await {
            Ok(join_res) => join_res.map_err(|e| loom_core::ToolError::Internal(e.to_string()))?,
            Err(_) => {
                let ev = Event {
                    id: gen_id(),
                    r#type: "tts.error".to_string(),
                    timestamp_ms: now_ms(),
                    source: "tts".to_string(),
                    metadata: {
                        let mut m = meta_for_timeout;
                        m.insert("timeout_ms".into(), self.cfg.timeout_ms.to_string());
                        m
                    },
                    payload: Vec::new(),
                    confidence: 0.0,
                    tags: vec![],
                    priority: 50,
                };
                let _ = self.bus.publish(&self.cfg.topic, ev).await;
                Err(loom_core::ToolError::Timeout)
            }
        }
    }
}

fn select_engine(cfg: &TtsSpeakProviderConfig, _voice_header: &str) -> String {
    if cfg.piper_bin.is_some() {
        return "piper".into();
    }
    if cfg.espeak_bin.is_some() {
        return "espeak-ng".into();
    }
    "none".into()
}

fn resolve_piper_voice_path(cfg: &TtsSpeakProviderConfig, voice_header: &str) -> Option<PathBuf> {
    if let Some(v) = &cfg.piper_voice {
        return Some(v.clone());
    }
    if voice_header.is_empty() {
        return None;
    }
    let vh = PathBuf::from(voice_header);
    if vh.exists() {
        return Some(vh);
    }
    if let Some(dir) = &cfg.piper_voice_dir {
        let candidate = dir.join(voice_header);
        if candidate.exists() {
            return Some(candidate);
        }
        for ext in ["onnx", "onnx.gz", "pt", "pth"].iter() {
            let c = dir.join(format!("{}.{}", voice_header, ext));
            if c.exists() {
                return Some(c);
            }
        }
    }
    None
}

fn synth_with_piper(
    cfg: &TtsSpeakProviderConfig,
    voice: &str,
    rate: f32,
    sample_rate: u32,
    text: &str,
    out_wav: &Path,
) -> loom_core::Result<()> {
    let piper = cfg
        .piper_bin
        .as_ref()
        .ok_or_else(|| loom_core::LoomError::AgentError("Piper binary not found".into()))?;
    let voice_path = resolve_piper_voice_path(cfg, voice).ok_or_else(|| {
        loom_core::LoomError::AgentError(
            "Piper voice not found; set PIPER_VOICE or headers.voice".into(),
        )
    })?;

    let mut cmd = Command::new(piper);
    cmd.arg("-m").arg(voice_path);
    cmd.arg("-f").arg(out_wav);
    let length_scale = (1.0f32 / rate).clamp(0.5, 2.0);
    cmd.arg("--length_scale")
        .arg(format!("{:.2}", length_scale));
    cmd.arg("--sample_rate").arg(sample_rate.to_string());
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    debug!(target = "tts", command = ?cmd, "Running piper");
    let mut child = cmd.spawn().map_err(loom_core::LoomError::IoError)?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(loom_core::LoomError::IoError)?;
    }
    let output = child
        .wait_with_output()
        .map_err(loom_core::LoomError::IoError)?;
    if !output.status.success() {
        return Err(loom_core::LoomError::AgentError(format!(
            "Piper failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
}

fn synth_with_espeak(
    cfg: &TtsSpeakProviderConfig,
    voice: &str,
    rate: f32,
    volume: f32,
    _sample_rate: u32,
    text: &str,
    out_wav: &Path,
) -> loom_core::Result<()> {
    let espeak = cfg
        .espeak_bin
        .as_ref()
        .ok_or_else(|| loom_core::LoomError::AgentError("espeak-ng not found".into()))?;
    let mut cmd = Command::new(espeak);
    let wpm = (160.0 * rate).round().clamp(80.0, 450.0) as i32;
    let amp = (100.0 * volume).round().clamp(50.0, 200.0) as i32;
    if !voice.is_empty() {
        cmd.arg("-v").arg(voice);
    }
    cmd.arg("-s").arg(wpm.to_string());
    cmd.arg("-a").arg(amp.to_string());
    cmd.arg("-w").arg(out_wav);
    cmd.arg(text);
    debug!(target = "tts", command = ?cmd, "Running espeak-ng");
    let output = Command::new(cmd.get_program())
        .args(cmd.get_args())
        .output()
        .map_err(loom_core::LoomError::IoError)?;
    if !output.status.success() {
        return Err(loom_core::LoomError::AgentError(format!(
            "espeak-ng failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
}

fn play_wav_with(player_bin: &Path, wav_path: &Path) -> std::io::Result<()> {
    let name = player_bin
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    match name {
        "aplay" => {
            Command::new(player_bin).arg(wav_path).status()?;
        }
        "paplay" => {
            Command::new(player_bin).arg(wav_path).status()?;
        }
        "ffplay" => {
            Command::new(player_bin)
                .arg("-autoexit")
                .arg("-nodisp")
                .arg(wav_path)
                .status()?;
        }
        _ => {
            Command::new(player_bin).arg(wav_path).status()?;
        }
    }
    Ok(())
}

fn scale_wav_pcm16_inplace(path: &Path, gain: f32) -> std::io::Result<()> {
    let mut f = File::open(path)?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;

    if &buf[0..4] != b"RIFF" || &buf[8..12] != b"WAVE" {
        return Ok(());
    }
    let mut idx = 12;
    let mut data_start = None;
    let mut data_len = 0usize;
    while idx + 8 <= buf.len() {
        let chunk_id = &buf[idx..idx + 4];
        let sz =
            u32::from_le_bytes([buf[idx + 4], buf[idx + 5], buf[idx + 6], buf[idx + 7]]) as usize;
        if chunk_id == b"data" {
            data_start = Some(idx + 8);
            data_len = sz;
            break;
        }
        idx += 8 + sz;
    }
    if let Some(start) = data_start {
        let end = start + data_len;
        let data = &mut buf[start..end];
        for chunk in data.chunks_exact_mut(2) {
            let s = i16::from_le_bytes([chunk[0], chunk[1]]);
            let scaled = (s as f32 * gain).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            let bytes = scaled.to_le_bytes();
            chunk[0] = bytes[0];
            chunk[1] = bytes[1];
        }
        let mut out = File::create(path)?;
        out.write_all(&buf)?;
    }
    Ok(())
}

fn select_player(pref: Option<&str>) -> Option<String> {
    if let Some(p) = pref {
        if get_from_path(p).is_some() {
            return Some(p.to_string());
        }
    }
    if get_from_path("aplay").is_some() {
        return Some("aplay".into());
    }
    if get_from_path("paplay").is_some() {
        return Some("paplay".into());
    }
    if get_from_path("ffplay").is_some() {
        return Some("ffplay".into());
    }
    None
}
