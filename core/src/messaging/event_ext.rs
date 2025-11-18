//! Extension trait for Event providing fluent helpers for envelope metadata.

use crate::proto::Event;

/// Extension trait for Event providing fluent helpers for envelope metadata.
///
/// Simplifies reading and writing envelope metadata without verbose Envelope::from_event() calls.
pub trait EventExt {
    /// Sets the thread_id metadata and returns self for chaining.
    fn with_thread(self, thread_id: String) -> Self;

    /// Sets the correlation_id metadata and returns self for chaining.
    fn with_correlation(self, correlation_id: String) -> Self;

    /// Sets the reply_to metadata and returns self for chaining.
    fn with_reply_to(self, reply_to: String) -> Self;

    /// Sets the sender metadata and returns self for chaining.
    fn with_sender(self, sender: String) -> Self;

    /// Reads thread_id from metadata.
    fn thread_id(&self) -> Option<&str>;

    /// Reads correlation_id from metadata.
    fn correlation_id(&self) -> Option<&str>;

    /// Reads reply_to from metadata.
    fn reply_to(&self) -> Option<&str>;

    /// Reads sender from metadata.
    fn sender(&self) -> Option<&str>;
}

impl EventExt for Event {
    fn with_thread(mut self, thread_id: String) -> Self {
        self.metadata.insert(
            crate::messaging::envelope::keys::THREAD_ID.to_string(),
            thread_id,
        );
        self
    }

    fn with_correlation(mut self, correlation_id: String) -> Self {
        self.metadata.insert(
            crate::messaging::envelope::keys::CORRELATION_ID.to_string(),
            correlation_id,
        );
        self
    }

    fn with_reply_to(mut self, reply_to: String) -> Self {
        self.metadata.insert(
            crate::messaging::envelope::keys::REPLY_TO.to_string(),
            reply_to,
        );
        self
    }

    fn with_sender(mut self, sender: String) -> Self {
        self.metadata
            .insert(crate::messaging::envelope::keys::SENDER.to_string(), sender);
        self
    }

    fn thread_id(&self) -> Option<&str> {
        self.metadata
            .get(crate::messaging::envelope::keys::THREAD_ID)
            .map(|s| s.as_str())
    }

    fn correlation_id(&self) -> Option<&str> {
        self.metadata
            .get(crate::messaging::envelope::keys::CORRELATION_ID)
            .map(|s| s.as_str())
    }

    fn reply_to(&self) -> Option<&str> {
        self.metadata
            .get(crate::messaging::envelope::keys::REPLY_TO)
            .map(|s| s.as_str())
    }

    fn sender(&self) -> Option<&str> {
        // Try without prefix first (Rust native), then with "loom." prefix (Python SDK)
        self.metadata
            .get(crate::messaging::envelope::keys::SENDER)
            .or_else(|| self.metadata.get("loom.sender"))
            .map(|s| s.as_str())
    }
}
