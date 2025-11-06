use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use loom_audio::{MicConfig, SttConfig, VadConfig, WakeWordConfig};

/// High-level configuration for the Voice Agent demo
#[derive(Clone, Debug)]
pub struct VoiceAgentConfig {
    pub mic: MicConfig,
    pub vad: VadConfig,
    pub stt: SttConfig,
    pub wake: WakeWordConfig,
    pub llm: LlmConfig,
    pub tts: TtsConfig,
    /// Topic where user queries are published by Wake module
    pub query_topic: String,
}

/// LLM client configuration (used to fill headers for llm.generate)
#[derive(Clone, Debug)]
pub struct LlmConfig {
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
    pub request_timeout_ms: u64,
    pub temperature: f32,
    pub system_prompt: String,
}

/// Local TTS preferences (mapped to tts.speak headers)
#[derive(Clone, Debug, Default)]
pub struct TtsConfig {
    pub voice: Option<String>,
    pub rate: Option<f32>,
    pub volume: Option<f32>,
    pub sample_rate: Option<u32>,
    pub player: Option<String>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
			base_url: std::env::var("VLLM_BASE_URL")
				.ok()
				.filter(|s| !s.is_empty())
				.unwrap_or_else(|| "http://localhost:8000/v1".to_string()),
			model: std::env::var("VLLM_MODEL")
				.ok()
				.filter(|s| !s.is_empty())
				.unwrap_or_else(|| "qwen2.5-0.5b-instruct".to_string()),
			api_key: std::env::var("VLLM_API_KEY").ok().filter(|s| !s.is_empty()),
			request_timeout_ms: std::env::var("REQUEST_TIMEOUT_MS")
				.ok()
				.and_then(|v| v.parse::<u64>().ok())
				.unwrap_or(30_000),
			temperature: std::env::var("VLLM_TEMPERATURE")
				.ok()
				.and_then(|v| v.parse::<f32>().ok())
				.unwrap_or(0.7),
			system_prompt: std::env::var("VOICE_SYSTEM_PROMPT").unwrap_or_else(|_| {
				"You are Loom's helpful and concise voice assistant. Answer briefly and clearly.".into()
			}),
		}
    }
}

impl Default for VoiceAgentConfig {
    fn default() -> Self {
        // Start from feature module defaults (which already consider env vars)
        let mut stt = SttConfig::default();
        // Provide gentle defaults for STT binary/model if unset
        if stt.whisper_bin.as_os_str().is_empty() {
            stt.whisper_bin = PathBuf::from("whisper");
        }
        if stt.whisper_model.as_os_str().is_empty() {
            stt.whisper_model = PathBuf::from("ggml-base.en.bin");
        }

        Self {
            mic: MicConfig::default(),
            vad: VadConfig::default(),
            stt,
            wake: WakeWordConfig::default(),
            llm: LlmConfig::default(),
            tts: TtsConfig::default(),
            query_topic: std::env::var("QUERY_TOPIC").unwrap_or_else(|_| "query".to_string()),
        }
    }
}

impl VoiceAgentConfig {
    /// Load configuration from a TOML file (path via VOICE_AGENT_CONFIG or ./voice_agent.toml),
    /// overlaying values onto sane defaults and env-driven defaults.
    pub fn load() -> Self {
        let default = Self::default();
        let path =
            std::env::var("VOICE_AGENT_CONFIG").unwrap_or_else(|_| "voice_agent.toml".into());
        let p = Path::new(&path);
        if !p.exists() {
            tracing::info!(target = "voice_agent", path = %path, "No TOML config found; using defaults/env");
            return default;
        }
        match fs::read_to_string(p) {
            Ok(s) => match toml::from_str::<VoiceAgentToml>(&s) {
                Ok(t) => t.overlay(default),
                Err(e) => {
                    tracing::warn!(target = "voice_agent", error = %e, "Failed to parse TOML; using defaults");
                    default
                }
            },
            Err(e) => {
                tracing::warn!(target = "voice_agent", error = %e, "Failed to read TOML; using defaults");
                default
            }
        }
    }
    /// Build LLM headers for the `llm.generate` ActionCall based on config
    pub fn llm_headers(&self) -> HashMap<String, String> {
        let mut h = HashMap::new();
        if !self.llm.model.is_empty() {
            h.insert("model".into(), self.llm.model.clone());
        }
        if !self.llm.base_url.is_empty() {
            h.insert("base_url".into(), self.llm.base_url.clone());
        }
        h.insert(
            "request_timeout_ms".into(),
            self.llm.request_timeout_ms.to_string(),
        );
        h.insert("temperature".into(), format!("{}", self.llm.temperature));
        if let Some(key) = &self.llm.api_key {
            if !key.is_empty() {
                // Some backends accept Authorization separately; for simplicity we keep it here
                h.insert("authorization".into(), format!("Bearer {}", key));
            }
        }
        h
    }

