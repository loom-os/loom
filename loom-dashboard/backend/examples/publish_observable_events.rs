//! Example: Publishing Events for Dashboard Observability
//!
//! This example demonstrates how to publish events that will be
//! visible in the Dashboard's Observability page.
//!
//! Run with: cargo run --example publish_observable_events

use loom_core::messaging::event_bus::EventBus;
use loom_core::proto::Event;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🚀 Starting observable events publisher");
    println!("📊 Start the Dashboard to see these events in real-time:");
    println!("   cd loom-dashboard/backend && cargo run");
    println!("   Open: http://localhost:3030");
    println!();

    // Create EventBus
    let event_bus = Arc::new(EventBus::new().await?);
    event_bus.start().await?;

    println!("✅ EventBus started");
    println!("📡 Publishing events to observable topics...\n");

    // Simulate different types of events
    for i in 0..20 {
        // Agent activity events
        publish_agent_event(&event_bus, "agent.planner", i).await?;
        sleep(Duration::from_millis(500)).await;

        publish_agent_event(&event_bus, "agent.researcher", i).await?;
        sleep(Duration::from_millis(500)).await;

        // Stream events
        publish_stream_event(&event_bus, "stream.chunk", i).await?;
        sleep(Duration::from_millis(300)).await;

        // Tool events
        publish_tool_event(&event_bus, "tool.web_search", i).await?;
        sleep(Duration::from_millis(700)).await;

        // Dashboard events
        publish_dashboard_event(&event_bus, i).await?;
        sleep(Duration::from_millis(1000)).await;

        println!("📨 Published batch #{}", i + 1);
    }

    println!("\n✅ All events published!");
    println!("💡 Check the Dashboard Observability page to see the events");

    // Keep running for a bit
    sleep(Duration::from_secs(5)).await;

    Ok(())
}

async fn publish_agent_event(
    event_bus: &EventBus,
    topic: &str,
    iteration: i32,
) -> anyhow::Result<()> {
    let agent_id = topic.split('.').last().unwrap_or("unknown");

    let mut metadata = HashMap::new();
    metadata.insert("agent_id".to_string(), agent_id.to_string());
    metadata.insert("iteration".to_string(), iteration.to_string());

    let event = Event {
        id: uuid::Uuid::new_v4().to_string(),
        r#type: "agent.activity".to_string(),
        source: topic.to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        metadata,
        payload: format!("{} is processing task #{}", agent_id, iteration).into_bytes(),
        ..Default::default()
    };

    event_bus.publish(topic, event).await?;
    Ok(())
}

async fn publish_stream_event(
    event_bus: &EventBus,
    topic: &str,
    iteration: i32,
) -> anyhow::Result<()> {
    let mut metadata = HashMap::new();
    metadata.insert("thread_id".to_string(), "demo-thread-123".to_string());
    metadata.insert("chunk_index".to_string(), iteration.to_string());

    let event = Event {
        id: uuid::Uuid::new_v4().to_string(),
        r#type: "stream.data".to_string(),
        source: topic.to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        metadata,
        payload: format!("Streaming chunk {} of response...", iteration).into_bytes(),
        ..Default::default()
    };

    event_bus.publish(topic, event).await?;
    Ok(())
}

async fn publish_tool_event(
    event_bus: &EventBus,
    topic: &str,
    iteration: i32,
) -> anyhow::Result<()> {
    let tool_name = topic.split('.').last().unwrap_or("unknown");

    let mut metadata = HashMap::new();
    metadata.insert("tool_name".to_string(), tool_name.to_string());
    metadata.insert("call_id".to_string(), format!("call-{}", iteration));

    let event = Event {
        id: uuid::Uuid::new_v4().to_string(),
        r#type: "tool.execute".to_string(),
        source: topic.to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        metadata,
        payload: format!("Executing {} with query={}", tool_name, iteration).into_bytes(),
        ..Default::default()
    };

    event_bus.publish(topic, event).await?;
    Ok(())
}

async fn publish_dashboard_event(event_bus: &EventBus, iteration: i32) -> anyhow::Result<()> {
    let mut metadata = HashMap::new();
    metadata.insert("source".to_string(), "demo".to_string());
    metadata.insert("iteration".to_string(), iteration.to_string());

    let event = Event {
        id: uuid::Uuid::new_v4().to_string(),
        r#type: "dashboard.update".to_string(),
        source: "dashboard".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        metadata,
        payload: format!("Dashboard update #{}", iteration).into_bytes(),
        ..Default::default()
    };

    event_bus.publish("dashboard", event).await?;
    Ok(())
}
