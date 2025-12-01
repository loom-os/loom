//! Plugin system implementation
//!
//! **DEPRECATED**: Use `tools::Tool` trait and `tools::ToolRegistry` instead.
//!
//! The unified tool system provides:
//! - `tools::Tool` - Standard tool trait with describe/validate/execute
//! - `tools::ToolRegistry` - Dynamic tool registration and discovery
//! - `tools::mcp::McpManager` - External tool protocol support
//!
//! Migration guide:
//! - `Plugin::handle_event()` → `Tool::execute()`
//! - `PluginManager::register_plugin()` → `ToolRegistry::register()`
//! - `PluginManager::call_plugin()` → `ToolRegistry::invoke()`

#![allow(deprecated)] // Allow deprecated usage within this module

use crate::{Event, LoomError, Result};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use tracing::{info, warn};

pub use crate::proto::{PluginMeta, PluginResponse, PluginType};

/// Plugin trait
///
/// **Deprecated**: Use `tools::Tool` instead.
#[deprecated(since = "0.2.0", note = "Use tools::Tool trait instead")]
#[async_trait]
pub trait Plugin: Send + Sync {
    async fn init(&mut self, meta: PluginMeta) -> Result<()>;
    async fn handle_event(&mut self, event: Event) -> Result<PluginResponse>;
    async fn health(&self) -> Result<bool>;
    async fn shutdown(&mut self) -> Result<()>;
}

/// Plugin Manager
pub struct PluginManager {
    plugins: Arc<DashMap<String, Box<dyn Plugin>>>,
}

impl PluginManager {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            plugins: Arc::new(DashMap::new()),
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Plugin Manager started");
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Plugin Manager shutting down");

        for mut entry in self.plugins.iter_mut() {
            if let Err(e) = entry.value_mut().shutdown().await {
                warn!("Error shutting down plugin {}: {}", entry.key(), e);
            }
        }
        self.plugins.clear();

        Ok(())
    }

    /// 注册插件
    pub async fn register_plugin(
        &self,
        plugin_id: String,
        mut plugin: Box<dyn Plugin>,
        meta: PluginMeta,
    ) -> Result<()> {
        plugin.init(meta).await?;
        self.plugins.insert(plugin_id.clone(), plugin);
        info!("Registered plugin: {}", plugin_id);
        Ok(())
    }

    /// Call plugin to handle event
    pub async fn call_plugin(&self, plugin_id: &str, event: Event) -> Result<PluginResponse> {
        if let Some(mut plugin) = self.plugins.get_mut(plugin_id) {
            plugin.value_mut().handle_event(event).await
        } else {
            Err(LoomError::PluginError(format!(
                "Plugin {} not found",
                plugin_id
            )))
        }
    }

    /// Check plugin health status
    pub async fn check_health(&self, plugin_id: &str) -> Result<bool> {
        if let Some(plugin) = self.plugins.get(plugin_id) {
            plugin.value().health().await
        } else {
            Err(LoomError::PluginError(format!(
                "Plugin {} not found",
                plugin_id
            )))
        }
    }
}
