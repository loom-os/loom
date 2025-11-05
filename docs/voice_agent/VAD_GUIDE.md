# Voice Activity Detection (VAD) Guide

This guide explains how to use the Voice Activity Detection (VAD) module in Loom to detect speech segments in audio streams.

## Overview

The VAD module consumes raw `audio_chunk` events and produces:

- `vad.speech_start`: Emitted when speech activity begins
- `vad.speech_end`: Emitted when speech activity ends
- `audio_voiced`: Voiced audio frames during speech (for downstream STT processing)

## Quick Start

```rust
use loom_core::audio::{MicConfig, MicSource, VadConfig, VadGate};
use loom_core::{EventBus, QoSLevel};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let event_bus = Arc::new(EventBus::new().await?);
    event_bus.start().await?;

    // Start microphone
    let mic_config = MicConfig::default();
    let mic_source = MicSource::new(Arc::clone(&event_bus), mic_config);
    mic_source.start().await?;

    // Start VAD
    let vad_config = VadConfig::default();
    let vad_gate = VadGate::new(Arc::clone(&event_bus), vad_config);
    vad_gate.start().await?;

    // Subscribe to VAD events
    let (_sub_id, mut rx) = event_bus
        .subscribe(
            "vad".to_string(),
            vec!["vad.speech_start".to_string(), "vad.speech_end".to_string()],
            QoSLevel::QosRealtime,
        )
        .await?;

    while let Some(event) = rx.recv().await {
        println!("VAD event: {:?}", event.r#type);
    }

    Ok(())
}
```

## Configuration

VAD behavior can be customized via environment variables:

### VAD Mode (Aggressiveness)

Controls how aggressive the VAD is in filtering out non-speech:

```bash
VAD_MODE=0  # Quality (most permissive, detects more speech)
VAD_MODE=1  # LowBitrate
VAD_MODE=2  # Aggressive (default, balanced)
VAD_MODE=3  # VeryAggressive (most strict, fewer false positives)
```

**Recommendation**: Start with `VAD_MODE=2` and adjust:

- Increase to 3 for noisy environments
- Decrease to 1 or 0 if missing speech in quiet environments

### Frame Size

The VAD processes audio in fixed-size frames:

```bash
VAD_FRAME_MS=10   # 10ms frames (more granular)
VAD_FRAME_MS=20   # 20ms frames (default, balanced)
VAD_FRAME_MS=30   # 30ms frames (less granular)
```

**Note**: Only 10, 20, and 30 ms are supported by the underlying webrtc-vad library.

### Speech Start Detection

Minimum consecutive voiced duration to trigger `speech_start`:

```bash
VAD_MIN_START_MS=60    # Default: 60ms
VAD_MIN_START_MS=100   # More conservative (reduces false starts)
VAD_MIN_START_MS=30    # More responsive (faster detection)
```

### Speech End Detection (Hangover)

Duration to wait after last voiced frame before triggering `speech_end`:

```bash
VAD_HANGOVER_MS=200    # Default: 200ms
VAD_HANGOVER_MS=300    # Longer hangover (bridges short pauses)
VAD_HANGOVER_MS=100    # Shorter hangover (quicker cutoff)
```

**Use case**: Increase hangover to avoid splitting sentences with brief pauses.

### Topic Configuration

Customize input/output event topics:

```bash
VAD_INPUT_TOPIC=audio.mic        # Default: where to read audio_chunk events
VAD_VOICED_TOPIC=audio.voiced    # Default: where to publish audio_voiced
VAD_TOPIC=vad                    # Default: where to publish speech_start/end
```

## Event Schema

### Input: `audio_chunk`

Expected metadata:

- `sample_rate`: Sample rate in Hz (8000, 16000, 32000, or 48000)
- `channels`: Number of audio channels (mono or stereo)

Payload: PCM16 little-endian audio samples

### Output: `vad.speech_start`

Metadata:

- `frame_ms`: Frame size used
- `mode`: VAD aggressiveness mode
- `sample_rate`: Audio sample rate

Payload: empty

### Output: `vad.speech_end`

Same metadata as `speech_start`, payload empty.

### Output: `audio_voiced`

Metadata:

- `sample_rate`: Audio sample rate (Hz)
- `channels`: Always "1" (mono)
- `encoding`: "pcm_s16le"
- `frame_ms`: Frame duration

Payload: PCM16 mono audio frame (voiced speech)

## Example: Running the VAD Demo

```bash
# Install dependencies (Linux)
sudo apt-get install -y libasound2-dev pkg-config

# Run with defaults
cargo run --example mic_vad --features mic,vad

# Run with custom settings
VAD_MODE=3 VAD_MIN_START_MS=100 VAD_HANGOVER_MS=300 \
  cargo run --example mic_vad --features mic,vad
```

## Integration with STT Pipeline

The typical flow for speech-to-text is:

1. **Mic** → `audio_chunk` events
2. **VAD** → `vad.speech_start`, `audio_voiced`, `vad.speech_end`
3. **Utterance Segmenter** → Buffer `audio_voiced` between start/end
4. **STT** → Convert buffered audio to text → `transcript.final`

See the full E2E voice pipeline example in `examples/e2e_voice_wake_llm_tts.rs` (coming soon).

## Troubleshooting

### Too Many False Starts

- Increase `VAD_MODE` (e.g., from 2 to 3)
- Increase `VAD_MIN_START_MS`

### Missing Speech Detection

- Decrease `VAD_MODE` (e.g., from 2 to 1)
- Decrease `VAD_MIN_START_MS`

### Speech Cut Off Too Soon

- Increase `VAD_HANGOVER_MS`

### Speech Segments Split Unexpectedly

- Increase `VAD_HANGOVER_MS` to bridge short pauses
- Consider post-processing to merge segments within a time window

## Performance Notes

- **CPU**: webrtc-vad is highly optimized; typical CPU usage < 1% on modern hardware
- **Latency**: Frame-level processing adds minimal latency (< frame_ms + 10ms)
- **Memory**: Negligible; only one frame at a time is held in memory

## Technical Details

- **Library**: Uses `webrtc-vad` crate (Rust wrapper for WebRTC VAD)
- **Algorithm**: GMM-based voice activity detection
- **Sample Rates**: 8kHz, 16kHz, 32kHz, 48kHz
- **Thread Safety**: VAD processing is non-Send but isolated per-event to avoid blocking

## Next Steps

- [Microphone Capture Guide](./MIC_GUIDE.md) - Configuring audio input
- [STT Integration](./STT_GUIDE.md) - Speech-to-text pipeline (coming soon)
- [E2E Voice Pipeline](./VOICE_PIPELINE.md) - Full voice assistant flow (coming soon)
