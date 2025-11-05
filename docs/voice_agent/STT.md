# Speech-to-Text (STT)

Real-time transcription for Loom using whisper.cpp, wired into the Mic → VAD → STT event pipeline.

- Input events: `vad.speech_start`, `audio_voiced`, `vad.speech_end`
- Output events: `transcript.final`
- Example: `core/examples/mic_vad_stt.rs`

## Prerequisites

- Linux audio dev headers (for mic capture via cpal):
  - Debian/Ubuntu: `sudo apt-get install -y libasound2-dev pkg-config`
- whisper.cpp built and a model downloaded:
  - `git clone https://github.com/ggerganov/whisper.cpp`
  - `cd whisper.cpp && make`
  - English-only (default, faster, higher accuracy for English):
    `bash ./models/download-ggml-model.sh base.en`
  - Multilingual (use for Chinese and other languages):
    `bash ./models/download-ggml-model.sh base`

## Configuration (env)

- `WHISPER_BIN` — path to `whisper-cli` (default: `whisper`)
- `WHISPER_MODEL_PATH` — path to model (default: `ggml-base.en.bin`)
- `WHISPER_LANG` — language code, or `auto` (default: `en`)
- `WHISPER_EXTRA_ARGS` — extra args, comma-separated (e.g., `--threads,4`)
- `STT_VAD_TOPIC` — VAD events topic (default: `vad`)
- `STT_VOICED_TOPIC` — voiced frames topic (default: `audio.voiced`)
- `STT_TRANSCRIPT_TOPIC` — output transcripts topic (default: `transcript`)
- `STT_TEMP_DIR` — temp directory for WAV files (default: system temp)

Microphone/VAD (for completeness):

- `MIC_DEVICE` — optional input device name substring (e.g., `USB`)
- `MIC_CHUNK_MS` — chunk size ms (default: 20)
- `VAD_MODE` — aggressiveness 0-3 (default: 2)
- `VAD_MIN_START_MS` — min voiced ms to start (default: 60)
- `VAD_HANGOVER_MS` — hangover ms (default: 200)

## How it works

1. VAD publishes `vad.speech_start`; STT starts buffering.
2. VAD forwards voiced frames as `audio_voiced` (PCM16 LE, mono).
3. VAD publishes `vad.speech_end`; STT writes a temporary WAV and runs `whisper-cli`.
4. STT publishes `transcript.final` with the recognized text.

See also: `docs/VAD_GUIDE.md`.

## Run the example

English-only (default):

```bash
WHISPER_BIN=./whisper.cpp/build/bin/whisper-cli \
WHISPER_MODEL_PATH=./whisper.cpp/models/ggml-base.en.bin \
WHISPER_LANG=en \
cargo run -p loom-core --example mic_vad_stt --features mic,vad,stt
```

Chinese (or other non-English languages):

```bash
WHISPER_BIN=./whisper.cpp/build/bin/whisper-cli \
WHISPER_MODEL_PATH=./whisper.cpp/models/ggml-base.bin \
WHISPER_LANG=zh \
cargo run -p loom-core --example mic_vad_stt --features mic,vad,stt
```

## Troubleshooting

- Whisper prints `[BLANK_AUDIO]` or transcripts are empty:
  - Ensure mic input isn’t U8/8-bit; Loom prefers F32/I16 and falls back smartly. Set `MIC_DEVICE` if needed.
  - Check input volume in GNOME Settings → Sound → Input (or use a USB headset).
  - Verify `/tmp/utterance_*.wav` quality; play with `aplay /tmp/utterance_*.wav`.
  - Confirm `test_whisper.sh` works on a known-good sample.
  - If speaking Chinese, ensure you're using the multilingual model (`ggml-base.bin`) and set `WHISPER_LANG=zh`.
- Wrong microphone used:
  - Select the device in Settings → Sound → Input, or set `MIC_DEVICE="USB"` (substring match).
- High CPU or slow start:
  - Use `ggml-base.en.bin`, add `WHISPER_EXTRA_ARGS="--threads,2"`, or try a smaller model.

## Notes

- STT currently invokes whisper.cpp per utterance; partial (streaming) transcripts are a future enhancement.
- Temporary WAV files are deleted unless `STT_KEEP_WAV` is set.
- VAD includes a short pre-roll so the beginning of speech is preserved.
- Event QoS: VAD uses realtime; transcripts use batched by default.
