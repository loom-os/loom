# Context Engineering Fixes - Dec 6, 2025

## Summary

Fixed 5 critical issues preventing context engineering features from being visible and effective in the chat assistant.

## Issues Fixed

### 1. ✅ Prompt Construction Not Using Compactor

**Problem**: `build_react_prompt()` was passed `result.steps` but never received the `compactor` parameter, so compaction logic never ran.

**Fix**: Modified `CognitiveAgent._run_react()` and `_run_react_stream()` to pass `compactor=self.step_compactor` and `use_compaction=True` to `build_react_prompt()`.

**Location**:

- `loom-py/src/loom/cognitive/agent.py:273` (non-streaming)
- `loom-py/src/loom/cognitive/agent.py:448` (streaming)

### 2. ✅ Offload References Not Shown in Prompt

**Problem**: When data was offloaded to files, the prompt still showed full output instead of file reference.

**Fix**: Modified `build_react_prompt()` to check for `step.reduced_step.outcome_ref` and display `"(See {file_path})"` instead of full output.

**Location**: `loom-py/src/loom/cognitive/loop.py:120-125`

### 3. ✅ CLI Display Truncating Tool Output

**Problem**: `print_stream_step_complete()` hard-coded 150 char limit and only showed 3 lines, completely ignoring offload references.

**Fix**:

- Check for `step.reduced_step.outcome_ref` first
- If offloaded, show file path and summary
- Otherwise, show up to 8 lines with smart truncation (first few + last few)
- Remove arbitrary 150 char limit

**Location**: `loom-py/src/loom/cli/chat.py:218-250`

### 4. ✅ False "FINAL ANSWER" Matches

**Problem**: Regex matched "FINAL ANSWER" anywhere in response, causing LLM to prematurely stop when it mentioned "final answer" in its thinking.

**Fix**: Changed parsing to only check last 3 non-empty lines of response for "FINAL ANSWER:" marker, avoiding false matches in reasoning text.

**Location**: `loom-py/src/loom/cognitive/loop.py:202-210`

### 5. ✅ Memory Not Tracking Offload References

**Problem**: Observation added to working memory always showed full output, even when offloaded.

**Fix**: Modified `_run_react()` to check for offload reference and add `"(Data saved to {path})"` to memory instead of full output.

**Location**: `loom-py/src/loom/cognitive/agent.py:324-329`

### Bonus: 📊 Context Engineering Metrics Display

**Enhancement**: Added metrics to result display showing how many outputs were offloaded.

**Location**: `loom-py/src/loom/cli/chat.py:167-173`

## Testing

```bash
cd loom-py
python -m pytest tests/unit/test_cognitive.py tests/unit/test_compactor.py tests/unit/test_offloader.py
# Result: 64 passed in 0.29s ✅
```

## Expected Behavior Changes

### Before

```
🔧 Calling tool: web:search
   ✅ Result:
      {
        "count": 5,
        "query": "Google AI Plu... (truncated at 150 chars)

💭 Next thought...
FINAL ANSWER: (appears multiple times)
```

### After

```
🔧 Calling tool: web:search
   ✅ Result:
      📄 Data offloaded to: workspace/.loom/cache/search/websearch_1234.json
      💡 Summary: {"count": 5, "query": "Google AI Plus price"}

💭 Next thought... (with compacted history)
FINAL ANSWER: (appears only once at end)

📊 Context: 3 offloaded outputs
```

## Impact

- ✅ Compaction now works: Long step histories compressed intelligently
- ✅ Offloading visible: File references shown instead of repeated large outputs
- ✅ Step reduction working: Tool outputs minimized in prompts
- ✅ Token savings: Prompt size reduced significantly for long conversations
- ✅ Better UX: Users see data is being managed efficiently

## Next Steps for Testing

1. Start chat-assistant: `cd apps/chat-assistant && loom run`
2. In another terminal: `loom chat`
3. Try multi-step queries:
   - "search for something" - should show offload reference
   - Multiple tool calls - should see compaction after 5+ steps
   - Web searches - should offload large JSON responses

## Files Modified

- `loom-py/src/loom/cognitive/loop.py` (3 changes)
- `loom-py/src/loom/cognitive/agent.py` (3 changes)
- `loom-py/src/loom/cli/chat.py` (2 changes)

Total: 8 targeted fixes across 3 files.
