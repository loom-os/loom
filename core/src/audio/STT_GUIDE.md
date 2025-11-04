# Speech-to-Text (STT) Module

The STT module provides real-time speech recognition capabilities by integrating with whisper.cpp. It consumes VAD (Voice Activity Detection) events to segment audio and transcribe speech into text.

## Features

- **Event-Driven**: Listens to `vad.speech_start` and `vad.speech_end` events
- **Automatic Segmentation**: Buffers audio frames between speech boundaries
- **CLI-based Transcription**: Uses whisper.cpp for high-quality transcription
- **Graceful Degradation**: Continues running even if whisper is not available
- **Configurable**: Supports multiple languages and custom whisper parameters

## Architecture

```
Mic → audio_chunk → VAD → vad.speech_start
                      ↓
                  audio_voiced → [STT Buffer]
                      ↓
                  vad.speech_end → WAV file → whisper.cpp → transcript.final
```

### Event Flow

1. **Input Events**:

   - `vad.speech_start`: Signals the start of speech, initializes audio buffer
   - `audio_voiced`: Voiced audio frames (PCM16) to be buffered
   - `vad.speech_end`: Signals end of speech, triggers transcription

2. **Output Events**:
   - `transcript.final`: Contains transcribed text and metadata

## Prerequisites

### 1. Install whisper.cpp

```bash
# Clone and build whisper.cpp
git clone https://github.com/ggerganov/whisper.cpp
cd whisper.cpp
make

# Download a model (e.g., base.en for English)
bash ./models/download-ggml-model.sh base.en

# Or download other models:
# base: multilingual, ~140MB
# small: multilingual, ~460MB
# medium: multilingual, ~1.5GB
# large: multilingual, ~2.9GB
```

### 2. System Dependencies

On Linux (required for microphone):

```bash
sudo apt-get install -y libasound2-dev pkg-config
```

## Configuration

Environment variables for STT configuration:

| Variable               | Description                          | Default         |
| ---------------------- | ------------------------------------ | --------------- |
| `WHISPER_BIN`          | Path to whisper.cpp executable       | `whisper`       |
| `WHISPER_MODEL_PATH`   | Path to whisper model file           | `ggml-base.bin` |
| `WHISPER_LANG`         | Language code (en, zh, auto, etc.)   | `auto`          |
| `STT_VAD_TOPIC`        | Topic to subscribe for VAD events    | `vad`           |
| `STT_VOICED_TOPIC`     | Topic to subscribe for voiced audio  | `audio.voiced`  |
| `STT_TRANSCRIPT_TOPIC` | Topic to publish transcripts         | `transcript`    |
| `STT_TEMP_DIR`         | Directory for temporary WAV files    | System temp dir |
| `WHISPER_EXTRA_ARGS`   | Extra whisper args (comma-separated) | _(empty)_       |

### Example Configuration

```bash
# Multilingual (default)
export WHISPER_BIN="./whisper.cpp/build/bin/whisper-cli"
export WHISPER_MODEL_PATH="./whisper.cpp/models/ggml-base.bin"
export WHISPER_LANG="auto"  # Auto-detect language
export WHISPER_EXTRA_ARGS="--threads,4"

# English-only (faster)
export WHISPER_MODEL_PATH="./whisper.cpp/models/ggml-base.en.bin"
export WHISPER_LANG="en"
```

## Usage

### Basic Example

```rust
use loom_core::audio::{MicConfig, MicSource, VadConfig, VadGate, SttConfig, SttEngine};
use loom_core::{EventBus, QoSLevel};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Create event bus
    let bus = Arc::new(EventBus::new().await?);
    bus.start().await?;

    // Configure and start microphone
    let mic_config = MicConfig {
        sample_rate_hz: 16_000,
        channels: 1,
        chunk_ms: 20,
        device_name: None,
        topic: "audio.mic".to_string(),
        source: "mic.primary".to_string(),
    };
    let mic_source = MicSource::new(Arc::clone(&bus), mic_config);
    mic_source.start().await?;

    // Configure and start VAD
    let vad_config = VadConfig::default();
    let vad_gate = VadGate::new(Arc::clone(&bus), vad_config);
    vad_gate.start().await?;

    // Configure and start STT
    let stt_config = SttConfig::default();
    let stt_engine = SttEngine::new(Arc::clone(&bus), stt_config);
    stt_engine.start().await?;

    // Subscribe to transcripts
    let (_sub_id, mut rx) = bus
        .subscribe(
            "transcript".to_string(),
            vec!["transcript.final".to_string()],
            QoSLevel::QosBatched,
        )
        .await?;

    // Process transcripts
    while let Some(event) = rx.recv().await {
        let text = event.metadata.get("text").unwrap();
        println!("Transcript: {}", text);
    }

    Ok(())
}
```

### Running the Example

