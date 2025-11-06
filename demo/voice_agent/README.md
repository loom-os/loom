# Voice Agent Demo (Mic → VAD → STT → Wake → LLM → TTS)

This demo runs a fully event-driven voice assistant loop using the Loom core runtime:

Mic → VAD → STT → Wake → LLM → TTS

- Mic capture (`cpal`) publishes `audio_chunk` events
- VAD (`webrtc-vad`) emits `vad.speech_start/end` and forwards `audio_voiced`
- STT buffers each utterance and calls `whisper.cpp` to produce `transcript.final`
- Wake word detector matches phrases like "hey loom" and arms the next utterance as a query
- LLM client (vLLM/OpenAI-compatible) generates an answer from a `PromptBundle`
- Local TTS (Piper preferred, falls back to espeak-ng) speaks the reply

## Prerequisites

- Rust toolchain (1.74+ recommended)
- Linux/macOS with microphone access
- ALSA dev headers on Linux for `cpal` (Ubuntu/Debian):
  - `sudo apt-get update && sudo apt-get install -y libasound2-dev pkg-config`
- whisper.cpp binary and a model file (for STT)
  - Build from https://github.com/ggerganov/whisper.cpp
  - Example model: `ggml-base.en.bin` (English-only, faster) or `ggml-base.bin`
- Optional: vLLM running locally (OpenAI API compatible)
  - Default base URL: `http://localhost:8000/v1`
  - You can also point to a cloud provider compatible with OpenAI API
- Optional TTS engines:
  - Piper (better quality) with a voice model, or
  - espeak-ng (widely available)

## Build

From the repo root:

```bash
cargo build -p voice_agent
```

If you prefer to build everything:

```bash
cargo build --workspace --features mic,vad,stt,wake,tts
```

## Configure

You can configure the demo via a TOML file or environment variables.

### Option A — voice_agent.toml (recommended)

Create a `voice_agent.toml` in your working directory (or set `VOICE_AGENT_CONFIG=/path/to/file`). Only set what you need; unspecified fields use sane defaults and env fallbacks.

Example:

```toml
query_topic = "query"

[llm]
base_url = "http://localhost:8000/v1"
model = "qwen2.5-0.5b-instruct"
temperature = 0.6
request_timeout_ms = 30000
system_prompt = "You are Loom's helpful and concise voice assistant."

[tts]
voice = "/models/piper/en_US-amy-medium.onnx"
rate = 1.0
volume = 1.0
sample_rate = 16000
player = "aplay"

[mic]
device_name = "alsa"
chunk_ms = 20
sample_rate_hz = 16000
channels = 1

[vad]
mode = 2
frame_ms = 20
min_start_ms = 60
hangover_ms = 200

[stt]
whisper_bin = "./loom-audio/whisper.cpp/build/bin/whisper"
whisper_model = "./loom-audio/whisper.cpp/models/ggml-base.en.bin"
language = "en"
extra_args = ["--threads", "4"]

[wake]
phrases = ["hey loom", "loom"]
max_distance = 1
match_anywhere = true
jaro_winkler_threshold = 0.9
min_query_chars = 4
```

## Get default STT/TTS models

Use the helper script to download a small, fast Whisper model for STT and a Piper voice for TTS. It supports Hugging Face mirrors via `HF_ENDPOINT`.

```bash
# From repo root
bash demo/voice_agent/scripts/setup_models.sh

# Customize (optional)
# WHISPER_MODEL: ggml-base.en.bin | ggml-small.en.bin | ggml-base.bin | ...
# PIPER_VOICE:   en_US-amy-medium | en_US-lessac-high | ...
# HF_ENDPOINT:   set a mirror domain if needed (e.g., https://hf-mirror.com)
# Example:
# WHISPER_MODEL=ggml-small.en.bin PIPER_VOICE=en_US-lessac-high \
#   HF_ENDPOINT=https://hf-mirror.com bash demo/voice_agent/scripts/setup_models.sh
```

Defaults and where files are placed:

- Whisper model → `./loom-audio/whisper.cpp/models/ggml-base.en.bin`
- Piper voice → `./demo/voice_agent/models/piper/en_US-amy-medium/en_US-amy-medium.onnx`

Update your `voice_agent.toml` to point to these files if your paths differ. The script prints the exact paths on completion.

### Option B — Environment variables

Environment variables (sane defaults included):

