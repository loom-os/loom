use crate::tools::{Tool, ToolError, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use tracing::{debug, warn};

/// Search result item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: Option<String>,
}

/// Brave Search API response structures
#[derive(Debug, Deserialize)]
struct BraveSearchResponse {
    web: Option<BraveWebResults>,
}

#[derive(Debug, Deserialize)]
struct BraveWebResults {
    results: Vec<BraveWebResult>,
}

#[derive(Debug, Deserialize)]
struct BraveWebResult {
    title: String,
    url: String,
    description: Option<String>,
}

/// Web search tool using Brave Search API
pub struct WebSearchTool {
    api_key: Option<String>,
    http_client: reqwest::Client,
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl WebSearchTool {
    /// Create a new web search tool, reading API key from environment
    pub fn new() -> Self {
        let api_key = std::env::var("BRAVE_API_KEY").ok();

        if api_key.is_some() {
            tracing::info!(target: "web_search", "Brave Search API key configured");
        } else {
            warn!(target: "web_search", "BRAVE_API_KEY not set, web search will not work");
        }

        // Build HTTP client with optional proxy support
        let mut client_builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36");

        // Check for proxy settings (HTTP_PROXY, HTTPS_PROXY, or ALL_PROXY)
        if let Ok(proxy_url) = std::env::var("HTTPS_PROXY")
            .or_else(|_| std::env::var("HTTP_PROXY"))
            .or_else(|_| std::env::var("ALL_PROXY"))
            .or_else(|_| std::env::var("https_proxy"))
            .or_else(|_| std::env::var("http_proxy"))
            .or_else(|_| std::env::var("all_proxy"))
        {
            tracing::info!(target: "web_search", proxy = %proxy_url, "Using proxy for web search");
            if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
                client_builder = client_builder.proxy(proxy);
            }
        }

        let http_client = client_builder
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            api_key,
            http_client,
        }
    }

    /// Create with explicit API key
    pub fn with_api_key(api_key: String) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_millis(15_000))
            .user_agent("loom-agent/0.1")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            api_key: Some(api_key),
            http_client,
        }
    }

    /// Perform search using Brave Search API
    async fn search_brave(&self, query: &str, count: usize) -> ToolResult<Vec<SearchResult>> {
        let api_key = self.api_key.as_ref().ok_or_else(|| {
            ToolError::ExecutionFailed(
                "BRAVE_API_KEY not configured. Set it in environment or loom.toml".to_string(),
            )
        })?;

        debug!(target: "web_search", query=%query, count=%count, "Performing Brave search");

        let url = format!(
            "https://api.search.brave.com/res/v1/web/search?q={}&count={}",
            urlencoding::encode(query),
            count
        );

        let resp = self
            .http_client
            .get(&url)
            .header("Accept", "application/json")
            .header("Accept-Encoding", "gzip")
            .header("X-Subscription-Token", api_key)
            .send()
            .await
            .map_err(|e| {
                tracing::error!(target: "web_search", error = %e, "Request failed");
                if e.is_timeout() {
                    ToolError::ExecutionFailed(format!("Search request timed out: {}", e))
                } else if e.is_connect() {
                    ToolError::ExecutionFailed(format!("Connection failed: {}", e))
                } else {
                    ToolError::ExecutionFailed(format!(
                        "Search request failed: {} (is_request={}, is_body={}, is_decode={})",
                        e,
                        e.is_request(),
                        e.is_body(),
                        e.is_decode()
                    ))
                }
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ToolError::ExecutionFailed(format!(
                "Brave Search API error: {} - {}",
                status, body
            )));
        }

        let data: BraveSearchResponse = resp.json().await.map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to parse search response: {}", e))
        })?;

        let results: Vec<SearchResult> = data
            .web
            .map(|web| {
                web.results
                    .into_iter()
                    .map(|r| SearchResult {
                        title: r.title,
                        url: r.url,
                        snippet: r.description,
                    })
                    .collect()
            })
            .unwrap_or_default();

        debug!(target: "web_search", result_count=%results.len(), "Search completed");

        Ok(results)
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> String {
        "web:search".to_string()
    }

    fn description(&self) -> String {
        "Search the web for information using Brave Search".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results (default: 5, max: 20)"
                }
            },
            "required": ["query"]
        })
    }

    async fn call(&self, arguments: Value) -> ToolResult<Value> {
        let query = arguments["query"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'query'".to_string()))?;

        let limit = arguments["limit"].as_u64().unwrap_or(5).min(20) as usize;

        let results = self.search_brave(query, limit).await?;

        Ok(json!({
            "query": query,
            "results": results,
            "count": results.len()
        }))
    }
}
