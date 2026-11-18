//! UMMP core engine.
//!
//! Layered modular monolith: Domain → Application → Infrastructure.
//! No GUI, AI provider, or Minecraft loader dependency.

pub mod analyzer;
pub mod checkpoint;
pub mod config;
pub mod events;
pub mod filesystem;
pub mod jobs;
pub mod knowledge;
pub mod metadata;
pub mod persistence;
pub mod project;
pub mod session;

pub use knowledge::{BreakingChange, Compatibility, KnowledgeBase, Mapping, ModDependency, Recipe};
pub use project::ProjectManager;
