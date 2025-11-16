"""Tests for memory integration in context."""

import hashlib
import time
from unittest.mock import AsyncMock, MagicMock

import pytest

from loom.context import Context
from loom.proto import memory_pb2 as pb_memory


class TestMemoryHashGeneration:
    """Test plan hash generation."""

    def test_plan_hash_consistency(self):
        """Test that identical plans generate identical hashes."""
        symbol = "BTC"
        action = "BUY"
        reasoning = "Strong bullish trend"

        # Generate hash twice - should be identical
        hash1 = hashlib.md5(f"{symbol}|{action}|{reasoning}".encode()).hexdigest()[:8]
        hash2 = hashlib.md5(f"{symbol}|{action}|{reasoning}".encode()).hexdigest()[:8]

        assert hash1 == hash2
        assert len(hash1) == 8  # Truncated to 8 chars

    def test_plan_hash_different_for_different_plans(self):
        """Test that different plans generate different hashes."""
        hash1 = hashlib.md5("BTC|BUY|Bullish".encode()).hexdigest()[:8]
        hash2 = hashlib.md5("BTC|SELL|Bearish".encode()).hexdigest()[:8]
        hash3 = hashlib.md5("ETH|BUY|Bullish".encode()).hexdigest()[:8]

        assert hash1 != hash2
        assert hash1 != hash3
        assert hash2 != hash3


