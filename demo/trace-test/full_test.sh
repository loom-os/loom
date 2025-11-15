#!/bin/bash
# Complete trace test script to ensure environment variables are correctly passed to Rust and Python

set -e

cd /home/jared/loom/demo/trace-test

echo "==================================="
echo "🔍 Loom Distributed Trace Test"
echo "==================================="
echo ""

# 1. Check observability stack
echo "[1/5] Checking Jaeger..."
if ! docker ps | grep -q jaeger; then
    echo "⚠️  Jaeger is not running, starting it..."
    cd ../../observability
    docker compose -f docker-compose.observability.yaml up -d
    echo "⏳ Waiting 10 seconds..."
    sleep 10
    cd -
else
    echo "✅ Jaeger is running"
fi

# 2. Stop old processes
echo ""
echo "[2/5] Stopping old processes..."
conda run -n loom loom down || true
sleep 2

# 3. Set environment variables (critical!)
echo ""
echo "[3/5] Setting environment variables..."
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
export OTEL_SERVICE_NAME=loom-trace-test
export RUST_LOG=info,loom_core=debug,loom_bridge=debug

echo "✅ Environment configured:"
echo "   OTEL_EXPORTER_OTLP_ENDPOINT: $OTEL_EXPORTER_OTLP_ENDPOINT"
echo "   OTEL_SERVICE_NAME: $OTEL_SERVICE_NAME"
echo "   RUST_LOG: $RUST_LOG"

# 4. Start loom with env vars
echo ""
echo "[4/5] Starting Loom..."
echo "📝 Running for 30 seconds to generate traces..."
echo "   View logs: tail -f logs/*.log"
echo ""

# Set ENV and run loom with 30s timeout
timeout 30 conda run -n loom bash -c "
    export OTEL_EXPORTER_OTLP_ENDPOINT=$OTEL_EXPORTER_OTLP_ENDPOINT
    export OTEL_SERVICE_NAME=$OTEL_SERVICE_NAME
    export RUST_LOG=$RUST_LOG
    loom run
" || true

echo ""
echo "[5/5] Test complete!"
echo ""

# 5. Show results
echo "==================================="
echo "📊 View Traces"
echo "==================================="
echo ""
echo "🌐 Jaeger UI: http://localhost:16686"
echo ""
echo "🔍 Find traces:"
echo "   1. Service: select 'loom-trace-test' or 'trace-test-sensor'"
echo "   2. Click 'Find Traces'"
echo "   3. Select a trace to view details"
echo ""
echo "✅ Expected results:"
echo "   - 3 services visible (sensor/processor/output)"
echo "   - Each trace has 5-7 spans"
echo "   - Spans are seamlessly connected with no large gaps"
echo "   - Includes: sensor.emit → bridge.publish → event_bus.publish → processor → ..."
echo ""
echo "==================================="
