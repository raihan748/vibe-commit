use vibe_commit::chunker::classifier::{CommitType, SemanticClassifier};
use vibe_commit::chunker::sorter::DependencySorter;
use vibe_commit::cli::args::OutputFormat;
use vibe_commit::diff::parser::DiffParser;
use vibe_commit::diff::types::SymbolKind;
use vibe_commit::planner::formatter::PlanFormatter;
use vibe_commit::plan_from_diff;

const SAMPLE_MONOLITHIC_DIFF: &str = r#"diff --git a/Cargo.toml b/Cargo.toml
index 1111111..2222222 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -10,3 +10,6 @@ edition = "2021"
 [dependencies]
 serde = { version = "1.0", features = ["derive"] }
+tokio = { version = "1.36", features = ["full"] }
+axum = "0.7"
+uuid = { version = "1.7", features = ["v4"] }
diff --git a/src/models/user.rs b/src/models/user.rs
new file mode 100644
--- /dev/null
+++ b/src/models/user.rs
@@ -0,0 +1,15 @@
+use serde::{Deserialize, Serialize};
+use uuid::Uuid;
+
+#[derive(Debug, Clone, Serialize, Deserialize)]
+pub struct User {
+    pub id: Uuid,
+    pub username: String,
+    pub email: String,
+}
+
+pub enum UserRole {
+    Admin,
+    Member,
+}
diff --git a/src/api/routes.rs b/src/api/routes.rs
new file mode 100644
--- /dev/null
+++ b/src/api/routes.rs
@@ -0,0 +1,18 @@
+use crate::models::user::User;
+use axum::{routing::get, Router, Json};
+
+pub fn user_router() -> Router {
+    Router::new().route("/users", get(list_users))
+}
+
+pub async fn list_users() -> Json<Vec<User>> {
+    Json(vec![])
+}
diff --git a/src/api/handler.rs b/src/api/handler.rs
index 3333333..4444444 100644
--- a/src/api/handler.rs
+++ b/src/api/handler.rs
@@ -15,5 +15,7 @@ pub async fn get_user(id: String) -> Result<String, String> {
-    let user = find_user(id).unwrap();
+    let user = find_user(id).unwrap_or_else(|| {
+        // fix: prevent panic on missing user record
+        return "User not found".to_string();
+    });
     Ok(user)
 }
diff --git a/tests/user_api_test.rs b/tests/user_api_test.rs
new file mode 100644
--- /dev/null
+++ b/tests/user_api_test.rs
@@ -0,0 +1,14 @@
+#[tokio::test]
+async fn test_user_router_initialization() {
+    let app = vibe_app::api::routes::user_router();
+    assert!(true);
+}
diff --git a/README.md b/README.md
index 5555555..6666666 100644
--- a/README.md
+++ b/README.md
@@ -1,3 +1,8 @@
 # Project SuperApp
 
+## Features
+- Asynchronous User API with Axum
+- UUID v4 Identity model
+- Comprehensive test coverage
"#;

#[test]
fn test_diff_parser_handles_monolithic_diff() {
    let diff = DiffParser::parse(SAMPLE_MONOLITHIC_DIFF).expect("Diff parsing should succeed");
    assert_eq!(diff.files.len(), 6);
    assert!(diff.total_additions > 0);

    // Verify Cargo.toml file patch
    let cargo_patch = diff.files.iter().find(|f| f.display_path() == "Cargo.toml").unwrap();
    assert_eq!(cargo_patch.hunks.len(), 1);
    assert_eq!(cargo_patch.total_additions, 3);
    assert_eq!(cargo_patch.total_deletions, 0);

    // Verify new file detection
    let user_patch = diff.files.iter().find(|f| f.display_path() == "src/models/user.rs").unwrap();
    assert!(user_patch.is_new);
    assert_eq!(user_patch.old_path, None);
    assert_eq!(user_patch.new_path, Some("src/models/user.rs".to_string()));

    // Verify symbols extracted from user.rs
    let symbols = user_patch.hunks[0].extracted_symbols();
    let struct_sym = symbols.iter().find(|s| s.kind == SymbolKind::Struct).unwrap();
    assert_eq!(struct_sym.name, "User");
    let enum_sym = symbols.iter().find(|s| s.kind == SymbolKind::Enum).unwrap();
    assert_eq!(enum_sym.name, "UserRole");
}

