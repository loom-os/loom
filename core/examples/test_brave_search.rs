//! Test Brave Search API connectivity
//!
//! Run with: cargo run --example test_brave_search --release

use std::error::Error;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let query = "Bitcoin price";

    // Read API key from environment to avoid embedding secrets in the repo
    let api_key = match std::env::var("BRAVE_API_KEY") {
        Ok(k) => k,
        Err(_) => {
            eprintln!("ERROR: BRAVE_API_KEY not set. Please set BRAVE_API_KEY in your environment or .env file.");
            std::process::exit(1);
        }
    };

    // Mask API key when logging to avoid leaking secrets
    let masked_key = if api_key.len() > 8 {
        format!("{}...{}", &api_key[..4], &api_key[api_key.len() - 4..])
    } else {
        "****".to_string()
    };

    println!("=== Brave Search API Test ===\n");
    println!("API Key: {}", masked_key);
    println!("Query: {}", query);

    // Build client similar to our WebSearchTool
    // Proxy is optional - configure via HTTPS_PROXY environment variable
    let mut client_builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36");

    if let Ok(proxy_url) = std::env::var("HTTPS_PROXY")
        .or_else(|_| std::env::var("HTTP_PROXY"))
        .or_else(|_| std::env::var("ALL_PROXY"))
    {
        println!("Using proxy: {}", proxy_url);
        client_builder = client_builder.proxy(reqwest::Proxy::all(&proxy_url)?);
    } else {
        println!("No proxy configured (set HTTPS_PROXY if needed)");
    }

    let client = client_builder.build()?;

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
