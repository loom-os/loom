//! Text-to-Speech (TTS) capability provider
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

mod provider;

pub use provider::{TtsSpeakProvider, TtsSpeakProviderConfig};