    /// Build TTS headers for the `tts.speak` ActionCall based on config
    pub fn tts_headers(&self) -> HashMap<String, String> {
        let mut h = HashMap::new();
        if let Some(v) = &self.tts.voice {
            h.insert("voice".into(), v.clone());
        }
        if let Some(v) = self.tts.rate {
            h.insert("rate".into(), format!("{}", v));
        }
        if let Some(v) = self.tts.volume {
            h.insert("volume".into(), format!("{}", v));
        }
        if let Some(v) = self.tts.sample_rate {
            h.insert("sample_rate".into(), v.to_string());
        }
        if let Some(v) = &self.tts.player {
            h.insert("player".into(), v.clone());
        }
        h
    }
}

// =========================
// TOML overlay definitions
// =========================

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct VoiceAgentToml {
    pub query_topic: Option<String>,
    pub mic: Option<MicToml>,
    pub vad: Option<VadToml>,
    pub stt: Option<SttToml>,
    pub wake: Option<WakeToml>,
    pub llm: Option<LlmToml>,
    pub tts: Option<TtsToml>,
}

impl VoiceAgentToml {
    fn overlay(self, mut base: VoiceAgentConfig) -> VoiceAgentConfig {
        if let Some(q) = self.query_topic {
            base.query_topic = q;
        }
        if let Some(m) = self.mic {
            m.apply(&mut base.mic);
        }
        if let Some(v) = self.vad {
            v.apply(&mut base.vad);
        }
        if let Some(s) = self.stt {
            s.apply(&mut base.stt);
        }
        if let Some(w) = self.wake {
            w.apply(&mut base.wake);
        }
        if let Some(l) = self.llm {
            l.apply(&mut base.llm);
        }
        if let Some(t) = self.tts {
            t.apply(&mut base.tts);
        }
        base
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct MicToml {
    pub sample_rate_hz: Option<u32>,
    pub channels: Option<u16>,
    pub chunk_ms: Option<u32>,
    pub device_name: Option<String>,
    pub topic: Option<String>,
    pub source: Option<String>,
}
impl MicToml {
    fn apply(self, m: &mut MicConfig) {
        if let Some(v) = self.sample_rate_hz {
            m.sample_rate_hz = v;
        }
        if let Some(v) = self.channels {
            m.channels = v;
        }
        if let Some(v) = self.chunk_ms {
            m.chunk_ms = v;
        }
        if let Some(v) = self.device_name {
            m.device_name = Some(v);
        }
        if let Some(v) = self.topic {
            m.topic = v;
        }
        if let Some(v) = self.source {
            m.source = v;
        }
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct VadToml {
    pub input_topic: Option<String>,
    pub voiced_topic: Option<String>,
    pub vad_topic: Option<String>,
    pub mode: Option<i32>,
    pub frame_ms: Option<u32>,
    pub min_start_ms: Option<u32>,
    pub hangover_ms: Option<u32>,
}
impl VadToml {
    fn apply(self, v: &mut VadConfig) {
        if let Some(x) = self.input_topic {
            v.input_topic = x;
        }
        if let Some(x) = self.voiced_topic {
            v.voiced_topic = x;
        }
        if let Some(x) = self.vad_topic {
            v.vad_topic = x;
        }
        if let Some(x) = self.mode {
            v.mode = x.clamp(0, 3);
        }
        if let Some(x) = self.frame_ms {
            v.frame_ms = x;
        }
        if let Some(x) = self.min_start_ms {
            v.min_start_ms = x;
        }
        if let Some(x) = self.hangover_ms {
            v.hangover_ms = x;
        }
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct SttToml {
    pub vad_topic: Option<String>,
    pub voiced_topic: Option<String>,
    pub transcript_topic: Option<String>,
    pub whisper_bin: Option<PathBuf>,
    pub whisper_model: Option<PathBuf>,
    pub language: Option<String>,
    pub temp_dir: Option<PathBuf>,
    pub extra_args: Option<Vec<String>>, // e.g., ["--threads", "4"]
}
impl SttToml {
    fn apply(self, s: &mut SttConfig) {
        if let Some(x) = self.vad_topic {
            s.vad_topic = x;
        }
        if let Some(x) = self.voiced_topic {
            s.voiced_topic = x;
        }
        if let Some(x) = self.transcript_topic {
            s.transcript_topic = x;
        }
        if let Some(x) = self.whisper_bin {
            s.whisper_bin = x;
        }
        if let Some(x) = self.whisper_model {
            s.whisper_model = x;
        }
        if let Some(x) = self.language {
            s.language = x;
        }
        if let Some(x) = self.temp_dir {
            s.temp_dir = x;
        }
        if let Some(mut x) = self.extra_args {
            s.extra_args = x.drain(..).filter(|a| !a.is_empty()).collect();
        }
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct WakeToml {
    pub transcript_topic: Option<String>,
    pub wake_topic: Option<String>,
    pub query_topic: Option<String>,
    pub phrases: Option<Vec<String>>, // comma-like via TOML arrays
    pub max_distance: Option<usize>,
    pub match_anywhere: Option<bool>,
    pub jaro_winkler_threshold: Option<f64>,
    pub min_query_chars: Option<usize>,
}
impl WakeToml {
    fn apply(self, w: &mut WakeWordConfig) {
        if let Some(x) = self.transcript_topic {
            w.transcript_topic = x;
        }
        if let Some(x) = self.wake_topic {
            w.wake_topic = x;
        }
        if let Some(x) = self.query_topic { /* prefer top-level query_topic, but keep backward-compat */
        }
        if let Some(x) = self.phrases {
            w.phrases = x.into_iter().map(|s| s.to_lowercase()).collect();
        }
        if let Some(x) = self.max_distance {
            w.max_distance = x;
        }
        if let Some(x) = self.match_anywhere {
            w.match_anywhere = x;
        }
        if let Some(x) = self.jaro_winkler_threshold {
            w.jaro_winkler_threshold = x;
        }
        if let Some(x) = self.min_query_chars {
            w.min_query_chars = x;
        }
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct LlmToml {
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub request_timeout_ms: Option<u64>,
    pub temperature: Option<f32>,
    pub system_prompt: Option<String>,
}
impl LlmToml {
    fn apply(self, l: &mut LlmConfig) {
        if let Some(x) = self.base_url {
            l.base_url = x;
        }
        if let Some(x) = self.model {
            l.model = x;
        }
        if let Some(x) = self.api_key {
            l.api_key = Some(x);
        }
        if let Some(x) = self.request_timeout_ms {
            l.request_timeout_ms = x;
        }
        if let Some(x) = self.temperature {
            l.temperature = x;
        }
        if let Some(x) = self.system_prompt {
            l.system_prompt = x;
        }
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct TtsToml {
    pub voice: Option<String>,
    pub rate: Option<f32>,
    pub volume: Option<f32>,
    pub sample_rate: Option<u32>,
    pub player: Option<String>,
}
impl TtsToml {
    fn apply(self, t: &mut TtsConfig) {
        if let Some(x) = self.voice {
            t.voice = Some(x);
        }
        if let Some(x) = self.rate {
            t.rate = Some(x);
        }
        if let Some(x) = self.volume {
            t.volume = Some(x);
        }
        if let Some(x) = self.sample_rate {
            t.sample_rate = Some(x);
        }
        if let Some(x) = self.player {
            t.player = Some(x);
        }
    }
}
