use crate::context::{PromptBundle, TokenBudget};
use serde_json::json;

/// Convert PromptBundle into both chat messages and a single fused text for the Responses API
pub fn promptbundle_to_messages_and_text(
    bundle: &PromptBundle,
    budget: TokenBudget,
) -> (Vec<serde_json::Value>, String) {
    // Approximate token->char ratio; conservative safety factor ~4 chars/token
    let char_budget = budget.max_input_tokens.saturating_mul(4);

    let mut system = bundle.system.clone();
    let mut context_block = String::new();
    if !bundle.context_docs.is_empty() {
        context_block.push_str("Context:\n");
        for d in &bundle.context_docs {
            context_block.push_str("- ");
            context_block.push_str(d);
            context_block.push('\n');
        }
    }
    let mut history_blocks: Vec<String> = bundle.history.clone();
    let mut instructions = bundle.instructions.clone();

    // Trim oldest history first until within char budget
    let mut assemble_len = system.len()
        + context_block.len()
        + instructions.len()
        + history_blocks.iter().map(|s| s.len()).sum::<usize>();
    while assemble_len > char_budget && !history_blocks.is_empty() {
        let removed = history_blocks.remove(0);
        assemble_len -= removed.len();
    }
    // If still too large, truncate instructions
    if assemble_len > char_budget && !instructions.is_empty() {
        let keep = instructions
            .char_indices()
            .take_while(|(i, _)| {
                *i < char_budget.saturating_sub(system.len() + context_block.len())
            })
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0);
        instructions.truncate(keep);
    }

    // Build chat messages (system + optional context + history as user + user instructions)
    let mut messages = Vec::new();
    if !system.is_empty() {
        messages.push(json!({"role": "system", "content": system}));
    }
    if !context_block.is_empty() {
        messages.push(json!({"role": "system", "content": context_block.clone()}));
    }
    for h in &history_blocks {
        messages.push(json!({"role": "user", "content": h}));
    }
    if !instructions.is_empty() {
        messages.push(json!({"role": "user", "content": instructions.clone()}));
    }

    // Fused text for Responses API input
    let mut fused = String::new();
    if !system.is_empty() {
        fused.push_str("System:\n");
        fused.push_str(&system);
        fused.push_str("\n\n");
    }
    if !context_block.is_empty() {
        fused.push_str(&context_block);
        fused.push('\n');
    }
    if !history_blocks.is_empty() {
        fused.push_str("History:\n");
        for h in &history_blocks {
            fused.push_str("- ");
            fused.push_str(h);
            fused.push('\n');
        }
        fused.push('\n');
    }
    if !instructions.is_empty() {
        fused.push_str("User:\n");
        fused.push_str(&instructions);
        fused.push('\n');
    }

    (messages, fused)
}
