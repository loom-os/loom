# Loom Audio Module

Event-driven audio processing pipeline for voice applications.

## Components

### 1. Microphone Capture (`mic.rs`)

Captures audio from system microphone and publishes `audio_chunk` events.

**Features**:

- Cross-platform via `cpal` (ALSA/PulseAudio on Linux, CoreAudio on macOS, WASAPI on Windows)
- Configurable sample rate (16kHz default) and chunk size (20ms default)
- Device selection via `MIC_DEVICE` environment variable
- PCM16 mono output

**Event Output**: `audio_chunk` on topic `audio.mic`

See [Microphone Guide](../../docs/MIC_GUIDE.md) for details.

### 2. Voice Activity Detection (`vad.rs`)

Detects speech segments in audio streams using WebRTC VAD.

**Features**:

- Real-time speech/non-speech classification
- Configurable aggressiveness (0-3)
- Smart speech boundary detection with hangover
- Outputs both boundary events and voiced frames

**Event Input**: `audio_chunk` from topic `audio.mic`
**Event Output**:

- `vad.speech_start`, `vad.speech_end` on topic `vad`
- `audio_voiced` on topic `audio.voiced`

See [VAD Guide](../../docs/VAD_GUIDE.md) for details.

### 3. Speech-to-Text (`stt.rs`)

Transcribes speech segments to text using whisper.cpp.

**Features**:

- Automatic utterance segmentation based on VAD events
- High-quality transcription via whisper.cpp
- Multi-language support
- Graceful degradation when whisper unavailable
- Configurable model selection

**Event Input**:

- `vad.speech_start`, `vad.speech_end` from topic `vad`
- `audio_voiced` from topic `audio.voiced`

**Event Output**: `transcript.final` on topic `transcript`

See [STT Guide](../../docs/STT.md) for details.

### 4. Wake Word on Transcript (`wake.rs`)

Lightweight wake word detection using final transcripts (no extra model).

**Features**:

- Listens to `transcript.final` events and matches phrases like "hey loom" or "loom"
- Fuzzy matching via Levenshtein distance (configurable, default <= 1)
- Publishes `wake_word_detected` with a fresh `session_id`
- If the same utterance has remainder after the wake phrase, publishes `user.query` immediately; otherwise arms and treats the next utterance as the query

**Event Input**: `transcript.final` from topic `transcript`

**Event Output**:

- `wake_word_detected` on topic `wake`
- `user.query` on topic `query`

Enable with feature flag `wake`.

Configuration:

- `WAKE_PHRASES`: Comma-separated list of phrases (default: `"hey loom,loom"`)
- `WAKE_FUZZY_DISTANCE`: Max edit distance (default: `1`)
- `WAKE_MIN_QUERY_CHARS`: Min chars to consider same-utterance query (default: `4`)
- `WAKE_MATCH_ANYWHERE`: Allow matching phrases anywhere in the sentence (default: `true`)
- `WAKE_JW_THRESHOLD`: Jaro–Winkler similarity threshold 0.0–1.0 (default: `0.90`) — higher is stricter
- `WAKE_TOPIC`: Output topic for wake events (default: `"wake"`)
- `QUERY_TOPIC`: Output topic for queries (default: `"query"`)

## Quick Start

### Prerequisites

**Linux** (Debian/Ubuntu):

```bash
sudo apt-get install -y libasound2-dev pkg-config
```

**macOS**: No additional dependencies

**Windows**: Ensure Windows SDK is installed

### Enable Features

Add to `Cargo.toml`:

```toml
loom-core = { version = "0.1", features = ["mic", "vad", "stt"] }
```

### Basic Usage

```rust
use loom_core::audio::{MicConfig, MicSource, VadConfig, VadGate};
use loom_core::{EventBus, QoSLevel};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let event_bus = Arc::new(EventBus::new().await?);
    event_bus.start().await?;

    // Start microphone capture
    let mic = MicSource::new(Arc::clone(&event_bus), MicConfig::default());
    mic.start().await?;

    // Start VAD
    let vad = VadGate::new(Arc::clone(&event_bus), VadConfig::default());
    vad.start().await?;

    // Subscribe to speech events
    let (_id, mut rx) = event_bus
        .subscribe("vad", vec!["vad.speech_start", "vad.speech_end"], QoSLevel::QosRealtime)
        .await?;

    while let Some(event) = rx.recv().await {
        println!("{:?} @ {}ms", event.r#type, event.timestamp_ms);
    }

    Ok(())
}
```

## Examples

### Mic Capture Only

```bash
cargo run --example mic_capture --features mic
```

### Mic + VAD

```bash
cargo run --example mic_vad --features mic,vad
```

### Mic + VAD + STT (Full Pipeline)

```bash
# Basic usage (requires whisper.cpp in PATH)
cargo run --example mic_vad_stt --features mic,vad,stt

# With custom whisper location (English-only default)
WHISPER_BIN=./whisper.cpp/build/bin/whisper-cli \
WHISPER_MODEL_PATH=./whisper.cpp/models/ggml-base.en.bin \
WHISPER_LANG=en \
cargo run --example mic_vad_stt --features mic,vad,stt

# Chinese (multilingual model + language)
WHISPER_BIN=./whisper.cpp/build/bin/whisper-cli \
WHISPER_MODEL_PATH=./whisper.cpp/models/ggml-base.bin \
WHISPER_LANG=zh \
cargo run --example mic_vad_stt --features mic,vad,stt
```

