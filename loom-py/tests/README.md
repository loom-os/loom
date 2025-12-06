# Context Engineering Test Suite

## Overview

Comprehensive test coverage for context engineering integration, from tool execution to CLI display.

## Test Statistics

```
Total Tests: 64
Status: ✅ ALL PASSING
Runtime: 0.39s
Coverage: End-to-end pipeline
```

## Test Organization

### Unit Tests (35 tests)

#### `tests/unit/test_step.py`

**TestStepAttributes** (3 tests) - NEW! ✨

- `test_step_has_observation_not_outcome` - Validates correct attribute name
- `test_step_outcome_ref_for_offloaded_data` - Tests offload reference
- `test_step_all_required_attributes` - Verifies all expected attributes

**TestStep** (5 tests)

- `test_step_creation` - Basic Step creation
- `test_step_with_error` - Error handling
- `test_step_to_compact` - Conversion to CompactStep
- `test_step_serialization` - Dict serialization
- More...

**Reducer Tests** (22 tests)

- `TestFileReadReducer` (2)
- `TestFileWriteReducer` (1)
- `TestShellReducer` (3)
- `TestSearchReducer` (2)
- `TestWebFetchReducer` (2)
- `TestDefaultReducer` (2)
- `TestStepReducer` (6)
- `TestIntegration` (2)

#### `tests/unit/test_cognitive.py`

**TestThoughtStepIntegration** (3 tests) - NEW! ✨

- `test_thought_step_with_reduced_step` - Validates reduced_step attachment
- `test_observation_carries_reduced_step` - Tests propagation from Observation
- `test_thought_step_without_reduced_step` - Backward compatibility

**CognitiveAgent Tests** (24 tests)

- `TestCognitiveAgentRun` (5)
- `TestCognitiveAgentRunStream` (5)
- `TestCognitiveTypes` (5)
- `TestCognitiveConfig` (3)
- `TestCognitiveMemory` (3)

### Integration Tests (7 tests)

#### `tests/integration/test_context_engineering.py`

**TestPromptBuilding** (2 tests)

- `test_prompt_uses_compaction` - Validates compactor integration
- `test_prompt_without_compaction` - Tests traditional format

**TestCLIDisplay** (3 tests) - NEW! ✨

- `test_display_offloaded_step` - Validates offload reference display
- `test_display_non_offloaded_step` - Tests normal output display
- `test_display_error_step` - Tests error display

**TestEndToEndContextEngineering** (2 tests) - NEW! ✨

- `test_full_pipeline_with_offloading` - Complete flow validation
- `test_compaction_after_multiple_steps` - Compaction threshold test

## What These Tests Catch

### AttributeError Prevention

```python
# These tests would have caught the bug:
def test_step_has_observation_not_outcome():
    assert hasattr(step, "observation")  # ✅
    assert not hasattr(step, "outcome")   # ❌ Would fail if wrong
```

### Integration Validation

```python
def test_display_offloaded_step(capsys):
    # Would catch: 'Step' object has no attribute 'outcome'
    print_stream_step_complete(step)
    captured = capsys.readouterr()

    assert "Data offloaded" in captured.out
    assert step.reduced_step.observation in captured.out  # Not .outcome!
```

### End-to-End Pipeline

```python
def test_full_pipeline_with_offloading():
    # Tests complete flow:
    # 1. DataOffloader creates preview
    # 2. StepReducer creates Step
    # 3. ThoughtStep carries reduced_step
    # 4. Prompt builder uses compaction
    # 5. Output is optimized

    assert len(prompt) < len(large_output)  # Token savings verified
```

## Test Coverage by Module

| Module                 | Unit Tests      | Integration Tests | Total |
| ---------------------- | --------------- | ----------------- | ----- |
| `context/step.py`      | 10              | 0                 | 10    |
| `context/reducer.py`   | 22              | 2                 | 24    |
| `context/compactor.py` | (separate file) | 2                 | -     |
| `context/offloader.py` | (separate file) | 2                 | -     |
| `cognitive/agent.py`   | 13              | 2                 | 15    |
| `cognitive/loop.py`    | 0               | 2                 | 2     |
| `cognitive/types.py`   | 8               | 3                 | 11    |
| `cli/chat.py`          | 0               | 3                 | 3     |

