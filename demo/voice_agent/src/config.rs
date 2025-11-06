use std::collections::HashMap;
use std::path::PathBuf;

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
