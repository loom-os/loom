# Directories (Agent & Capability)

The directory components provide lightweight, thread-safe, in-memory catalogs for multi-agent coordination and capability discovery. Both directories use `DashMap` for lock-free concurrent access.

## AgentDirectory

Thread-safe registry tracking agents with their subscriptions and capabilities for quick lookup.

### Data Model

**AgentInfo**:

- `agent_id`: String - Unique identifier
- `subscribed_topics`: Vec<String> - EventBus topics this agent subscribes to
- `capabilities`: Vec<String> - Capability names this agent provides
- `metadata`: HashMap<String,String> - Arbitrary key-value metadata (e.g., role, version, region)

### API

- `new()` - Creates empty directory
- `register_agent(info: AgentInfo)` - Registers or updates an agent; atomically updates all indices
- `unregister_agent(agent_id: &str)` - Removes agent and cleans up all indices
- `get(agent_id: &str) -> Option<AgentInfo>` - Retrieves specific agent info
- `by_topic(topic: &str) -> Vec<String>` - Finds all agent IDs subscribed to a topic
- `by_capability(capability: &str) -> Vec<String>` - Finds all agent IDs providing a capability
- `all() -> Vec<AgentInfo>` - Returns all registered agents

### Indexing

Internal indices are maintained for:

- **topic_index**: topic -> agent_ids (for routing and fanout)
- **capability_index**: capability -> agent_ids (for task allocation)

All indices are automatically updated on register/unregister operations.

### Thread Safety

All methods are safe to call concurrently. Lock-free reads using `DashMap`.

## CapabilityDirectory

Thread-safe snapshot-based directory for capability discovery from `ActionBroker`.

### Snapshot Model

The directory uses a **snapshot model**: call `refresh_from_broker()` to update the internal cache from the ActionBroker's current state. Queries operate on this cached snapshot, not live broker state. This design avoids circular dependencies between `ActionBroker` and `EventBus`.

### API

- `new()` - Creates empty directory
- `refresh_from_broker(broker: &ActionBroker)` - Clears and rebuilds snapshot from broker's current capabilities
- `list() -> Vec<CapabilityDescriptor>` - Returns all capabilities in snapshot
- `find_by_name(name: &str) -> Vec<CapabilityDescriptor>` - Finds all versions of a capability by name
- `get(name: &str, version: &str) -> Option<CapabilityDescriptor>` - Retrieves specific capability by name:version

### Thread Safety

All methods are safe to call concurrently. Uses `DashMap` for lock-free reads.

## Integration Patterns

### Agent Lifecycle

- **On agent creation**: Call `AgentDirectory::register_agent()` with agent info
- **On agent shutdown**: Call `AgentDirectory::unregister_agent()` to clean up

### Capability Discovery

- **At startup**: Call `CapabilityDirectory::refresh_from_broker()` after registering all providers
- **Periodic refresh**: Optional periodic refresh if providers are dynamically registered/unregistered
- **Before routing**: Use `find_by_name()` or `get()` to resolve capabilities for task allocation

### Collaboration Routing

- **Fanout routing**: Use `AgentDirectory::by_topic()` to find agents for broadcast
- **Task allocation**: Use `AgentDirectory::by_capability()` to find agents for specific tasks
- **Metadata filtering**: Query `all()` and filter by metadata fields (e.g., region, load)

## Examples

See `core/tests/directory_test.rs` and `core/tests/integration/e2e_directory.rs` for indexing, snapshot usage, and integration patterns.