- Microphone

  - `MIC_DEVICE` – substring of input device name to select (optional)
  - `MIC_CHUNK_MS` – chunk size in ms (default 20)
  - `MIC_TOPIC` – default `audio.mic`

- VAD

  - `VAD_MODE` – 0..3 aggressiveness (default 2)
  - `VAD_FRAME_MS` – 10|20|30 (default 20)
  - `VAD_MIN_START_MS` – min voiced duration to start (default 60)
  - `VAD_HANGOVER_MS` – hangover after last voice (default 200)

- STT (whisper.cpp)

  - `WHISPER_BIN` – path to whisper executable (default `whisper` on PATH)
  - `WHISPER_MODEL_PATH` – path to model (default `ggml-base.en.bin`)
  - `WHISPER_LANG` – e.g., `en`, `auto` (default `en`)
  - `WHISPER_EXTRA_ARGS` – comma-separated extra flags, e.g. `--threads,4`
  - `STT_TEMP_DIR` – where to write wav files (default system temp)

- Wake word

  - `WAKE_PHRASES` – comma list, default `hey loom,loom`
  - `WAKE_FUZZY_DISTANCE` – Levenshtein per-token (default 1)
  - `WAKE_JW_THRESHOLD` – Jaro-Winkler gate (default 0.90)
  - `WAKE_MATCH_ANYWHERE` – allow match anywhere (default true)

- LLM (OpenAI-compatible)

  - `VLLM_BASE_URL` – default `http://localhost:8000/v1`
  - `VLLM_MODEL` – default `qwen2.5-0.5b-instruct`
  - `VLLM_API_KEY` – if your backend requires it
  - `REQUEST_TIMEOUT_MS` – default 30000
  - `VLLM_TEMPERATURE` – default 0.7
  - `VOICE_SYSTEM_PROMPT` – system message for the assistant

- TTS
  - Piper: `PIPER_BIN`, `PIPER_VOICE`, `PIPER_VOICE_DIR`
  - espeak-ng: `ESPEAK_BIN`
  - Playback: `TTS_PLAYER` (aplay|paplay|ffplay), optional
  - Options: `TTS_VOICE`, `TTS_RATE`, `TTS_VOLUME`, `TTS_SAMPLE_RATE`

## Run

```bash
# Example minimal config (adjust paths)
export WHISPER_BIN=./core/whisper.cpp/build/bin/whisper
export WHISPER_MODEL_PATH=./core/whisper.cpp/models/ggml-base.en.bin
export VLLM_BASE_URL=http://localhost:8000/v1
export VLLM_MODEL=qwen2.5-0.5b-instruct
# Optional: Piper
# export PIPER_BIN=$(which piper)
# export PIPER_VOICE=/path/to/en_US-amy-medium.onnx

cargo run -p voice_agent
```

Speak a wake phrase like "hey loom". The next utterance is treated as your query. The assistant will reply and TTS will speak it.

## Serve a local LLM with vLLM (Qwen2.5-0.5B-Instruct)

vLLM typically requires a GPU. If you don’t have one, consider using an OpenAI-compatible alternative (e.g., Ollama), then set `llm.base_url` and `llm.model` accordingly in your `voice_agent.toml`.

- Model: `Qwen/Qwen2.5-0.5B-Instruct` on Hugging Face

### Option A — Docker

```bash
docker run --gpus all --rm -it -p 8000:8000 \
  -e HF_TOKEN=$HF_TOKEN \
  vllm/vllm-openai:latest \
  --host 0.0.0.0 \
  --port 8000 \
  --model Qwen/Qwen2.5-0.5B-Instruct \
  --max-model-len 4096 \
  --gpu-memory-utilization 0.90
```

### Option B — Python (pip)

```bash
pip install "vllm>=0.6.0.post1"  # or a recent version compatible with your CUDA/ROCm
python -m vllm.entrypoints.openai.api_server \
  --host 0.0.0.0 \
  --port 8000 \
  --model Qwen/Qwen2.5-0.5B-Instruct \
  --max-model-len 4096
```

Once the server is up, wire the demo via `voice_agent.toml`:

```toml
[llm]
base_url = "http://localhost:8000/v1"
model = "Qwen/Qwen2.5-0.5B-Instruct" # must match what vLLM serves
temperature = 0.6
request_timeout_ms = 30000
```

