//! Retrieval subsystem for context items.

pub mod strategy;

pub use strategy::{
    CompositeRetrieval, ImportanceRetrieval, RecencyRetrieval, RetrievalStrategy, RetrievalTrigger,
    TypeFilteredRetrieval,
};
