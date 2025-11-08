#!/bin/bash
# Debug STT issues

cd /home/jared/loom/core

echo "🔍 STT Debug Mode"
echo ""
echo "Configuration:"
echo "  VAD_MODE=0 (most permissive)"
echo "  VAD_MIN_START_MS=200 (need at least 200ms of speech)"
echo "  RUST_LOG=info (logging enabled)"
echo "  STT_KEEP_WAV=1 (keep WAV files for inspection)"
echo ""
echo "💡 Tips:"
echo "  - Speak clearly and loudly into the microphone"
echo "  - Speak for at least 0.5-1 second"
echo "  - Check /tmp/utterance_*.wav files to see captured audio"
echo "  - Play them with: aplay /tmp/utterance_*.wav"
echo ""
echo "Press Ctrl+C to stop"
echo ""

RUST_LOG=info \
VAD_MODE=0 \
VAD_MIN_START_MS=200 \
STT_KEEP_WAV=1 \
WHISPER_BIN=./whisper.cpp/build/bin/whisper-cli \
WHISPER_MODEL_PATH=./whisper.cpp/models/ggml-base.bin \
cargo run --example mic_vad_stt --features mic,vad,stt