```bash
# With default settings (expects whisper in PATH, multilingual model)
cargo run --example mic_vad_stt --features mic,vad,stt

# With custom whisper location (multilingual)
WHISPER_BIN=./whisper.cpp/build/bin/whisper-cli \
WHISPER_MODEL_PATH=./whisper.cpp/models/ggml-base.bin \
cargo run --example mic_vad_stt --features mic,vad,stt

# English-only model (faster)
WHISPER_BIN=./whisper.cpp/build/bin/whisper-cli \
WHISPER_MODEL_PATH=./whisper.cpp/models/ggml-base.en.bin \
WHISPER_LANG=en \
cargo run --example mic_vad_stt --features mic,vad,stt

# Force specific language (e.g., Chinese)
WHISPER_BIN=./whisper.cpp/build/bin/whisper-cli \
WHISPER_MODEL_PATH=./whisper.cpp/models/ggml-base.bin \
WHISPER_LANG=zh \
cargo run --example mic_vad_stt --features mic,vad,stt
```

## Event Schema

### Input: `vad.speech_start`

Signals the beginning of a speech segment.

```json
{
  "type": "vad.speech_start",
  "source": "vad",
  "metadata": {
    "sample_rate": "16000",
    "mode": "2"
  }
}
```

### Input: `audio_voiced`

Voiced audio frame (PCM16 little-endian).

```json
{
  "type": "audio_voiced",
  "source": "vad",
  "metadata": {
    "sample_rate": "16000",
    "channels": "1",
    "encoding": "pcm_s16le",
    "frame_ms": "20"
  },
  "payload": "<binary PCM data>"
}
```

### Input: `vad.speech_end`

Signals the end of a speech segment, triggers transcription.

```json
{
  "type": "vad.speech_end",
  "source": "vad",
  "metadata": {
    "sample_rate": "16000",
    "mode": "2"
  }
}
```

### Output: `transcript.final`

Transcribed text from the speech segment.

```json
{
  "type": "transcript.final",
  "source": "stt",
  "metadata": {
    "sample_rate": "16000",
    "duration_ms": "1250",
    "language": "en",
    "text": "hello world this is a test"
  },
  "payload": "<text as UTF-8 bytes>"
}
```

## Implementation Details

### Utterance Buffering

- Audio frames are buffered between `speech_start` and `speech_end`
- Minimum utterance duration: 200ms (configurable in code)
- Short utterances are automatically discarded

### WAV File Generation

- Temporary WAV files are created in `STT_TEMP_DIR`
- Format: PCM16, mono, sample rate from metadata
- Files are automatically deleted after transcription

### Whisper.cpp Integration

The STT engine calls whisper.cpp as a subprocess:

```bash
whisper.cpp/main \
  -m models/ggml-base.en.bin \
  -f /tmp/utterance_abc123.wav \
  -l en \
  --no-timestamps \
  --no-prints
```

### Error Handling

- Missing whisper binary: logs warning, continues without transcription
- Missing model file: logs warning, continues without transcription
- Transcription failure: logs error, discards utterance
- Short utterances (<200ms): silently discarded

## Performance

### Latency

- **Buffer accumulation**: Near real-time (depends on speech duration)
- **WAV write**: <10ms for typical utterances
- **Whisper transcription**:
  - `tiny`: ~100-300ms
  - `base`: ~200-500ms
  - `small`: ~500-1000ms
  - `medium`/`large`: 1-5s

### Resource Usage

- **Memory**: Minimal (buffers 1 utterance at a time)
- **CPU**: Depends on whisper model size
- **Disk I/O**: Temporary WAV files (~640KB per second of audio @ 16kHz)

## Troubleshooting

### "Whisper binary not found"

```bash
# Check if whisper is in PATH or set WHISPER_BIN
which whisper
# or
export WHISPER_BIN=/path/to/whisper.cpp/main
```

### "Whisper model not found"

```bash
# Download a model
cd whisper.cpp
bash ./models/download-ggml-model.sh base.en

# Set the model path
export WHISPER_MODEL_PATH="$(pwd)/models/ggml-base.en.bin"
```

### Empty or incorrect transcriptions

- Try a larger model (`small` or `medium`)
- Adjust VAD parameters for better speech detection
- Check audio quality and microphone settings
- Set correct language with `WHISPER_LANG`

### High CPU usage

- Use a smaller model (`tiny` or `base`)
- Add `--threads` parameter: `WHISPER_EXTRA_ARGS="--threads,2"`
- Increase VAD `hangover_ms` to reduce number of transcriptions

## Future Enhancements (P1)

- [ ] In-process streaming STT with `whisper-rs`
- [ ] Partial transcript events during long utterances
- [ ] GPU acceleration support
- [ ] Model hot-swapping
- [ ] Confidence scores from whisper output
- [ ] Multi-language auto-detection
- [ ] Speaker diarization

## Related Modules

- [VAD Guide](../../docs/VAD_GUIDE.md) - Voice Activity Detection
- [Audio README](./README.md) - Audio capture and processing
- [Examples](../../examples/) - Complete usage examples
