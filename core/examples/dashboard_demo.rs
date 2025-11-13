// Dashboard demonstration example
//
// Shows how to run Loom Core with Dashboard enabled

use loom_core::{
    dashboard::{DashboardConfig, DashboardServer, EventBroadcaster, FlowTracker},
    directory::AgentDirectory,
    event::{Event, EventBus, EventExt},
};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize telemetry
    loom_core::telemetry::init_telemetry()?;

    info!("Starting Loom with Dashboard...");

    // Create core components
    let mut event_bus = EventBus::new().await?;
    let agent_directory = Arc::new(AgentDirectory::new());

    // Create dashboard broadcaster and flow tracker
    let broadcaster = EventBroadcaster::new(1000);
    let flow_tracker = Arc::new(FlowTracker::new());

    // Connect EventBus to Dashboard
    event_bus.set_dashboard_broadcaster(broadcaster.clone());

    let event_bus = Arc::new(event_bus);

    // Start Dashboard server
    let config = DashboardConfig::from_env();
    let dashboard = DashboardServer::new(config.clone(), broadcaster, agent_directory.clone())
        .with_flow_tracker(flow_tracker.clone());

    info!(
        "Dashboard will be available at http://{}:{}",
        config.host, config.port
    );

    // Spawn dashboard server
    let dashboard_handle = tokio::spawn(async move {
        if let Err(e) = dashboard.serve().await {
            eprintln!("Dashboard error: {}", e);
        }
    });

    // Register some example agents
    let planner_info = loom_core::directory::AgentInfo {
        agent_id: "planner".to_string(),
        subscribed_topics: vec!["agent.task".to_string(), "thread.*.broadcast".to_string()],
        capabilities: vec!["plan.create".to_string()],
        metadata: Default::default(),
    };

    agent_directory.register_agent(planner_info);

    agent_directory.register_agent(loom_core::directory::AgentInfo {
        agent_id: "researcher".to_string(),
        subscribed_topics: vec!["agent.research".to_string()],
        capabilities: vec!["web.search".to_string()],
        metadata: Default::default(),
    });

    agent_directory.register_agent(loom_core::directory::AgentInfo {
        agent_id: "writer".to_string(),
        subscribed_topics: vec!["agent.write".to_string()],
        capabilities: vec!["content.generate".to_string()],
        metadata: Default::default(),
    });

    info!("Registered 3 agents");

    // Simulate event flow between components
    let thread_id = format!(
        "thread-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );

    info!("Starting event flow simulation...");

    // Simulate continuous event flow
    tokio::spawn({
        let flow_tracker = flow_tracker.clone();
        let event_bus = event_bus.clone();
        async move {
            let agents = vec!["planner", "researcher", "writer"];
            let topics = vec!["agent.task", "agent.research", "agent.write"];

            for i in 0.. {
                sleep(Duration::from_millis(1500)).await;

                let agent_idx = i % agents.len();
                let agent = agents[agent_idx];
                let topic = topics[agent_idx];

                // Record flow: EventBus -> Agent
                flow_tracker.record_flow("EventBus", agent, topic).await;

                // Also record reverse flow: Agent -> EventBus (agent publishing)
                if i % 3 == 0 {
                    flow_tracker.record_flow(agent, "EventBus", topic).await;
                }

                // Occasionally show Router and LLM interaction
                if i % 5 == 0 {
                    flow_tracker
                        .record_flow("Router", "llm-provider", "llm.request")
                        .await;
                    flow_tracker
                        .record_flow("llm-provider", "Router", "llm.response")
                        .await;
                }

                let event = Event {
                    id: format!("event-{}-{}", thread_id, i),
                    r#type: "task.created".to_string(),
                    timestamp_ms: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as i64,
                    source: agent.to_string(),
                    payload: format!("Task {} from {}", i, agent).into_bytes(),
                    metadata: Default::default(),
                    confidence: 1.0,
                    tags: vec![],
                    priority: 50,
                }
                .with_thread(thread_id.clone())
                .with_sender(agent.to_string())
                .with_correlation(format!("corr-{}", i));

                let _ = event_bus.publish(topic, event).await;

                if i < 20 {
                    info!("Published event {} from {}", i, agent);
                }
            }
        }
    });

    info!("Demo complete. Dashboard server will continue running...");
    info!("Open http://{}:{} to view events", config.host, config.port);

    // Keep dashboard running
    dashboard_handle.await?;

    Ok(())
}
