#[cfg(feature = "mic")]
mod run_demo {
    use loom_core::audio::{MicConfig, MicSource};
    use loom_core::event::EventBus;
    use loom_core::proto::QoSLevel;
    use loom_core::Result;
    use std::sync::Arc;
    use tokio::time::{sleep, Duration};
    use tracing::{info, warn};

    #[tokio::main]
    pub async fn main() -> Result<()> {
        tracing_subscriber::fmt::init();

        let bus = Arc::new(EventBus::new().await?);

        // Start mic capture
        let mic = MicSource::new(Arc::clone(&bus), MicConfig::default());
        let _handle = mic.start().await?;

        // Subscribe to audio chunks with realtime QoS
        let (_sub_id, mut rx) = bus
            .subscribe(
                "audio.mic".to_string(),
                vec!["audio_chunk".to_string()],
                QoSLevel::QosRealtime,
            )
            .await?;

        // Receive for a short period to demonstrate
        let mut received = 0usize;
        let until = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            tokio::select! {
                _ = sleep(Duration::from_millis(100)), if tokio::time::Instant::now() >= until => {
                    break;
                }
                maybe = rx.recv() => {
                    if let Some(ev) = maybe {
                        received += 1;
                        if received % 10 == 0 {
                            let rate = ev.metadata.get("sample_rate").cloned().unwrap_or_default();
                            let ch = ev.metadata.get("channels").cloned().unwrap_or_default();
                            info!("received {} audio_chunk(s), rate={} ch={}", received, rate, ch);
                        }
                    } else {
                        warn!("subscription channel closed");
                        break;
                    }
                }
            }
        }

        info!("mic_capture demo done; received {} chunks", received);
        Ok(())
    }
}

#[cfg(feature = "mic")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_demo::main()?;
    Ok(())
}

#[cfg(not(feature = "mic"))]
fn main() {
    eprintln!("Enable feature `mic` to run this example:\n  cargo run -p loom-core --example mic_capture --features mic");
}