#[test]
fn test_diff_parser_handles_file_deletion_and_rename() {
    let diff_str = r#"diff --git a/old_config.json b/new_config.json
similarity index 100%
rename from old_config.json
rename to new_config.json
diff --git a/obsolete.txt b/obsolete.txt
deleted file mode 100644
--- a/obsolete.txt
+++ /dev/null
@@ -1,2 +0,0 @@
-line1
-line2
"#;
    let diff = DiffParser::parse(diff_str).unwrap();
    assert_eq!(diff.files.len(), 2);

    let rename_patch = &diff.files[0];
    assert!(rename_patch.is_renamed);
    assert_eq!(rename_patch.old_path.as_deref(), Some("old_config.json"));
    assert_eq!(rename_patch.new_path.as_deref(), Some("new_config.json"));

    let delete_patch = &diff.files[1];
    assert!(delete_patch.is_deleted);
    assert_eq!(delete_patch.new_path, None);
    assert_eq!(delete_patch.total_deletions, 2);
}

#[test]
fn test_semantic_classifier_categories() {
    let diff = DiffParser::parse(SAMPLE_MONOLITHIC_DIFF).unwrap();

    let mut classifications = Vec::new();
    for file in &diff.files {
        let chunks = SemanticClassifier::classify_file(file);
        for chunk in chunks {
            classifications.push((chunk.file_path.clone(), chunk.commit_type, chunk.scope.clone()));
        }
    }


    // Cargo.toml -> chore(deps)
    let cargo = classifications.iter().find(|c| c.0 == "Cargo.toml").unwrap();
    assert_eq!(cargo.1, CommitType::Chore);
    assert_eq!(cargo.2, "deps");

    // src/models/user.rs -> feat(models)
    let user = classifications.iter().find(|c| c.0 == "src/models/user.rs").unwrap();
    assert_eq!(user.1, CommitType::Feat);
    assert_eq!(user.2, "models");

    // src/api/routes.rs -> feat(api)
    let routes = classifications.iter().find(|c| c.0 == "src/api/routes.rs").unwrap();
    assert_eq!(routes.1, CommitType::Feat);
    assert_eq!(routes.2, "api");

    // src/api/handler.rs -> fix(api)
    let handler = classifications.iter().find(|c| c.0 == "src/api/handler.rs").unwrap();
    assert_eq!(handler.1, CommitType::Fix);

    // tests/user_api_test.rs -> test(tests)
    let test = classifications.iter().find(|c| c.0 == "tests/user_api_test.rs").unwrap();
    assert_eq!(test.1, CommitType::Test);

    // README.md -> docs(readme)
    let readme = classifications.iter().find(|c| c.0 == "README.md").unwrap();
    assert_eq!(readme.1, CommitType::Docs);
    assert_eq!(readme.2, "readme");
}

#[test]
fn test_dependency_sorter_topological_order() {
    let diff = DiffParser::parse(SAMPLE_MONOLITHIC_DIFF).unwrap();
    let mut chunks = Vec::new();
    for file in &diff.files {
        chunks.extend(SemanticClassifier::classify_file(file));
    }

    let sorted = DependencySorter::sort(chunks);
    let order: Vec<CommitType> = sorted.iter().map(|c| c.commit_type).collect();

    // Verify topological order:
    // Chore comes first
    assert_eq!(order.first(), Some(&CommitType::Chore));
    // Docs comes last
    assert_eq!(order.last(), Some(&CommitType::Docs));

    // Find indices
    let idx_chore = order.iter().position(|t| *t == CommitType::Chore).unwrap();
    let idx_feat = order.iter().position(|t| *t == CommitType::Feat).unwrap();
    let idx_fix = order.iter().position(|t| *t == CommitType::Fix).unwrap();
    let idx_test = order.iter().position(|t| *t == CommitType::Test).unwrap();
    let idx_docs = order.iter().position(|t| *t == CommitType::Docs).unwrap();

    assert!(idx_chore < idx_feat, "Chore must precede Feat");
    assert!(idx_feat < idx_test, "Feat must precede Test");
    assert!(idx_test < idx_docs, "Test must precede Docs");
    assert!(idx_fix < idx_docs, "Fix must precede Docs");
}

#[test]
fn test_plan_generator_conventional_commits() {
    let plan = plan_from_diff(SAMPLE_MONOLITHIC_DIFF, None).expect("Planning should succeed");

    assert!(!plan.commits.is_empty());
    assert_eq!(plan.total_files, 6);

    for commit in &plan.commits {
        assert!(commit.title.contains('('), "Commit title should contain scope: {}", commit.title);
        assert!(commit.title.contains("): "), "Commit title should follow conventional format: {}", commit.title);
        assert!(!commit.files.is_empty());
        assert!(commit.additions > 0 || commit.deletions > 0);
    }
}

#[test]
fn test_plan_generator_max_commits_limit() {
    let plan = plan_from_diff(SAMPLE_MONOLITHIC_DIFF, Some(3)).expect("Planning with max commits should succeed");
    assert!(plan.commits.len() <= 3, "Plan should not exceed max commits limit: got {}", plan.commits.len());
}