### Custom Configuration

```bash
# More aggressive VAD with longer hangover
VAD_MODE=3 VAD_HANGOVER_MS=300 \
  cargo run --example mic_vad --features mic,vad
```

## Event Pipeline

```
┌─────────────┐     ┌─────────────┐     ┌──────────────┐
│  Microphone │────▶│     VAD     │────▶│     STT      │
└─────────────┘     └─────────────┘     └──────────────┘
   audio_chunk      speech_start              ▼
                    audio_voiced         transcript.final
                    speech_end
```

## Architecture

- **Event-Driven**: All components communicate via EventBus
- **QoS-Aware**: Audio uses `QoSRealtime` for low-latency delivery
- **Backpressure**: Automatic event dropping when consumers can't keep up
- **Thread-Safe**: Non-Send audio processing isolated from async runtime

## Configuration

All audio components use environment variables for configuration:

### Microphone

- `MIC_LOG_DEVICES`: If set (e.g., `1`), log all available input devices and their supported sample-rate range and max channels to help selection (use with `MIC_DEVICE`).

Notes:

- The runtime logs the actual device, sample rate, channels, and sample format chosen. Use `MIC_DEVICE` to force a specific device by substring.
- For best STT accuracy, prefer internal/USB mics over Bluetooth HFP/HSP profiles (which are narrowband and low quality).

### VAD

- `VAD_MODE`: Aggressiveness (0–3, default: `2`)
- `VAD_FRAME_MS`: Frame size in milliseconds (one of `10`, `20`, `30`; default: `20`)
- `VAD_MIN_START_MS`: Minimum voiced duration before emitting `vad.speech_start` (default: `60`)
- `VAD_HANGOVER_MS`: Delay after last voiced frame before emitting `vad.speech_end` (default: `200`)
- `VAD_INPUT_TOPIC`: Input audio chunk topic (default: `"audio.mic"`)
- `VAD_VOICED_TOPIC`: Voiced audio topic (default: `"audio.voiced"`)
- `VAD_TOPIC`: VAD event topic (default: `"vad"`)

### STT

- `WHISPER_BIN`: Path to whisper.cpp executable (default: "whisper")
- `WHISPER_MODEL_PATH`: Path to model file (default: "ggml-base.en.bin" - English-only)
- `WHISPER_LANG`: Language code (default: "en"). For Chinese use `ggml-base.bin` + `WHISPER_LANG=zh`.
- `WHISPER_EXTRA_ARGS`: Comma-separated extra args (default: none)
- `STT_VAD_TOPIC`: VAD events topic (default: "vad")
- `STT_VOICED_TOPIC`: Voiced audio topic (default: "audio.voiced")
- `STT_TRANSCRIPT_TOPIC`: Transcript output topic (default: "transcript")
- `STT_TEMP_DIR`: Temp directory for WAV files (default: system temp)

## Roadmap

### P0 (Current)

- [x] Microphone capture with cpal
- [x] Voice Activity Detection with webrtc-vad
- [x] Utterance segmentation (buffer between speech_start/end)
- [x] STT integration (whisper.cpp CLI)

### P1 (Next)

- [ ] In-process STT (whisper-rs or vosk)
- [ ] Audio format conversion utilities
- [ ] Noise suppression (RNNoise)
- [ ] Echo cancellation

### P2 (Future)

- [ ] Wake word detection (Porcupine/OpenWakeWord)
- [ ] Speaker diarization
- [ ] Audio preprocessing (AGC, filtering)
- [ ] Streaming STT with partial results

## Testing

Run tests with audio features:

```bash
# Test all audio features
cargo test --features mic,vad,stt

# Test specific modules
cargo test --features mic,vad --test vad
cargo test --features stt --test stt
```

## Performance

- **Microphone**: ~5% CPU @ 16kHz mono (varies by device)
- **VAD**: <1% CPU overhead
- **Latency**: ~30-50ms end-to-end (mic → VAD → event)
- **Memory**: <1MB per audio stream

## Troubleshooting

### Linux: No audio device found

```bash
# Check ALSA devices
arecord -l

# Test recording
arecord -d 3 -f S16_LE -r 16000 test.wav

# Set device explicitly
MIC_DEVICE="USB" cargo run --example mic_capture --features mic
```

### Audio choppy or distorted

- Reduce `MIC_CHUNK_MS` for lower latency
- Check system audio settings
- Ensure no other apps monopolizing audio

### VAD not detecting speech

- Lower `VAD_MODE` (more permissive)
- Reduce `VAD_MIN_START_MS`
- Check microphone levels

### Too many false VAD triggers

- Increase `VAD_MODE` (more aggressive)
- Increase `VAD_MIN_START_MS`
- Use headset microphone to reduce ambient noise

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.

## License

See [LICENSE](../../LICENSE).
