# Directories (Agent & Capability)

The directory components provide a lightweight, in-memory catalog for multi-agent coordination without coupling core modules tightly.

## AgentDirectory

Tracks agents with their subscriptions and capabilities for quick lookup.

- AgentInfo: { agent_id, subscribed_topics: Vec<String>, capabilities: Vec<String>, metadata: HashMap<String,String> }
- register_agent(info)
- unregister_agent(agent_id)
- get(agent_id) -> Option<AgentInfo>
- by_topic(topic) -> Vec<agent_id>
- by_capability(capability) -> Vec<agent_id>
- all() -> Vec<AgentInfo>

Indexing is maintained for topics and capabilities for efficient reverse lookups.

## CapabilityDirectory

Maintains a snapshot of capabilities available via `ActionBroker`.

- refresh_from_broker(&ActionBroker)
- list() -> Vec<CapabilityDescriptor>
- find_by_name(name) -> Vec<CapabilityDescriptor>
- get(name, version) -> Option<CapabilityDescriptor>

This avoids adding a hard dependency from `ActionBroker` back to `EventBus`.

## Integration Notes

- Runtime owners can update `AgentDirectory` when creating/deleting agents.
- For dynamic capability changes, call `refresh_from_broker` when needed (e.g., at startup or on periodic cadence).

## Examples

See `core/tests/directory_test.rs` for indexing and snapshot usage.
