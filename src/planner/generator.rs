use crate::chunker::classifier::{CommitType, SemanticChunk};
use crate::planner::message::CommitMessageBuilder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedCommit {
    pub index: usize,
    pub commit_type: CommitType,
    pub scope: String,
    pub title: String,
    pub body: Vec<String>,
    pub files: Vec<String>,
    pub hunk_count: usize,
    pub additions: usize,
    pub deletions: usize,
    pub patch: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitPlan {
    pub commits: Vec<PlannedCommit>,
    pub total_files: usize,
    pub total_additions: usize,
    pub total_deletions: usize,
}

pub struct PlanGenerator;

impl PlanGenerator {
    /// Generates an atomic conventional commit plan from sorted semantic chunks.
    pub fn generate(chunks: &[SemanticChunk], max_commits: Option<usize>) -> CommitPlan {
        if chunks.is_empty() {
            return CommitPlan {
                commits: Vec::new(),
                total_files: 0,
                total_additions: 0,
                total_deletions: 0,
            };
        }

        // Group consecutive chunks with matching commit_type and scope
        let mut groups: Vec<Vec<&SemanticChunk>> = Vec::new();
        for chunk in chunks {
            if let Some(last_group) = groups.last_mut() {
                let first = last_group[0];
                if first.commit_type == chunk.commit_type && first.scope == chunk.scope {
                    last_group.push(chunk);
                    continue;
                }
            }
            groups.push(vec![chunk]);
        }

        // If max_commits specified and groups exceed it, merge adjacent compatible groups
        if let Some(limit) = max_commits {
            let max_k = limit.max(1);
            while groups.len() > max_k {
                // Find best adjacent pair to merge:
                // 1. Same commit_type
                // 2. Or adjacent compatible types (e.g. Chore + Docs or Fix + Refactor)
                let mut best_idx = 0;
                let mut found_same_type = false;

                for i in 0..(groups.len() - 1) {
                    if groups[i][0].commit_type == groups[i + 1][0].commit_type {
                        best_idx = i;
                        found_same_type = true;
                        break;
                    }
                }

                if !found_same_type {
                    // Pick the two smallest adjacent groups to merge
                    let mut min_weight = usize::MAX;
                    for i in 0..(groups.len() - 1) {
                        let w = groups[i].len() + groups[i + 1].len();
                        if w < min_weight {
                            min_weight = w;
                            best_idx = i;
                        }
                    }
                }

                let next_group = groups.remove(best_idx + 1);
                groups[best_idx].extend(next_group);
            }
        }

        let mut planned_commits = Vec::new();
        let mut all_files = std::collections::HashSet::new();
        let mut total_additions = 0;
        let mut total_deletions = 0;

        for (idx, group) in groups.into_iter().enumerate() {
            let primary = group[0];
            let commit_type = primary.commit_type;

            // Merge scopes if group has multiple scopes
            let scopes: Vec<&str> = group.iter().map(|c| c.scope.as_str()).collect();
            let mut unique_scopes = Vec::new();
            for s in scopes {
                if !unique_scopes.contains(&s) {
                    unique_scopes.push(s);
                }
            }
            let combined_scope = if unique_scopes.len() == 1 {
                unique_scopes[0].to_string()
            } else if unique_scopes.len() <= 2 {
                unique_scopes.join("/")
            } else {
                primary.scope.clone()
            };

            let summary = &primary.summary;
            let title = CommitMessageBuilder::build_title(commit_type, &combined_scope, summary);
            let body = CommitMessageBuilder::build_body(&group);

            let mut files = Vec::new();
            let mut hunk_count = 0;
            let mut additions = 0;
            let mut deletions = 0;
            let mut patch_content = String::new();

            for chunk in group {
                if !files.contains(&chunk.file_path) {
                    files.push(chunk.file_path.clone());
                    all_files.insert(chunk.file_path.clone());
                }
                hunk_count += chunk.hunk_indices.len();
                additions += chunk.additions;
                deletions += chunk.deletions;
                patch_content.push_str(&chunk.raw_patch);
            }

            total_additions += additions;
            total_deletions += deletions;

            planned_commits.push(PlannedCommit {
                index: idx + 1,
                commit_type,
                scope: combined_scope,
                title,
                body,
                files,
                hunk_count,
                additions,
                deletions,
                patch: patch_content,
            });
        }

        CommitPlan {
            commits: planned_commits,
            total_files: all_files.len(),
            total_additions,
            total_deletions,
        }
    }
}
