// Audio-related event sources and utilities

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

// Shared audio utilities
pub mod utils;
