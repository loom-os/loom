#!/bin/bash
# 完整的trace测试脚本，确保环境变量正确传递给Rust和Python

set -e

cd /home/jared/loom/demo/trace-test

echo "==================================="
echo "🔍 Loom Distributed Trace Test"
echo "==================================="
echo ""

# 1. 检查observability stack
echo "[1/5] 检查Jaeger..."
if ! docker ps | grep -q jaeger; then
    echo "⚠️  Jaeger未运行，正在启动..."
    cd ../../observability
    docker compose -f docker-compose.observability.yaml up -d
    echo "⏳ 等待10秒..."
    sleep 10
    cd -
else
    echo "✅ Jaeger运行中"
fi

# 2. 停止旧进程
echo ""
echo "[2/5] 清理旧进程..."
conda run -n loom loom down || true
sleep 2

# 3. 设置环境变量 (关键！)
echo ""
echo "[3/5] 配置环境变量..."
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
export OTEL_SERVICE_NAME=loom-trace-test
export RUST_LOG=info,loom_core=debug,loom_bridge=debug

echo "✅ 环境配置："
echo "   OTEL_EXPORTER_OTLP_ENDPOINT: $OTEL_EXPORTER_OTLP_ENDPOINT"
echo "   OTEL_SERVICE_NAME: $OTEL_SERVICE_NAME"
echo "   RUST_LOG: $RUST_LOG"

# 4. 启动loom (带环境变量)
echo ""
echo "[4/5] 启动Loom..."
echo "📝 将运行30秒生成traces..."
echo "   查看日志: tail -f logs/*.log"
echo ""

# 使用timeout并保持环境变量
timeout 30 conda run -n loom bash -c "
    export OTEL_EXPORTER_OTLP_ENDPOINT=$OTEL_EXPORTER_OTLP_ENDPOINT
    export OTEL_SERVICE_NAME=$OTEL_SERVICE_NAME
    export RUST_LOG=$RUST_LOG
    loom run
" || true

echo ""
echo "[5/5] 测试完成!"
echo ""

# 5. 显示结果
echo "==================================="
echo "📊 查看Traces"
echo "==================================="
echo ""
echo "🌐 Jaeger UI: http://localhost:16686"
echo ""
echo "🔍 查找traces："
echo "   1. Service: 选择 'loom-trace-test' 或 'trace-test-sensor'"
echo "   2. 点击 'Find Traces'"
echo "   3. 选择一个trace查看详情"
echo ""
echo "✅ 预期结果："
echo "   - 看到 3个services (sensor/processor/output)"
echo "   - 每个trace有 5-7个spans"
echo "   - spans无缝连接，无大量空白"
echo "   - 包含: sensor.emit → bridge.publish → event_bus.publish → processor → ..."
echo ""
echo "==================================="
