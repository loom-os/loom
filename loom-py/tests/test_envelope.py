"""Unit tests for loom.Envelope."""

from loom import Envelope
from loom.proto.generated import event_pb2


class TestEnvelope:
    """Test Envelope class functionality."""

    def test_envelope_creation(self) -> None:
        """Test basic envelope creation."""
        env = Envelope.new(
            type="test.event",
            payload=b"test payload",
            thread_id="thread-1",
            correlation_id="corr-1",
        )

        # ID is auto-generated UUID
        assert env.id is not None
        assert env.type == "test.event"
        assert env.payload == b"test payload"
        assert env.metadata["loom.thread_id"] == "thread-1"
        assert env.metadata["loom.correlation_id"] == "corr-1"

    def test_envelope_from_proto(self) -> None:
        """Test creating envelope from protobuf."""
        proto_event = event_pb2.Event(
            id="evt-001",
            type="test.type",
            timestamp_ms=123456789,
            source="test-source",
            payload=b"test data",
            metadata={
                "loom.thread_id": "thread-1",
                "loom.sender": "agent-1",
            },
        )

        env = Envelope.from_proto(proto_event)
        assert env.id == "evt-001"
        assert env.type == "test.type"
        assert env.payload == b"test data"
        assert env.thread_id == "thread-1"
        assert env.sender == "agent-1"

    def test_envelope_to_proto(self) -> None:
        """Test converting envelope to protobuf."""
        env = Envelope.new(
            type="test.event",
            payload=b"test payload",
            thread_id="thread-1",
            sender="agent-1",
        )

        proto_event = env.to_proto(event_pb2.Event)
        assert proto_event.id == env.id
        assert proto_event.type == "test.event"
        assert proto_event.payload == b"test payload"
        assert proto_event.metadata["loom.thread_id"] == "thread-1"
        assert proto_event.metadata["loom.sender"] == "agent-1"

    def test_envelope_roundtrip(self) -> None:
        """Test envelope proto conversion roundtrip."""
        original = Envelope.new(
            type="roundtrip.test",
            payload=b"roundtrip data",
            thread_id="thread-rt",
            correlation_id="corr-rt",
            sender="sender-rt",
            reply_to="reply-topic",
        )

        proto = original.to_proto(event_pb2.Event)
        restored = Envelope.from_proto(proto)

        assert restored.id == original.id
        assert restored.type == original.type
        assert restored.payload == original.payload
        assert restored.thread_id == original.thread_id
        assert restored.correlation_id == original.correlation_id
        assert restored.sender == original.sender
        assert restored.reply_to == original.reply_to

    def test_envelope_with_ttl(self) -> None:
        """Test envelope with TTL."""
        env = Envelope.new(
            type="test.ttl",
            payload=b"ttl test",
            ttl_ms=5000,
        )

        assert env.ttl_ms == 5000
        assert env.metadata.get("loom.ttl_ms") == "5000"

    def test_envelope_without_optional_fields(self) -> None:
        """Test envelope with minimal fields."""
        env = Envelope.new(
            type="minimal.event",
            payload=b"minimal",
        )

        assert env.id is not None  # Auto-generated
        assert env.type == "minimal.event"
        assert env.payload == b"minimal"
        assert env.thread_id is None
        assert env.correlation_id is None
        assert env.sender is None
