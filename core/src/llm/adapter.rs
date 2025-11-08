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

    // Compute current assembled size in characters
    let mut assembled_chars = system.chars().count()
        + context_block.chars().count()
        + instructions.chars().count()
        + history_blocks
            .iter()
            .map(|s| s.chars().count())
            .sum::<usize>();

    // Add overhead for fused text formatting labels
    let mut formatting_overhead = 0;
    if !system.is_empty() {
        formatting_overhead += "System:\n\n\n".len(); // "System:\n" + "\n\n"
    }
    if !context_block.is_empty() {
        formatting_overhead += "\n".len(); // extra newline after context
    }
    if !history_blocks.is_empty() {
        formatting_overhead += "History:\n\n".len(); // "History:\n" + final "\n"
        formatting_overhead += history_blocks.len() * "- \n".len(); // "- " + "\n" per item
    }
    if !instructions.is_empty() {
        formatting_overhead += "User:\n\n".len(); // "User:\n" + "\n"
    }

    assembled_chars += formatting_overhead;

    // Trim oldest history first until within char budget (character-accurate)
    while assembled_chars > char_budget && !history_blocks.is_empty() {
        let removed = history_blocks.remove(0);
        assembled_chars = assembled_chars.saturating_sub(removed.chars().count() + "- \n".len());
    }
    // Update formatting overhead if all history was removed
    if history_blocks.is_empty() {
        assembled_chars = assembled_chars.saturating_sub("History:\n\n".len());
    }

    // If still too large, truncate instructions
    if assembled_chars > char_budget && !instructions.is_empty() {
        let fixed_overhead = system.chars().count()
            + context_block.chars().count()
            + if !system.is_empty() {
                "System:\n\n\n".len()
            } else {
                0
            }
            + if !context_block.is_empty() {
                "\n".len()
            } else {
                0
            }
            + "User:\n\n".len();
        let allowed_chars = char_budget.saturating_sub(fixed_overhead);
        instructions = instructions.chars().take(allowed_chars).collect();
        assembled_chars = system.chars().count()
            + context_block.chars().count()
            + instructions.chars().count()
            + if !system.is_empty() {
                "System:\n\n\n".len()
            } else {
                0
            }
            + if !context_block.is_empty() {
                "\n".len()
            } else {
                0
            }
            + if !instructions.is_empty() {
                "User:\n\n".len()
            } else {
                0
            };
    }

    // If still too large, truncate context_block
    if assembled_chars > char_budget && !context_block.is_empty() {
        let fixed_overhead = system.chars().count()
            + if !system.is_empty() {
                "System:\n\n\n".len()
            } else {
                0
            }
            + if !instructions.is_empty() {
                instructions.chars().count() + "User:\n\n".len()
            } else {
                0
            }
            + "\n".len(); // newline after context
        let allowed_chars = char_budget.saturating_sub(fixed_overhead);
        // Truncate context keeping the "Context:\n- " prefix
        let prefix = "Context:\n- ";
        if allowed_chars > prefix.len() {
            let content_chars = allowed_chars - prefix.len() - "\n".len();
            context_block = format!(
                "{}{}\n",
                prefix,
                bundle
                    .context_docs
                    .first()
                    .unwrap_or(&String::new())
                    .chars()
                    .take(content_chars)
                    .collect::<String>()
            );
        } else {
            context_block.clear();
        }
        assembled_chars = system.chars().count()
            + context_block.chars().count()
            + instructions.chars().count()
            + if !system.is_empty() {
                "System:\n\n\n".len()
            } else {
                0
            }
            + if !context_block.is_empty() {
                "\n".len()
            } else {
                0
            }
            + if !instructions.is_empty() {
                "User:\n\n".len()
            } else {
                0
            };
    }

    // If still too large, truncate system (last resort)
    if assembled_chars > char_budget && !system.is_empty() {
        let fixed_overhead = context_block.chars().count()
            + instructions.chars().count()
            + "System:\n\n\n".len()
            + if !context_block.is_empty() {
                "\n".len()
            } else {
                0
            }
            + if !instructions.is_empty() {
                "User:\n\n".len()
            } else {
                0
            };
        let allowed_chars = char_budget.saturating_sub(fixed_overhead);
        system = system.chars().take(allowed_chars).collect();
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
