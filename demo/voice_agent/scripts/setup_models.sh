#!/usr/bin/env bash
# Download default STT and TTS models for the Voice Agent demo.
# - Whisper (STT): ggml-base.en.bin for whisper.cpp
# - Piper (TTS): en_US-amy-medium (.onnx + .json)
#
# Usage:
#   ./scripts/setup_models.sh                 # default paths under ./loom-audio and ./models
#   HF_ENDPOINT=https://hf-mirror.com ./scripts/setup_models.sh  # use mirror
#   WHISPER_MODEL=ggml-small.en.bin ./scripts/setup_models.sh    # choose whisper size
#   PIPER_VOICE=en_US-lessac-high ./scripts/setup_models.sh      # choose different voice
#
# Notes:
# - This script does NOT build the whisper.cpp binary. Build it separately if needed.
# - Update voice_agent.toml to point to the downloaded files if you use non-default paths.

set -euo pipefail

HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/.." && pwd)

# Allow mirror override (Hugging Face)
HF_ENDPOINT="${HF_ENDPOINT:-https://huggingface.co}"

# ===============
# Whisper (STT)
# ===============

WHISPER_DIR_DEFAULT="$ROOT/../../loom-audio/whisper.cpp/models"
WHISPER_DIR="${WHISPER_DIR:-$WHISPER_DIR_DEFAULT}"
WHISPER_MODEL="${WHISPER_MODEL:-ggml-base.en.bin}"
WHISPER_PATH="$WHISPER_DIR/$WHISPER_MODEL"

mkdir -p "$WHISPER_DIR"

if [[ ! -f "$WHISPER_PATH" ]]; then
  echo "[STT] Downloading whisper model: $WHISPER_MODEL"
  # Try direct HF link to ggerganov/whisper.cpp model repo
  # e.g., https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin
  WHISPER_URL="$HF_ENDPOINT/ggerganov/whisper.cpp/resolve/main/$WHISPER_MODEL?download=true"
  if command -v curl >/dev/null 2>&1; then
    curl -fL "$WHISPER_URL" -o "$WHISPER_PATH"
  else
    wget -O "$WHISPER_PATH" "$WHISPER_URL"
  fi
else
  echo "[STT] Whisper model already present: $WHISPER_PATH"
fi

# ===============
# Piper (TTS)
# ===============

PIPER_DIR_DEFAULT="$ROOT/models/piper"
PIPER_DIR="${PIPER_VOICE_DIR:-$PIPER_DIR_DEFAULT}"
PIPER_VOICE_NAME="${PIPER_VOICE:-en_US-amy-medium}"
PIPER_LANG_PREFIX="${PIPER_LANG_PREFIX:-en}"
PIPER_SUBDIR="$PIPER_DIR/$PIPER_VOICE_NAME"

mkdir -p "$PIPER_SUBDIR"

# Rhasspy piper voices on HF follow the structure:
#   rhasspy/piper-voices/<lang>/<voice>/onnx/<voice>.onnx
#   rhasspy/piper-voices/<lang>/<voice>/<voice>.onnx.json

PIPER_BASE="$HF_ENDPOINT/rhasspy/piper-voices/resolve/main/$PIPER_LANG_PREFIX/$PIPER_VOICE_NAME"
PIPER_ONNX_URL="$PIPER_BASE/onnx/$PIPER_VOICE_NAME.onnx?download=true"
PIPER_JSON_URL="$PIPER_BASE/$PIPER_VOICE_NAME.onnx.json?download=true"

PIPER_ONNX_PATH="$PIPER_SUBDIR/$PIPER_VOICE_NAME.onnx"
PIPER_JSON_PATH="$PIPER_SUBDIR/$PIPER_VOICE_NAME.onnx.json"

if [[ ! -f "$PIPER_ONNX_PATH" ]]; then
  echo "[TTS] Downloading Piper voice model: $PIPER_VOICE_NAME"
  if command -v curl >/dev/null 2>&1; then
    curl -fL "$PIPER_ONNX_URL" -o "$PIPER_ONNX_PATH"
  else
    wget -O "$PIPER_ONNX_PATH" "$PIPER_ONNX_URL"
  fi
else
  echo "[TTS] Piper ONNX already present: $PIPER_ONNX_PATH"
fi

if [[ ! -f "$PIPER_JSON_PATH" ]]; then
  echo "[TTS] Downloading Piper voice config (json)"
  if command -v curl >/dev/null 2>&1; then
    curl -fL "$PIPER_JSON_URL" -o "$PIPER_JSON_PATH"
  else
    wget -O "$PIPER_JSON_PATH" "$PIPER_JSON_URL"
  fi
else
  echo "[TTS] Piper JSON already present: $PIPER_JSON_PATH"
fi

echo ""
echo "Done. Update your voice_agent.toml if needed:"
echo "  [stt]"
echo "  whisper_model = \"$WHISPER_PATH\""
echo ""
echo "  [tts]"
echo "  voice = \"$PIPER_ONNX_PATH\""
echo ""
echo "Tip: if you use a mirror, export HF_ENDPOINT=https://your.mirror before running this script."
