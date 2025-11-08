use async_trait::async_trait;
use loom_core::action_broker::{ActionBroker, CapabilityProvider};
use loom_core::proto::{
    ActionCall, ActionResult, ActionStatus, CapabilityDescriptor, ProviderKind, QoSLevel,
};
use loom_core::{LoomError, Result};
use std::sync::Arc;
use tokio::sync::Mutex;

// Helper to create a test ActionCall
fn make_call(id: &str, capability: &str, version: &str, payload: Vec<u8>) -> ActionCall {
    ActionCall {
        id: id.to_string(),
        capability: capability.to_string(),
        version: version.to_string(),
        payload,
        headers: Default::default(),
        timeout_ms: 5000,
        correlation_id: String::new(),
        qos: QoSLevel::QosRealtime as i32,
    }
}

// Mock provider that echoes the payload
struct EchoProvider {
    name: String,
    version: String,
}

#[async_trait]
impl CapabilityProvider for EchoProvider {
    fn descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor {
            name: self.name.clone(),
            version: self.version.clone(),
            provider: ProviderKind::ProviderNative as i32,
            metadata: Default::default(),
        }
    }

    async fn invoke(&self, call: ActionCall) -> Result<ActionResult> {
        Ok(ActionResult {
            id: call.id,
            status: ActionStatus::ActionOk as i32,
            output: call.payload,
            error: None,
        })
    }
}

// Mock provider that always errors
struct ErrorProvider;

#[async_trait]
impl CapabilityProvider for ErrorProvider {
    fn descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor {
            name: "test.error".to_string(),
            version: "1.0.0".to_string(),
            provider: ProviderKind::ProviderNative as i32,
            metadata: Default::default(),
        }
    }

    async fn invoke(&self, _call: ActionCall) -> Result<ActionResult> {
        Err(LoomError::PluginError("intentional error".to_string()))
    }
}

// Mock provider that delays
struct SlowProvider {
    delay_ms: u64,
}

#[async_trait]
impl CapabilityProvider for SlowProvider {
    fn descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor {
            name: "test.slow".to_string(),
            version: "1.0.0".to_string(),
            provider: ProviderKind::ProviderNative as i32,
            metadata: Default::default(),
        }
    }

    async fn invoke(&self, call: ActionCall) -> Result<ActionResult> {
        tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
        Ok(ActionResult {
            id: call.id,
            status: ActionStatus::ActionOk as i32,
            output: vec![],
            error: None,
        })
    }
}

// Mock provider with invocation counter
struct CountingProvider {
    count: Arc<Mutex<usize>>,
}

#[async_trait]
impl CapabilityProvider for CountingProvider {
    fn descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor {
            name: "test.count".to_string(),
            version: "1.0.0".to_string(),
            provider: ProviderKind::ProviderNative as i32,
            metadata: Default::default(),
        }
    }

    async fn invoke(&self, call: ActionCall) -> Result<ActionResult> {
        let mut c = self.count.lock().await;
        *c += 1;
        Ok(ActionResult {
            id: call.id,
            status: ActionStatus::ActionOk as i32,
            output: vec![],
            error: None,
        })
    }
}

#[tokio::test]
async fn register_and_invoke_capability() -> Result<()> {
    let broker = ActionBroker::new();
    broker.register_provider(Arc::new(EchoProvider {
        name: "test.echo".to_string(),
        version: "1.0.0".to_string(),
    }));

    let call = make_call("call1", "test.echo", "1.0.0", b"hello".to_vec());

    let result = broker.invoke(call).await?;
    assert_eq!(result.status, ActionStatus::ActionOk as i32);
    assert_eq!(result.output, b"hello");
    Ok(())
}

#[tokio::test]
async fn invoke_without_version_matches_by_name() -> Result<()> {
    let broker = ActionBroker::new();
    broker.register_provider(Arc::new(EchoProvider {
        name: "test.any".to_string(),
        version: "2.0.0".to_string(),
    }));

    let call = make_call("call2", "test.any", "", b"data".to_vec());

    let result = broker.invoke(call).await?;
    assert_eq!(result.status, ActionStatus::ActionOk as i32);
    assert_eq!(result.output, b"data");
    Ok(())
}