class TestMemoryIntegrationWithMockClient:
    """Test memory operations with mocked gRPC client."""

    @pytest.fixture
    def mock_client(self):
        """Create a mock BridgeClient."""
        client = MagicMock()
        # Mock async methods
        client.save_plan = AsyncMock()
        client.get_recent_plans = AsyncMock()
        client.check_duplicate = AsyncMock()
        client.mark_executed = AsyncMock()
        client.check_executed = AsyncMock()
        client.get_execution_stats = AsyncMock()
        return client

    @pytest.fixture
    def context(self, mock_client):
        """Create a Context with mocked client."""
        ctx = Context(agent_id="test-agent", client=mock_client)
        return ctx

    @pytest.mark.asyncio
    async def test_save_plan_success(self, context, mock_client):
        """Test successful plan saving."""
        # Mock response
        mock_response = pb_memory.SavePlanResponse(
            success=True,
            plan_hash="abc12345",
            error_message="",
        )
        mock_client.save_plan.return_value = mock_response

        # Call save_plan
        plan_hash = await context.save_plan(
            symbol="BTC",
            action="BUY",
            confidence=0.85,
            reasoning="Bullish trend",
            method="llm",
        )

        # Verify
        assert plan_hash == "abc12345"
        mock_client.save_plan.assert_called_once()

        # Check the request structure
        call_args = mock_client.save_plan.call_args
        request = call_args[0][0]
        assert request.session_id == "test-agent"
        assert request.plan.symbol == "BTC"
        assert request.plan.action == "BUY"
        assert abs(request.plan.confidence - 0.85) < 0.01

    @pytest.mark.asyncio
    async def test_save_plan_failure(self, context, mock_client):
        """Test plan saving failure handling."""
        # Mock failure response
        mock_response = pb_memory.SavePlanResponse(
            success=False,
            plan_hash="",
            error_message="Database error",
        )
        mock_client.save_plan.return_value = mock_response

        # Should raise exception
        with pytest.raises(RuntimeError, match="Failed to save plan"):
            await context.save_plan(
                symbol="BTC",
                action="BUY",
                confidence=0.8,
                reasoning="Test",
                method="llm",
            )

    @pytest.mark.asyncio
    async def test_get_recent_plans(self, context, mock_client):
        """Test retrieving recent plans."""
        # Mock response with multiple plans
        now_ms = int(time.time() * 1000)
        mock_plans = [
            pb_memory.PlanRecord(
                timestamp_ms=now_ms - 60000,
                symbol="BTC",
                action="BUY",
                confidence=0.8,
                reasoning="Bullish",
                plan_hash="hash1",
                method="llm",
            ),
            pb_memory.PlanRecord(
                timestamp_ms=now_ms - 120000,
                symbol="BTC",
                action="HOLD",
                confidence=0.6,
                reasoning="Wait",
                plan_hash="hash2",
                method="rule",
            ),
        ]
        mock_response = pb_memory.GetRecentPlansResponse(
            plans=mock_plans,
            success=True,
            error_message="",
        )
        mock_client.get_recent_plans.return_value = mock_response

        # Get plans
        plans = await context.get_recent_plans(symbol="BTC", limit=5)

        # Verify
        assert len(plans) == 2
        assert plans[0]["action"] == "BUY"
        assert plans[1]["action"] == "HOLD"
        assert "timestamp_ms" in plans[0]
        assert "plan_hash" in plans[0]

    @pytest.mark.asyncio
    async def test_check_duplicate_plan_found(self, context, mock_client):
        """Test duplicate plan detection - duplicate found."""
        now_ms = int(time.time() * 1000)
        mock_duplicate_plan = pb_memory.PlanRecord(
            timestamp_ms=now_ms - 60000,
            symbol="BTC",
            action="BUY",
            confidence=0.8,
            reasoning="Bullish",
            plan_hash="abc123",
            method="llm",
        )
        mock_response = pb_memory.CheckDuplicateResponse(
            is_duplicate=True,
            duplicate_plan=mock_duplicate_plan,
            time_since_duplicate_ms=60000,
        )
        mock_client.check_duplicate.return_value = mock_response

        # Check duplicate
        is_dup, dup_info = await context.check_duplicate_plan(
            symbol="BTC",
            action="BUY",
            reasoning="Bullish",
            time_window_sec=300,
        )

        # Verify
        assert is_dup is True
        assert dup_info["plan_hash"] == "abc123"
        assert dup_info["time_since_ms"] == 60000

    @pytest.mark.asyncio
    async def test_check_duplicate_plan_not_found(self, context, mock_client):
        """Test duplicate plan detection - no duplicate."""
        mock_response = pb_memory.CheckDuplicateResponse(
            is_duplicate=False,
            time_since_duplicate_ms=0,
        )
        mock_client.check_duplicate.return_value = mock_response

        # Check duplicate
        is_dup, dup_info = await context.check_duplicate_plan(
            symbol="BTC",
            action="SELL",
            reasoning="Bearish",
            time_window_sec=300,
        )

        # Verify
        assert is_dup is False
        assert dup_info is None

    @pytest.mark.asyncio
    async def test_mark_plan_executed(self, context, mock_client):
        """Test marking plan as executed."""
        mock_response = pb_memory.MarkExecutedResponse(
            success=True,
            error_message="",
        )
        mock_client.mark_executed.return_value = mock_response

        # Mark executed
        await context.mark_plan_executed(
            plan_hash="abc123",
            symbol="BTC",
            action="BUY",
            confidence=0.8,
            status="success",
            executed=True,
            order_id="order-123",
            order_size_usdt=100.0,
        )

        # Verify call was made
        mock_client.mark_executed.assert_called_once()
        call_args = mock_client.mark_executed.call_args
        request = call_args[0][0]
        assert request.plan_hash == "abc123"
        assert request.execution.order_id == "order-123"
        assert request.execution.executed is True

    @pytest.mark.asyncio
    async def test_check_plan_executed_true(self, context, mock_client):
        """Test checking if plan was executed - executed."""
        mock_execution = pb_memory.ExecutionRecord(
            timestamp_ms=int(time.time() * 1000),
            plan_hash="abc123",
            symbol="BTC",
            action="BUY",
            confidence=0.8,
            status="success",
            executed=True,
            order_id="order-123",
            order_size_usdt=100.0,
        )
        mock_response = pb_memory.CheckExecutedResponse(
            is_executed=True,
            execution=mock_execution,
        )
        mock_client.check_executed.return_value = mock_response

        # Check executed
        is_exec, exec_info = await context.check_plan_executed("abc123")

        # Verify
        assert is_exec is True
        assert exec_info["order_id"] == "order-123"
        assert exec_info["status"] == "success"

    @pytest.mark.asyncio
    async def test_check_plan_executed_false(self, context, mock_client):
        """Test checking if plan was executed - not executed."""
        mock_response = pb_memory.CheckExecutedResponse(
            is_executed=False,
        )
        mock_client.check_executed.return_value = mock_response

        # Check executed
        is_exec, exec_info = await context.check_plan_executed("abc123")

        # Verify
        assert is_exec is False
        assert exec_info is None

    @pytest.mark.asyncio
    async def test_get_execution_stats(self, context, mock_client):
        """Test retrieving execution statistics."""
        # Create mock execution records
        mock_executions = [
            pb_memory.ExecutionRecord(
                timestamp_ms=int(time.time() * 1000) - i * 60000,
                plan_hash=f"hash{i}",
                symbol="BTC",
                action=["BUY", "SELL", "HOLD"][i % 3],
                confidence=0.8,
                status="success" if i < 6 else "error",
                executed=True,
                order_id=f"order-{i}",
                order_size_usdt=100.0,
            )
            for i in range(10)
        ]

        mock_response = pb_memory.GetExecutionStatsResponse(
            total_executions=10,
            successful_executions=6,
            failed_executions=4,
            win_rate=0.6,
            duplicate_prevented=2,
            recent_executions=mock_executions,
        )
        mock_client.get_execution_stats.return_value = mock_response

        # Get stats
        stats = await context.get_execution_stats("BTC")

        # Verify
        assert stats["total_executions"] == 10
        assert stats["successful_executions"] == 6
        assert abs(stats["win_rate"] - 0.6) < 0.01
        assert len(stats["recent_executions"]) == 10

    @pytest.mark.asyncio
    async def test_rpc_error_handling(self, context, mock_client):
        """Test RPC error handling."""
        # Mock RPC failure
        mock_client.save_plan.side_effect = Exception("Network error")

        # Should propagate exception
        with pytest.raises(Exception, match="Network error"):
            await context.save_plan(
                symbol="BTC",
                action="BUY",
                confidence=0.8,
                reasoning="Test",
                method="llm",
            )
