#[cfg(feature = "mic")]
mod run_demo {
    use loom_core::audio::{MicConfig, MicSource};
    use loom_core::event::EventBus;
    use loom_core::proto::QoSLevel;
    use loom_core::Result;
    use std::sync::Arc;
    use tokio::signal;
    use tokio::time::Duration;
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

        // Receive for a configurable period; MIC_DEMO_SECONDS=0 (or "inf") means run until Ctrl-C
        let mut received = 0usize;
        let demo_secs = std::env::var("MIC_DEMO_SECONDS")
            .ok()
            .and_then(|s| {
                if s.to_lowercase() == "inf" {
                    Some(0u64)
                } else {
                    s.parse::<u64>().ok()
                }
            })
            .unwrap_or(5);

        if demo_secs == 0 {
            info!("mic_capture running until Ctrl-C (MIC_DEMO_SECONDS=0)");
            loop {
                tokio::select! {
                    _ = signal::ctrl_c() => {
                        info!("Ctrl-C received, exiting mic_capture");
                        break;
                    }
                    maybe = rx.recv() => {
                        match maybe {
                            Some(ev) => {
                                received += 1;
                                if received % 10 == 0 {
                                    let rate = ev.metadata.get("sample_rate").cloned().unwrap_or_default();
                                    let ch = ev.metadata.get("channels").cloned().unwrap_or_default();
                                    info!("received {} audio_chunk(s), rate={} ch={}", received, rate, ch);
                                }
                            }
                            None => {
                                warn!("subscription channel closed");
                                break;
                            }
                        }
                    }
                }
            }
        } else {
            let start = tokio::time::Instant::now();
            let duration = Duration::from_secs(demo_secs);
            loop {
                if tokio::time::Instant::now().duration_since(start) >= duration {
                    break;
                }
                match rx.recv().await {
                    Some(ev) => {
                        received += 1;
                        if received % 10 == 0 {
                            let rate = ev.metadata.get("sample_rate").cloned().unwrap_or_default();
                            let ch = ev.metadata.get("channels").cloned().unwrap_or_default();
                            info!(
                                "received {} audio_chunk(s), rate={} ch={}",
                                received, rate, ch
                            );
                        }
                    }
                    None => {
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
