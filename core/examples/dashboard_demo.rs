// Dashboard demonstration example
//
// Shows how to run Loom Core with Dashboard enabled

use loom_core::{
    dashboard::{DashboardConfig, DashboardServer, EventBroadcaster, FlowTracker},
    directory::AgentDirectory,
    event::{Event, EventBus, EventExt, QoSLevel},
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

    // Wire real subscriptions to demonstrate end-to-end consumption
    // planner: receives tasks -> publishes research request
    {
        let event_bus = event_bus.clone();
        let flow_tracker = flow_tracker.clone();
        let (sub_id, mut rx) = event_bus
            .subscribe("agent.task".to_string(), vec![], QoSLevel::QosRealtime)
            .await?;
        info!("planner subscribed: {}", sub_id);
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                flow_tracker
                    .record_flow("EventBus", "planner", "agent.task")
                    .await;
                // Planner emits research request
                let out = Event {
                    id: format!("{}-plan", event.id),
                    r#type: "plan.created".to_string(),
                    timestamp_ms: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as i64,
                    source: "planner".to_string(),
                    payload: format!("Plan for {}", String::from_utf8_lossy(&event.payload))
                        .into_bytes(),
                    metadata: Default::default(),
                    confidence: 1.0,
                    tags: vec![],
                    priority: 50,
                }
                .with_thread(event.thread_id().unwrap_or("thread").to_string())
                .with_sender("planner".to_string())
                .with_correlation(event.correlation_id().unwrap_or_default().to_string());
                flow_tracker
                    .record_flow("planner", "EventBus", "agent.research")
                    .await;
                let _ = event_bus.publish("agent.research", out).await;
            }
        });
    }

    // researcher: receives research req -> publishes draft to writer
    {
        let event_bus = event_bus.clone();
        let flow_tracker = flow_tracker.clone();
        let (sub_id, mut rx) = event_bus
            .subscribe("agent.research".to_string(), vec![], QoSLevel::QosRealtime)
            .await?;
        info!("researcher subscribed: {}", sub_id);
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                flow_tracker
                    .record_flow("EventBus", "researcher", "agent.research")
                    .await;
                let out = Event {
                    id: format!("{}-research", event.id),
                    r#type: "research.completed".to_string(),
                    timestamp_ms: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as i64,
                    source: "researcher".to_string(),
                    payload: b"Findings: ...".to_vec(),
                    metadata: Default::default(),
                    confidence: 1.0,
                    tags: vec![],
                    priority: 50,
                }
                .with_thread(event.thread_id().unwrap_or("thread").to_string())
                .with_sender("researcher".to_string())
                .with_correlation(event.correlation_id().unwrap_or_default().to_string());
                flow_tracker
                    .record_flow("researcher", "EventBus", "agent.write")
                    .await;
                let _ = event_bus.publish("agent.write", out).await;
            }
        });
    }

    // writer: receives draft -> publishes final to thread.broadcast
    {
        let event_bus = event_bus.clone();
        let flow_tracker = flow_tracker.clone();
        let (sub_id, mut rx) = event_bus
            .subscribe("agent.write".to_string(), vec![], QoSLevel::QosRealtime)
            .await?;
        info!("writer subscribed: {}", sub_id);
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                flow_tracker
                    .record_flow("EventBus", "writer", "agent.write")
                    .await;
                let out = Event {
                    id: format!("{}-final", event.id),
                    r#type: "content.finalized".to_string(),
                    timestamp_ms: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as i64,
                    source: "writer".to_string(),
                    payload: b"Article: ...".to_vec(),
                    metadata: Default::default(),
                    confidence: 1.0,
                    tags: vec![],
                    priority: 50,
                }
                .with_thread(event.thread_id().unwrap_or("thread").to_string())
                .with_sender("writer".to_string())
                .with_correlation(event.correlation_id().unwrap_or_default().to_string());
                flow_tracker
                    .record_flow("writer", "EventBus", "thread.broadcast")
                    .await;
                let _ = event_bus.publish("thread.broadcast", out).await;
            }
        });
    }

    // planner listens for final broadcast (close the loop)
    {
        let flow_tracker = flow_tracker.clone();
        let event_bus = event_bus.clone();
        let (sub_id, mut rx) = event_bus
            .subscribe(
                "thread.broadcast".to_string(),
                vec![],
                QoSLevel::QosRealtime,
            )
            .await?;
        info!("planner broadcast subscriber: {}", sub_id);
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                flow_tracker
                    .record_flow("EventBus", "planner", "thread.broadcast")
                    .await;
                let _ = event; // end-of-pipeline
            }
        });
    }

    // Simulate continuous event flow: seed pipeline by publishing to agent.task
    tokio::spawn({
        let flow_tracker = flow_tracker.clone();
        let event_bus = event_bus.clone();
        async move {
            for i in 0.. {
                sleep(Duration::from_millis(1500)).await;
                let event = Event {
                    id: format!("event-{}-{}", thread_id, i),
                    r#type: "task.created".to_string(),
                    timestamp_ms: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as i64,
                    source: "planner".to_string(),
                    payload: format!("Task {}", i).into_bytes(),
                    metadata: Default::default(),
                    confidence: 1.0,
                    tags: vec![],
                    priority: 50,
                }
                .with_thread(thread_id.clone())
                .with_sender("planner".to_string())
                .with_correlation(format!("corr-{}", i));

                // Planner seeds the pipeline with a task
                flow_tracker
                    .record_flow("planner", "EventBus", "agent.task")
                    .await;
                let _ = event_bus.publish("agent.task", event).await;

                if i < 20 {
                    info!("Published task {}", i);
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
