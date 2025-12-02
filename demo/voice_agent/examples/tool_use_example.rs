/// Standalone Tool Use Example
///
/// This example demonstrates how to use the Tool Orchestrator with web.search and weather.get
/// capabilities using the ToolRegistry API.
///
/// Run with: cargo run --example tool_use_example
use loom_core::cognitive::llm::orchestrator::{OrchestratorOptions, ToolChoice, ToolOrchestrator};
use loom_core::context::{PromptBundle, TokenBudget};
use loom_core::{LlmClient, LlmClientConfig, Result, ToolRegistry, WeatherTool, WebSearchTool};
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,tool_orch=debug,web_search=debug,weather=debug")
        .with_target(true)
        .init();

    info!("🚀 Tool Use Example - Web Search & Weather");

    // 1. Create ToolRegistry and register tools
    let registry = Arc::new(ToolRegistry::new());

    info!("📦 Registering web.search tool");
    registry.register(Arc::new(WebSearchTool::new())).await;

    info!("📦 Registering weather.get tool");
    registry.register(Arc::new(WeatherTool::new())).await;

    // Verify tools are registered
    let tools = registry.list_tools();
    info!("✅ Registered {} tools", tools.len());
    for tool in &tools {
        info!("   - {}", tool.name());
    }

    // 2. Create LLM client (requires OpenAI-compatible endpoint)
    let llm_config = LlmClientConfig {
        base_url: std::env::var("LLM_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:11434/v1".to_string()),
        api_key: std::env::var("LLM_API_KEY").ok(),
        model: std::env::var("LLM_MODEL").unwrap_or_else(|_| "qwen2.5:latest".to_string()),
        temperature: 0.7,
        request_timeout_ms: 60_000,
    };

    let llm_client = Arc::new(LlmClient::new(llm_config)?);
    info!("🤖 LLM client configured");

    // 3. Create Tool Orchestrator with registry
    let mut orchestrator = ToolOrchestrator::new(llm_client, Arc::clone(&registry));

    // 4. Example queries
    let examples = vec![
        "What's the weather like in Tokyo?",
        "Search for information about Rust programming language",
        "What's the weather in London and search for tourist attractions there",
    ];

    for (i, query) in examples.iter().enumerate() {
        println!("\n{}", "=".repeat(80));
        info!("Query #{}: {}", i + 1, query);
        println!("{}\n", "=".repeat(80));

        let bundle = PromptBundle {
            system:
                "You are a helpful assistant with access to web search and weather information. \
                 Use these tools when needed to provide accurate, up-to-date answers."
                    .to_string(),
            instructions: query.to_string(),
            tools_json_schema: None,
            context_docs: vec![],
            history: vec![],
        };

        let budget = TokenBudget {
            max_input_tokens: 2048,
            max_output_tokens: 512,
        };

        let options = OrchestratorOptions {
            tool_choice: ToolChoice::Auto,
            per_tool_timeout_ms: 30_000,
            refine_on_tool_result: true,
            max_tools_exposed: 64,
        };

        match orchestrator
            .run(
                &bundle,
                Some(budget),
                options,
                Some(format!("example_{}", i)),
            )
            .await
        {
            Ok(answer) => {
                info!("✨ Answer: {}", answer.text);

                if !answer.tool_calls.is_empty() {
                    info!("🔧 Tools used ({}):", answer.tool_calls.len());
                    for (idx, call) in answer.tool_calls.iter().enumerate() {
                        info!(
                            "   {}. {} with args: {}",
                            idx + 1,
                            call.name,
                            serde_json::to_string(&call.arguments).unwrap_or_default()
                        );
                    }
                }

                if !answer.tool_results.is_empty() {
                    info!("📊 Tool results ({}):", answer.tool_results.len());
                    for (idx, result) in answer.tool_results.iter().enumerate() {
                        info!("   {}. {}", idx + 1, result);
                    }
                }
            }
            Err(e) => {
                info!("❌ Error: {}", e);
            }
        }
    }

    // 5. Display statistics
    println!("\n{}", "=".repeat(80));
    info!("📈 Orchestrator Statistics");
    println!("{}", "=".repeat(80));
    info!(
        "Total invocations: {}",
        orchestrator.stats.total_invocations
    );
    info!("Total tool calls: {}", orchestrator.stats.total_tool_calls);
    info!("Tool errors: {}", orchestrator.stats.total_tool_errors);
    info!(
        "Average tool latency: {:.2}ms",
        orchestrator.stats.avg_tool_latency_ms
    );

    Ok(())
}
