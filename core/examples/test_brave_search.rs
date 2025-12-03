//! Test Brave Search API connectivity
//!
//! Run with: cargo run --example test_brave_search --release

use std::error::Error;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = "BSAr_AtvLx7KPT7UyNY_FgTtcX-Zmrk";
    let query = "Bitcoin price";

    println!("=== Brave Search API Test ===\n");
    println!("API Key: {}...", &api_key[..15]);
    println!("Query: {}", query);

    // Build client similar to our WebSearchTool
    // Use system proxy (127.0.0.1:7897)
    let proxy = reqwest::Proxy::all("http://127.0.0.1:7897")?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .proxy(proxy)
        .build()?;

    let url = format!(
        "https://api.search.brave.com/res/v1/web/search?q={}&count=3",
        urlencoding::encode(query)
    );

    println!("URL: {}\n", url);
    println!("Sending request...");

    let start = std::time::Instant::now();

    let result = client
        .get(&url)
        .header("Accept", "application/json")
        .header("Accept-Encoding", "gzip")
        .header("X-Subscription-Token", api_key)
        .send()
        .await;

    let elapsed = start.elapsed();
    println!("Request took: {:?}\n", elapsed);

    match result {
        Ok(resp) => {
            println!("✅ Response received!");
            println!("Status: {}", resp.status());
            println!("Headers: {:?}", resp.headers());

            let body = resp.text().await?;
            println!("\nBody (first 1000 chars):");
            println!("{}", &body[..body.len().min(1000)]);
        }
        Err(e) => {
            println!("❌ Request failed!");
            println!("Error: {}", e);
            println!("Is timeout: {}", e.is_timeout());
            println!("Is connect: {}", e.is_connect());
            println!("Is request: {}", e.is_request());

            if let Some(source) = e.source() {
                println!("Source: {}", source);
            }
        }
    }

    Ok(())
}
