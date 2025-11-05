use loom_core::llm::promptbundle_to_messages_and_text;
use loom_core::{PromptBundle, TokenBudget};

#[test]
fn adapter_truncates_history_under_budget() {
    let bundle = PromptBundle {
        system: "sys".into(),
        instructions: "do it".into(),
        tools_json_schema: None,
        context_docs: vec!["doc1".into()],
        history: vec!["a".repeat(2000), "b".repeat(2000), "c".repeat(10)],
    };
    let budget = TokenBudget {
        max_input_tokens: 512,
        max_output_tokens: 32,
    };
    let (_messages, fused) = promptbundle_to_messages_and_text(&bundle, budget);
    // Expect that fused is under char budget (~2048 chars) and contains 'c' but likely not both 'a' and 'b'
    assert!(fused.len() <= budget.max_input_tokens * 4 + 64); // allow small overhead
    assert!(fused.contains(&"c".repeat(10)));
}