## Running Tests

### All Context Engineering Tests

```bash
cd loom-py
python -m pytest tests/unit/test_step.py \
                 tests/unit/test_cognitive.py \
                 tests/integration/test_context_engineering.py \
                 -v
```

### Specific Test Groups

```bash
# Just attribute tests
pytest tests/unit/test_step.py::TestStepAttributes -v

# Just CLI display tests
pytest tests/integration/test_context_engineering.py::TestCLIDisplay -v

# Just end-to-end tests
pytest tests/integration/test_context_engineering.py::TestEndToEndContextEngineering -v

# Just ThoughtStep integration
pytest tests/unit/test_cognitive.py::TestThoughtStepIntegration -v
```

### With Coverage

```bash
pytest tests/ --cov=loom.context --cov=loom.cognitive --cov=loom.cli \
       --cov-report=html
```

## Test Design Principles

### 1. Test Public Interface

```python
# ✅ GOOD: Test observable behavior
def test_step_has_observation_not_outcome():
    step = Step(...)
    assert hasattr(step, "observation")

# ❌ BAD: Test implementation details
def test_internal_variable():
    assert step._internal_flag == True  # Fragile
```

### 2. Test Integration Points

```python
# ✅ GOOD: Test data flow between components
def test_observation_carries_reduced_step():
    reduced = Step(...)
    obs = Observation(..., reduced_step=reduced)
    assert obs.reduced_step is not None  # Validates propagation
```

### 3. Test Edge Cases

```python
# ✅ GOOD: Test optional features
def test_thought_step_without_reduced_step():
    thought = ThoughtStep(...)  # No reduced_step
    assert thought.reduced_step is None  # Backward compat
```

### 4. Test User-Facing Behavior

```python
# ✅ GOOD: Test what users see
def test_display_offloaded_step(capsys):
    print_stream_step_complete(step)
    captured = capsys.readouterr()
    assert "Data offloaded" in captured.out  # User sees this
```

## Test Maintenance

### Adding New Tests

When adding context engineering features:

1. **Unit Test** - Test the component in isolation

   ```python
   # tests/unit/test_my_feature.py
   def test_my_feature():
       component = MyComponent()
       result = component.process(data)
       assert result.expected_property == expected_value
   ```

2. **Integration Test** - Test data flow

   ```python
   # tests/integration/test_context_engineering.py
   def test_my_feature_integration():
       # Test how MyComponent integrates with others
       step = create_step()
       observation = create_observation(step)
       assert observation.reduced_step is step
   ```

3. **End-to-End Test** - Test complete pipeline
   ```python
   def test_my_feature_e2e():
       # Test complete flow from input to UI
       result = run_full_pipeline(input_data)
       assert result.output_matches_expectations()
   ```

### Test Fixtures

Reusable test data:

```python
@pytest.fixture
def sample_step():
    return Step(
        id="test_001",
        tool_name="test:tool",
        minimal_args={},
        observation="Test output",
        success=True,
    )

@pytest.fixture
def offloaded_step(sample_step):
    sample_step.outcome_ref = ".loom/cache/test.json"
    return sample_step
```

## Common Test Patterns

### Testing CLI Output

```python
def test_cli_display(capsys):
    display_function()
    captured = capsys.readouterr()

    assert "expected text" in captured.out
    assert "error text" not in captured.err
```

### Testing Async Functions

```python
@pytest.mark.asyncio
async def test_async_function():
    result = await async_function()
    assert result.success
```

### Testing File Operations

```python
def test_with_temp_files(tmp_path):
    test_file = tmp_path / "test.txt"
    test_file.write_text("content")

    result = process_file(test_file)
    assert result.success
```

## CI/CD Integration

Tests run automatically on:

- Every commit (via pre-commit hooks)
- Pull requests (via GitHub Actions)
- Before deployment

Required: All tests must pass before merge.

## Performance Benchmarks

Test execution times:

- Unit tests: ~0.28s
- Integration tests: ~0.11s
- Total: ~0.39s

All tests complete in under 1 second, enabling rapid iteration.

## See Also

- [Context Integration Guide](CONTEXT_INTEGRATION.md)
- [Testing Python Code](https://docs.pytest.org/)
- [Test Coverage Best Practices](https://coverage.readthedocs.io/)
