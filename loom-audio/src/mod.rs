// Audio-related event sources and utilities

// Shared audio utilities
pub(crate) mod utils;

#[cfg(feature = "mic")]
pub mod mic;

#[cfg(feature = "mic")]
pub use mic::{MicConfig, MicSource};

#[cfg(feature = "vad")]
pub mod vad;

#[cfg(feature = "vad")]
pub use vad::{VadConfig, VadGate};

#[cfg(feature = "stt")]
pub mod stt;

#[cfg(feature = "stt")]
pub use stt::{SttConfig, SttEngine};

#[cfg(feature = "wake")]
pub mod wake;

#[cfg(feature = "wake")]
pub use wake::{WakeWordConfig, WakeWordDetector};

#[cfg(feature = "tts")]
pub mod tts;

#[cfg(feature = "tts")]
pub use tts::{TtsSpeakProvider, TtsSpeakProviderConfig};
// (utils re-export intentionally crate-visible only)
