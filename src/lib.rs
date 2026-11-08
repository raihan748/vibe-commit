//! vibe-commit: Monolithic AI diff chunker & atomic conventional commit planner.

pub mod chunker;
pub mod cli;
pub mod diff;
pub mod planner;

pub use chunker::*;
pub use cli::*;
pub use diff::*;
pub use planner::*;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// High-level API: Takes a raw unified diff string, parses it, classifies and sorts hunks,
/// and produces an atomic conventional commit plan.
pub fn plan_from_diff(diff_content: &str, max_commits: Option<usize>) -> Result<CommitPlan, DiffParseError> {
    let unified_diff = DiffParser::parse(diff_content)?;

    let mut all_chunks = Vec::new();
    for file in &unified_diff.files {
        let file_chunks = SemanticClassifier::classify_file(file);
        all_chunks.extend(file_chunks);
    }

    let sorted_chunks = DependencySorter::sort(all_chunks);
    let plan = PlanGenerator::generate(&sorted_chunks, max_commits);
    Ok(plan)
}
