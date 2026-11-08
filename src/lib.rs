//! vibe-commit: Monolithic AI diff chunker & atomic conventional commit planner.

pub mod chunker;
pub mod diff;

pub use chunker::*;
pub use diff::*;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");
