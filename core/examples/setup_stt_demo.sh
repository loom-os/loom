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

# Check if model exists (English-only by default for speed/accuracy)
if [ ! -f "whisper.cpp/models/ggml-base.en.bin" ]; then
    echo "📦 Downloading base.en (English-only) model..."
    cd whisper.cpp
    bash ./models/download-ggml-model.sh base.en
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
echo "  WHISPER_MODEL_PATH=./whisper.cpp/models/ggml-base.en.bin \\"
echo "  WHISPER_LANG=en \\"
echo "  cargo run --example mic_vad_stt --features mic,vad,stt"
echo ""
echo "💡 For Chinese or other languages, use the multilingual model and set language:"
echo "   WHISPER_MODEL_PATH=./whisper.cpp/models/ggml-base.bin WHISPER_LANG=zh"
echo ""