If you run vLLM on a different host/port, update `base_url` accordingly (e.g., `http://127.0.0.1:8080/v1`). If your model requires auth, set `HF_TOKEN` in Docker, or pass credentials as appropriate for your environment.

### Option C — Direct `vllm serve` CLI

If you installed vLLM as a tool, this mirrors your example:

```bash
vllm serve "Qwen/Qwen2.5-0.5B-Instruct" \
  --port 8000 \
  --host 0.0.0.0 \
  --max-model-len 1024
```

Tip: You can set the name that the API reports with `--served-model-name`, then use the exact same value in your `voice_agent.toml`:

```bash
vllm serve ./models/Qwen2.5-0.5B-Instruct \
  --host 0.0.0.0 --port 8000 \
  --served-model-name Qwen/Qwen2.5-0.5B-Instruct
```

### Network-restricted environments

If accessing huggingface.co is unreliable or blocked:

- Use a mirror for the Hugging Face Hub (if your environment provides one). For example:

```bash
export HF_ENDPOINT="https://hf-mirror.com"   # Example mirror domain; replace with one you trust
```

- Or pre-download models, then run vLLM against the local folder. This avoids any network access at runtime:

```bash
pip install -U "huggingface_hub[cli]"
huggingface-cli download Qwen/Qwen2.5-0.5B-Instruct \
  --local-dir ./models/Qwen2.5-0.5B-Instruct \
  --local-dir-use-symlinks False

# Serve the local directory and fix the API model name for the client
vllm serve ./models/Qwen2.5-0.5B-Instruct \
  --host 0.0.0.0 --port 8000 \
  --served-model-name Qwen/Qwen2.5-0.5B-Instruct

# Match the served model name in your app config
cat > voice_agent.toml <<'EOF'
[llm]
base_url = "http://localhost:8000/v1"
model = "Qwen/Qwen2.5-0.5B-Instruct"
EOF
```

Advanced tips:

- Set `HF_HOME=/path/to/cache` to control where models are cached.
- vLLM also supports `--download-dir /path/to/cache` to cache downloads under a specific directory.
- For private models, set `HF_TOKEN` (Docker env or shell env) before serving.

## Architecture

High-level topics and event types:

- `audio.mic`: `audio_chunk` frames with metadata (rate/channels/device)
- `audio.voiced`: `audio_voiced` frames (mono) gated by VAD
- `vad`: `vad.speech_start`, `vad.speech_end`
- `transcript`: `transcript.final` with `text` in metadata and payload
- `wake`: `wake_word_detected` with matched phrase and session_id
- `query`: `user.query` with session_id and `text`
- `tts`: `tts.start`, `tts.done`, `tts.error` (observability)

The demo uses the Loom `ActionBroker` to invoke built-in capabilities:

- `llm.generate` – wraps the HTTP client and `PromptBundle` adapter
- `tts.speak` – chooses Piper or espeak-ng, degrades gracefully if missing

## Troubleshooting

- Microphone not found: set `MIC_LOG_DEVICES=1` to list devices and set `MIC_DEVICE` accordingly.
- No STT: ensure `WHISPER_BIN` and `WHISPER_MODEL_PATH` exist; check CPU load and use a smaller model.
- LLM errors: confirm your base URL/model, and that the backend is running. Inspect logs for HTTP status/output.
- No audio playback: install `aplay` (ALSA), `paplay` (PulseAudio), or `ffplay` (FFmpeg). The WAV file path is logged when synthesis succeeds.

## Notes

- STT runs per-utterance via the whisper CLI for reliability and simplicity. For streaming STT, consider an in-process engine in a future iteration.
- This demo is event-driven end-to-end; each stage can fail independently without blocking the others.

## Project structure (quick review)

- `loom-proto`: shared protobuf-generated types; re-exported by `loom-core` as `loom_core::proto`
- `loom-audio`: mic/VAD/STT/wake/TTS modules and providers
- `core` (loom-core): runtime (event bus, agent runtime, router, LLM client, etc.)
- `demo/voice_agent`: this E2E app, with `voice_agent.toml` config

Tips:

- Prefer keeping `voice_agent.toml` at the demo root and run the app from that directory. Otherwise set `VOICE_AGENT_CONFIG` to an absolute path.
- The model name in your config must match the model name vLLM serves. For HF IDs, prefer the full `org/name` form (e.g., `Qwen/Qwen2.5-0.5B-Instruct`).
