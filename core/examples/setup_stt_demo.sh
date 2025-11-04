#!/bin/bash
# Quick setup script for STT demo

set -e

echo "🎙️  Loom STT Demo Setup"
echo ""

# Check if whisper.cpp directory exists
if [ ! -d "whisper.cpp" ]; then
    echo "📦 whisper.cpp not found. Downloading..."
    git clone https://github.com/ggerganov/whisper.cpp
    cd whisper.cpp
    make -j
    cd ..
    echo "✅ Built whisper.cpp"
else
    echo "✅ Found whisper.cpp"
fi

# Check if model exists
if [ ! -f "whisper.cpp/models/ggml-base.bin" ]; then
    echo "📦 Downloading base multilingual model..."
    cd whisper.cpp
    bash ./models/download-ggml-model.sh base
    cd ..
    echo "✅ Downloaded model"
else
    echo "✅ Found model"
fi

echo ""
echo "🚀 Ready to run STT demo!"
echo ""
echo "Run with:"
echo "  WHISPER_BIN=./whisper.cpp/build/bin/whisper-cli \\"
echo "  WHISPER_MODEL_PATH=./whisper.cpp/models/ggml-base.bin \\"
echo "  cargo run --example mic_vad_stt --features mic,vad,stt"
echo ""
echo "💡 The model supports auto language detection."
echo "   Set WHISPER_LANG=en for English-only (faster) or WHISPER_LANG=zh for Chinese."
echo ""