#[test]
fn test_plan_formatter_markdown_output() {
    let plan = plan_from_diff(SAMPLE_MONOLITHIC_DIFF, None).unwrap();
    let md = PlanFormatter::format(&plan, OutputFormat::Markdown, false).unwrap();

    assert!(md.contains("# VibeCommit Execution Plan"));
    assert!(md.contains("| # | Type | Scope | Title | Files | Diff |"));
    assert!(md.contains("### Commit 1:"));
}

#[test]
fn test_plan_formatter_json_output() {
    let plan = plan_from_diff(SAMPLE_MONOLITHIC_DIFF, None).unwrap();
    let json_str = PlanFormatter::format(&plan, OutputFormat::Json, false).unwrap();

    assert!(json_str.contains("\"total_files\": 6"));
    assert!(json_str.contains("\"commits\":"));

    let deserialized: vibe_commit::planner::generator::CommitPlan =
        serde_json::from_str(&json_str).expect("JSON must roundtrip deserialize");
    assert_eq!(deserialized.total_files, plan.total_files);
    assert_eq!(deserialized.commits.len(), plan.commits.len());
}

#[test]
fn test_plan_formatter_shell_dry_run() {
    let plan = plan_from_diff(SAMPLE_MONOLITHIC_DIFF, None).unwrap();
    let shell = PlanFormatter::format(&plan, OutputFormat::Shell, true).unwrap();

    assert!(shell.contains("#!/usr/bin/env bash"));
    assert!(shell.contains("[DRY RUN]"));
    assert!(shell.contains("[DRY-RUN] Would stage and commit:"));
}

#[test]
fn test_empty_diff_handling() {
    let plan = plan_from_diff("", None).unwrap();
    assert_eq!(plan.commits.len(), 0);
    assert_eq!(plan.total_files, 0);

    let md = PlanFormatter::format(&plan, OutputFormat::Markdown, false).unwrap();
    assert!(md.contains("No changes detected in diff."));
}

#[test]
fn test_hunk_header_variations() {
    let diff_str = r#"--- a/file.txt
+++ b/file.txt
@@ -1 +1 @@
-old
+new
@@ -10,3 +10,4 @@ pub fn calculate()
 context
-bad
+good
+better
"#;
    let diff = DiffParser::parse(diff_str).unwrap();
    assert_eq!(diff.files.len(), 1);
    assert_eq!(diff.files[0].hunks.len(), 2);
    assert_eq!(diff.files[0].hunks[0].old_lines, 1);
    assert_eq!(diff.files[0].hunks[0].new_lines, 1);
    assert_eq!(diff.files[0].hunks[1].heading.as_deref(), Some("pub fn calculate()"));
}

#[test]
fn test_multi_hunk_splitting_logic_and_test() {
    let diff_str = r#"diff --git a/src/service.rs b/src/service.rs
index 0000001..0000002 100644
--- a/src/service.rs
+++ b/src/service.rs
@@ -10,5 +10,12 @@ pub struct Service;
 impl Service {
+    pub fn process_data(&self) -> bool {
+        true
+    }
 }
@@ -30,4 +37,9 @@ impl Service {
 #[cfg(test)]
 mod tests {
+    #[test]
+    fn test_process_data() {
+        assert!(true);
+    }
 }
"#;
    let diff = DiffParser::parse(diff_str).unwrap();
    assert_eq!(diff.files.len(), 1);
    let patch = &diff.files[0];
    assert_eq!(patch.hunks.len(), 2);

    let chunks = SemanticClassifier::classify_file(patch);
    assert_eq!(chunks.len(), 2, "Should split service logic and test into separate chunks");

    assert_eq!(chunks[0].commit_type, CommitType::Feat);
    assert_eq!(chunks[1].commit_type, CommitType::Test);
}

#[test]
fn test_plan_formatter_shell_real_commands() {
    let plan = plan_from_diff(SAMPLE_MONOLITHIC_DIFF, None).unwrap();
    let shell = PlanFormatter::format(&plan, OutputFormat::Shell, false).unwrap();

    assert!(shell.contains("#!/usr/bin/env bash"));
    assert!(shell.contains("set -euo pipefail"));
    assert!(shell.contains("git add \"Cargo.toml\""));
    assert!(shell.contains("git commit -m \"chore(deps): update project configurations and dependencies\""));
}

#[test]
fn test_malformed_hunk_header_error() {
    let bad_diff = r#"--- a/file.txt
+++ b/file.txt
@@ invalid header @@
+hello
"#;
    let result = DiffParser::parse(bad_diff);
    assert!(result.is_err(), "Invalid hunk header must return error");
}

