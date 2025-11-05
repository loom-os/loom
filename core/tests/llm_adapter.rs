use loom_core::context::{PromptBundle, TokenBudget};
use loom_core::llm::promptbundle_to_messages_and_text;

#[test]
fn adapter_respects_char_budget_and_builds_messages() {
    let bundle = PromptBundle {
        system: "S".repeat(50),
        instructions: "I".repeat(500),
        tools_json_schema: None,
        context_docs: vec!["Doc A".into(), "Doc B".into()],
        history: vec!["H1".into(), "H2".into(), "H3".into()],
    };
    let budget = TokenBudget {
        max_input_tokens: 32, // ~128 chars budget
        max_output_tokens: 64,
    };

    let (messages, fused) = promptbundle_to_messages_and_text(&bundle, budget);

    // Should include at least one system message and one user message
    assert!(!messages.is_empty());
    assert!(messages
        .iter()
        .any(|m| m.get("role").and_then(|r| r.as_str()) == Some("system")));
    assert!(messages
        .iter()
        .any(|m| m.get("role").and_then(|r| r.as_str()) == Some("user")));

    // Fused text should contain the System and Context headers when present
    assert!(fused.contains("System:"));
    assert!(fused.contains("Context:"));

    // Ensure we did not grossly exceed the rough char budget (+ some header overhead)
    assert!(
        fused.chars().count() <= 128 + 64,
        "fused too large: {} chars",
        fused.chars().count()
    );
}