#[tokio::test]
async fn invoke_unregistered_capability_errors() -> Result<()> {
    let broker = ActionBroker::new();
    let call = make_call("call_missing", "missing.capability", "", vec![]);

    let result = broker.invoke(call).await;
    assert!(result.is_err(), "should error on unregistered capability");
    Ok(())
}

#[tokio::test]
async fn provider_error_returns_action_error_result() -> Result<()> {
    let broker = ActionBroker::new();
    broker.register_provider(Arc::new(ErrorProvider));

    let call = make_call("call_err", "test.error", "1.0.0", vec![]);

    let result = broker.invoke(call).await?;
    assert_eq!(result.status, ActionStatus::ActionError as i32);
    assert!(result.error.is_some());
    assert!(result.error.unwrap().message.contains("intentional error"));
    Ok(())
}

#[tokio::test]
async fn timeout_returns_action_timeout_result() -> Result<()> {
    let broker = ActionBroker::new();
    broker.register_provider(Arc::new(SlowProvider { delay_ms: 1000 }));

    let mut call = make_call("call_timeout", "test.slow", "1.0.0", vec![]);
    call.timeout_ms = 100; // short timeout

    let result = broker.invoke(call).await?;
    assert_eq!(result.status, ActionStatus::ActionTimeout as i32);
    assert!(result.error.is_some());
    assert!(result.error.unwrap().code.contains("TIMEOUT"));
    Ok(())
}

#[tokio::test]
async fn list_capabilities_returns_registered_providers() -> Result<()> {
    let broker = ActionBroker::new();
    broker.register_provider(Arc::new(EchoProvider {
        name: "cap1".to_string(),
        version: "1.0.0".to_string(),
    }));
    broker.register_provider(Arc::new(EchoProvider {
        name: "cap2".to_string(),
        version: "2.0.0".to_string(),
    }));

    let caps = broker.list_capabilities();
    assert_eq!(caps.len(), 2);
    assert!(caps.iter().any(|c| c.name == "cap1"));
    assert!(caps.iter().any(|c| c.name == "cap2"));
    Ok(())
}

#[tokio::test]
async fn duplicate_registration_replaces_previous_provider() -> Result<()> {
    let broker = ActionBroker::new();
    let count1 = Arc::new(Mutex::new(0));
    let count2 = Arc::new(Mutex::new(0));

    broker.register_provider(Arc::new(CountingProvider {
        count: Arc::clone(&count1),
    }));
    // Register again with same name:version, should replace
    broker.register_provider(Arc::new(CountingProvider {
        count: Arc::clone(&count2),
    }));

    let call = make_call("call_dup", "test.count", "1.0.0", vec![]);

    broker.invoke(call).await?;

    // count2 should have been invoked, count1 should not
    assert_eq!(
        *count1.lock().await,
        0,
        "old provider should not be invoked"
    );
    assert_eq!(*count2.lock().await, 1, "new provider should be invoked");
    Ok(())
}

#[tokio::test]
async fn idempotent_call_returns_cached_result() -> Result<()> {
    let broker = ActionBroker::new();
    let count = Arc::new(Mutex::new(0));
    broker.register_provider(Arc::new(CountingProvider {
        count: Arc::clone(&count),
    }));

    let call = make_call("call_idem", "test.count", "1.0.0", vec![]);

    // Invoke twice with same ID
    let res1 = broker.invoke(call.clone()).await?;
    let res2 = broker.invoke(call.clone()).await?;

    assert_eq!(res1.id, res2.id);
    // Provider should have been invoked only once due to cache
    assert_eq!(
        *count.lock().await,
        1,
        "provider invoked once, second was cached"
    );
    Ok(())
}

#[tokio::test]
async fn zero_or_negative_timeout_defaults_to_30s() -> Result<()> {
    let broker = ActionBroker::new();
    broker.register_provider(Arc::new(EchoProvider {
        name: "test.echo".to_string(),
        version: "1.0.0".to_string(),
    }));

    let mut call = make_call("call_default_timeout", "test.echo", "1.0.0", vec![]);
    call.timeout_ms = 0; // should default to 30000ms

    // This should not timeout even with 0 timeout_ms
    let result = broker.invoke(call).await?;
    assert_eq!(result.status, ActionStatus::ActionOk as i32);
    Ok(())
}
