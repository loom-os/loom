"""OpenTelemetry initialization and configuration for Loom Python SDK."""

import os
from typing import Optional

from opentelemetry import trace
from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import OTLPSpanExporter
from opentelemetry.sdk.resources import Resource
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor

_initialized = False


def init_telemetry(
    service_name: Optional[str] = None,
    otlp_endpoint: Optional[str] = None,
) -> None:
    """Initialize OpenTelemetry tracing for Loom Python agents.

    Args:
        service_name: Service name for traces (default: from OTEL_SERVICE_NAME env or "loom-python")
        otlp_endpoint: OTLP collector endpoint (default: from OTEL_EXPORTER_OTLP_ENDPOINT env or http://localhost:4317)
    """
    global _initialized
    if _initialized:
        return

    # Get configuration from environment or parameters
    service_name = service_name or os.getenv("OTEL_SERVICE_NAME", "loom-python")
    otlp_endpoint = otlp_endpoint or os.getenv(
        "OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4317"
    )

    # Create resource with service attributes
    resource = Resource.create(
        {
            "service.name": service_name,
            "service.version": "0.1.0",
            "deployment.environment": os.getenv("DEPLOYMENT_ENV", "development"),
        }
    )

    # Create tracer provider
    provider = TracerProvider(resource=resource)

    # Add OTLP exporter with batch processor
    otlp_exporter = OTLPSpanExporter(endpoint=otlp_endpoint, insecure=True)
    # 🔧 Configure batch processor with shorter intervals to avoid stale spans
    span_processor = BatchSpanProcessor(
        otlp_exporter,
        max_queue_size=2048,  # Default 2048
        schedule_delay_millis=1000,  # Flush every 1 second (default 5000)
        max_export_batch_size=512,  # Default 512
    )
    provider.add_span_processor(span_processor)

    # Set as global tracer provider
    trace.set_tracer_provider(provider)

    _initialized = True
    print(
        f"[loom.tracing] OpenTelemetry initialized: service={service_name}, endpoint={otlp_endpoint}"
    )


def shutdown_telemetry() -> None:
    """Shutdown OpenTelemetry gracefully, flushing pending spans."""
    global _initialized
    if not _initialized:
        return

    provider = trace.get_tracer_provider()
    if hasattr(provider, "shutdown"):
        provider.shutdown()

    _initialized = False
    print("[loom.tracing] OpenTelemetry shutdown complete")


__all__ = ["init_telemetry", "shutdown_telemetry"]
