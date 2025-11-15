#!/bin/bash
# Quick test script for trace-test demo

set -e

echo "==================================="
echo "Loom Trace Test - Quick Start"
echo "==================================="
echo ""

# Check if observability stack is running
echo "[1/4] Checking observability stack..."
if ! docker ps | grep -q jaeger; then
    echo "❌ Jaeger not running. Starting observability stack..."
    cd ../../observability
    docker compose -f docker-compose.observability.yaml up -d
    echo "⏳ Waiting 10 seconds for services to start..."
    sleep 10
    cd -
else
    echo "✅ Jaeger is running"
fi

# Set environment variables
echo ""
echo "[2/4] Setting environment variables..."
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
export OTEL_SERVICE_NAME=trace-test
echo "✅ OTEL_EXPORTER_OTLP_ENDPOINT=$OTEL_EXPORTER_OTLP_ENDPOINT"
echo "✅ OTEL_SERVICE_NAME=$OTEL_SERVICE_NAME"

# Start loom
echo ""
echo "[3/4] Starting Loom demo..."
echo "📝 This will run for 30 seconds to generate traces..."
echo ""

# Run with timeout
timeout 30 conda run -n loom loom run || true

echo ""
echo "[4/4] ✅ Demo completed!"
echo ""
echo "==================================="
echo "📊 View traces in Jaeger:"
echo "   http://localhost:16686"
echo ""
echo "🔍 What to look for:"
echo "   1. Select service: 'trace-test-sensor'"
echo "   2. Click 'Find Traces'"
echo "   3. You should see traces with 7+ spans"
echo "   4. Click a trace to see the full flow:"
echo "      sensor.emit_data → ... → processor → ... → output"
echo "==================================="
