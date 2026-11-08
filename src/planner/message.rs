use crate::chunker::classifier::{CommitType, SemanticChunk};

pub struct CommitMessageBuilder;

impl CommitMessageBuilder {
    /// Formats a conventional commit title: `<type>(<scope>): <summary>`
    pub fn build_title(commit_type: CommitType, scope: &str, summary: &str) -> String {
        let clean_scope = if scope.trim().is_empty() {
            "core"
        } else {
            scope.trim()
        };

        let clean_summary = summary.trim().trim_end_matches('.');
        format!("{}({}): {}", commit_type.as_str(), clean_scope, clean_summary)
    }

    /// Builds explanation bullet points based on chunk details and symbols.
    pub fn build_body(chunks: &[&SemanticChunk]) -> Vec<String> {
        let mut lines = Vec::new();
        for chunk in chunks {
            for detail in &chunk.details {
                let bullet = format!("- {}", detail);
                if !lines.contains(&bullet) {
                    lines.push(bullet);
                }
            }
        }
        lines
    }
}
