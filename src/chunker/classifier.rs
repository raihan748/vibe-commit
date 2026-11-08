use crate::diff::types::{DiffLineType, FilePatch, Hunk, StructuralSymbol, SymbolKind};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Standard conventional commit types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommitType {
    Chore,
    Feat,
    Fix,
    Refactor,
    Test,
    Docs,
    Style,
}

impl CommitType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CommitType::Chore => "chore",
            CommitType::Feat => "feat",
            CommitType::Fix => "fix",
            CommitType::Refactor => "refactor",
            CommitType::Test => "test",
            CommitType::Docs => "docs",
            CommitType::Style => "style",
        }
    }
}

impl std::fmt::Display for CommitType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A classified semantic chunk of code changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticChunk {
    pub commit_type: CommitType,
    pub scope: String,
    pub file_path: String,
    pub hunk_indices: Vec<usize>,
    pub summary: String,
    pub details: Vec<String>,
    pub additions: usize,
    pub deletions: usize,
    pub symbols: Vec<StructuralSymbol>,
    pub raw_patch: String,
    pub priority_rank: usize,
}

pub struct SemanticClassifier;

impl SemanticClassifier {
    /// Classifies an entire FilePatch into one or more SemanticChunks.
    /// Can split multi-concern files by hunk when appropriate.
    pub fn classify_file(patch: &FilePatch) -> Vec<SemanticChunk> {
        let path = patch.display_path();
        let extension = patch.extension().unwrap_or("");

        // 1. Documentation files
        if is_documentation_path(path, extension) {
            return vec![Self::create_chunk(
                patch,
                CommitType::Docs,
                infer_docs_scope(path),
                70,
                0..patch.hunks.len(),
                "update documentation and guides",
            )];
        }

        // 2. Chore / Configuration / Dependencies
        if is_chore_path(path) {
            let scope = infer_chore_scope(path);
            return vec![Self::create_chunk(
                patch,
                CommitType::Chore,
                scope,
                10,
                0..patch.hunks.len(),
                "update project configurations and dependencies",
            )];
        }

        // 3. Test files
        if is_test_path(path) {
            let scope = infer_scope_from_path(path);
            return vec![Self::create_chunk(
                patch,
                CommitType::Test,
                scope,
                60,
                0..patch.hunks.len(),
                "add comprehensive test coverage",
            )];
        }

        // 4. Source code files: inspect hunks
        if patch.hunks.len() > 1 && should_split_hunks(patch) {
            let mut chunks = Vec::new();
            for (i, hunk) in patch.hunks.iter().enumerate() {
                let (ctype, rank, summary) = classify_single_hunk(hunk, path);
                let scope = infer_scope_from_path(path);
                chunks.push(Self::create_chunk_from_hunk(
                    patch,
                    ctype,
                    scope,
                    rank,
                    i,
                    hunk,
                    &summary,
                ));
            }
            chunks
        } else {
            let (ctype, rank, summary) = classify_hunks_aggregate(&patch.hunks, path);
            let scope = infer_scope_from_path(path);
            vec![Self::create_chunk(
                patch,
                ctype,
                scope,
                rank,
                0..patch.hunks.len(),
                &summary,
            )]
        }
    }

    fn create_chunk(
        patch: &FilePatch,
        commit_type: CommitType,
        scope: String,
        priority_rank: usize,
        hunk_range: std::ops::Range<usize>,
        default_summary: &str,
    ) -> SemanticChunk {
        let mut selected_hunks = Vec::new();
        let mut hunk_indices = Vec::new();
        let mut additions = 0;
        let mut deletions = 0;
        let mut symbols = Vec::new();

        for i in hunk_range {
            if let Some(hunk) = patch.hunks.get(i) {
                hunk_indices.push(i);
                additions += hunk.additions;
                deletions += hunk.deletions;
                symbols.extend(hunk.extracted_symbols());
                selected_hunks.push(hunk);
            }
        }

        let mut raw_patch = String::new();
        let old_p = patch.old_path.as_deref().unwrap_or("/dev/null");
        let new_p = patch.new_path.as_deref().unwrap_or("/dev/null");
        raw_patch.push_str(&format!("diff --git a/{} b/{}\n", patch.display_path(), patch.display_path()));
        if patch.is_new {
            raw_patch.push_str("new file mode 100644\n");
        } else if patch.is_deleted {
            raw_patch.push_str("deleted file mode 100644\n");
        }
        raw_patch.push_str(&format!("--- {}\n", if old_p == "/dev/null" { "/dev/null".to_string() } else { format!("a/{}", old_p) }));
        raw_patch.push_str(&format!("+++ {}\n", if new_p == "/dev/null" { "/dev/null".to_string() } else { format!("b/{}", new_p) }));
        for h in &selected_hunks {
            raw_patch.push_str(&h.raw_hunk());
        }

        let (summary, details) = generate_details(patch.display_path(), commit_type, default_summary, &symbols, additions, deletions);

        SemanticChunk {
            commit_type,
            scope,
            file_path: patch.display_path().to_string(),
            hunk_indices,
            summary,
            details,
            additions,
            deletions,
            symbols,
            raw_patch,
            priority_rank,
        }
    }

    fn create_chunk_from_hunk(
        patch: &FilePatch,
        commit_type: CommitType,
        scope: String,
        priority_rank: usize,
        hunk_index: usize,
        hunk: &Hunk,
        summary: &str,
    ) -> SemanticChunk {
        let symbols = hunk.extracted_symbols();
        let mut raw_patch = String::new();
        let old_p = patch.old_path.as_deref().unwrap_or("/dev/null");
        let new_p = patch.new_path.as_deref().unwrap_or("/dev/null");
        raw_patch.push_str(&format!("diff --git a/{} b/{}\n", patch.display_path(), patch.display_path()));
        raw_patch.push_str(&format!("--- {}\n", if old_p == "/dev/null" { "/dev/null".to_string() } else { format!("a/{}", old_p) }));
        raw_patch.push_str(&format!("+++ {}\n", if new_p == "/dev/null" { "/dev/null".to_string() } else { format!("b/{}", new_p) }));
        raw_patch.push_str(&hunk.raw_hunk());

        let (final_summary, details) = generate_details(patch.display_path(), commit_type, summary, &symbols, hunk.additions, hunk.deletions);

        SemanticChunk {
            commit_type,
            scope,
            file_path: patch.display_path().to_string(),
            hunk_indices: vec![hunk_index],
            summary: final_summary,
            details,
            additions: hunk.additions,
            deletions: hunk.deletions,
            symbols,
            raw_patch,
            priority_rank,
        }
    }
}

fn is_documentation_path(path: &str, ext: &str) -> bool {
    let lower = path.to_lowercase();
    lower.contains("readme")
        || lower.contains("license")
        || lower.contains("contributing")
        || lower.contains("changelog")
        || lower.starts_with("docs/")
        || lower.contains("/docs/")
        || ext == "md"
        || ext == "txt"
        || ext == "adoc"
}

fn is_chore_path(path: &str) -> bool {
    let lower = path.to_lowercase();
    let name = Path::new(path).file_name().and_then(|n| n.to_str()).unwrap_or("");
    matches!(
        name,
        "Cargo.toml"
            | "Cargo.lock"
            | "package.json"
            | "package-lock.json"
            | "pnpm-lock.yaml"
            | "yarn.lock"
            | "tsconfig.json"
            | ".gitignore"
            | ".dockerignore"
            | "Dockerfile"
            | "docker-compose.yml"
            | "docker-compose.yaml"
            | "Makefile"
            | "go.mod"
            | "go.sum"
            | "pyproject.toml"
            | "requirements.txt"
            | ".editorconfig"
            | ".eslintrc.json"
    ) || lower.starts_with(".github/")
        || lower.contains("/.github/")
}

fn is_test_path(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.starts_with("tests/")
        || lower.contains("/tests/")
        || lower.starts_with("test/")
        || lower.contains("/test/")
        || lower.starts_with("spec/")
        || lower.contains("/spec/")
        || lower.contains("__tests__")
        || lower.ends_with("_test.rs")
        || lower.ends_with("_test.go")
        || lower.ends_with(".test.ts")
        || lower.ends_with(".test.js")
        || lower.ends_with(".spec.ts")
        || lower.ends_with(".spec.js")
        || lower.ends_with("_test.py")
}

fn infer_docs_scope(path: &str) -> String {
    let lower = path.to_lowercase();
    if lower.contains("readme") {
        "readme".to_string()
    } else if lower.contains("license") {
        "license".to_string()
    } else {
        "docs".to_string()
    }
}

fn infer_chore_scope(path: &str) -> String {
    let name = Path::new(path).file_name().and_then(|n| n.to_str()).unwrap_or("");
    match name {
        "Cargo.toml" | "Cargo.lock" | "package.json" | "go.mod" | "go.sum" => "deps".to_string(),
        ".gitignore" | ".dockerignore" | ".editorconfig" => "config".to_string(),
        "Dockerfile" | "docker-compose.yml" | "docker-compose.yaml" => "docker".to_string(),
        "Makefile" => "build".to_string(),
        _ if path.contains(".github") => "ci".to_string(),
        _ => "chore".to_string(),
    }
}

pub fn infer_scope_from_path(path: &str) -> String {
    let p = Path::new(path);
    // If inside src/foo/bar.rs -> foo
    let components: Vec<&str> = p.iter().filter_map(|c| c.to_str()).collect();
    if components.len() >= 2 {
        if components[0] == "src" {
            if components.len() >= 3 {
                return components[1].to_string();
            } else {
                // src/main.rs or src/lib.rs
                let stem = Path::new(components[1]).file_stem().and_then(|s| s.to_str()).unwrap_or("core");
                return stem.to_string();
            }
        } else if components[0] == "tests" {
            return "tests".to_string();
        }
    }

    p.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("core")
        .to_string()
}

fn should_split_hunks(patch: &FilePatch) -> bool {
    // If one hunk is test/doc and another is logic, split them
    let mut has_test = false;
    let mut has_logic = false;
    for hunk in &patch.hunks {
        let is_test = hunk.lines.iter().any(|l| {
            l.line_type == DiffLineType::Addition && (l.content.contains("#[test]") || l.content.contains("test_"))
        });
        if is_test {
            has_test = true;
        } else {
            has_logic = true;
        }
    }
    has_test && has_logic
}

fn is_fix_indicator(content: &str) -> bool {
    let lower = content.to_lowercase();
    if lower.contains("unwrap_or") || lower.contains("handle_err") || lower.contains("if err != nil") {
        return true;
    }
    // Check whole word tokens so 'debug' does not match 'bug', and 'prefix' does not match 'fix'
    for token in lower.split(|c: char| !c.is_alphanumeric() && c != '_') {
        if token == "fix" || token == "fixes" || token == "fixed" || token == "bug" || token == "bugfix" || token == "hotfix" {
            return true;
        }
    }
    false
}

fn classify_single_hunk(hunk: &Hunk, _file_path: &str) -> (CommitType, usize, String) {
    let symbols = hunk.extracted_symbols();

    // Check if test hunk
    let is_test = hunk.lines.iter().any(|l| {
        l.line_type == DiffLineType::Addition
            && (l.content.contains("#[test]") || l.content.contains("assert!") || l.content.contains("test_"))
    });
    if is_test {
        return (CommitType::Test, 60, "add unit test verification".to_string());
    }

    // Check if fix
    let is_fix = hunk.lines.iter().any(|l| {
        l.line_type == DiffLineType::Addition && is_fix_indicator(&l.content)
    });
    if is_fix && hunk.additions <= 10 {
        return (CommitType::Fix, 40, "fix edge cases and error handling".to_string());
    }

    // Check if model / struct / types
    let has_types = symbols.iter().any(|s| {
        matches!(
            s.kind,
            SymbolKind::Struct | SymbolKind::Class | SymbolKind::Interface | SymbolKind::Enum | SymbolKind::TypeAlias
        )
    });
    if has_types {
        let type_names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).filter(|n| !n.is_empty()).collect();
        let summary = if !type_names.is_empty() {
            format!("define {} data models and schemas", type_names.join(", "))
        } else {
            "define core data types and entities".to_string()
        };
        return (CommitType::Feat, 20, summary);
    }

    // Feat / Functions / Business logic
    let has_fn = symbols.iter().any(|s| s.kind == SymbolKind::Function);
    if has_fn {
        let fn_names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).filter(|n| !n.is_empty()).collect();
        let summary = if !fn_names.is_empty() {
            format!("implement {}", fn_names.join(", "))
        } else {
            "implement core business logic".to_string()
        };
        return (CommitType::Feat, 30, summary);
    }

    if hunk.deletions > 0 && hunk.additions == 0 {
        return (CommitType::Refactor, 50, "clean up obsolete logic".to_string());
    }

    (CommitType::Feat, 30, "add implementation updates".to_string())
}

fn classify_hunks_aggregate(hunks: &[Hunk], file_path: &str) -> (CommitType, usize, String) {
    let mut all_symbols = Vec::new();
    let mut total_adds = 0;
    let mut total_dels = 0;
    let mut contains_test = false;
    let mut contains_fix = false;

    for h in hunks {
        total_adds += h.additions;
        total_dels += h.deletions;
        all_symbols.extend(h.extracted_symbols());
        for l in &h.lines {
            if l.line_type == DiffLineType::Addition {
                let lower = l.content.to_lowercase();
                if lower.contains("#[test]") || lower.contains("test_") || lower.contains("assert!") {
                    contains_test = true;
                }
                if is_fix_indicator(&l.content) {
                    contains_fix = true;
                }
            }
        }
    }

    if contains_test && total_adds > 0 && all_symbols.is_empty() {
        return (CommitType::Test, 60, "add unit test suite".to_string());
    }

    if contains_fix && total_adds <= 15 && total_dels <= 10 {
        return (CommitType::Fix, 40, "fix boundary handling and edge cases".to_string());
    }

    // Check if new types
    let type_symbols: Vec<&str> = all_symbols
        .iter()
        .filter(|s| {
            matches!(
                s.kind,
                SymbolKind::Struct | SymbolKind::Class | SymbolKind::Interface | SymbolKind::Enum | SymbolKind::TypeAlias
            )
        })
        .map(|s| s.name.as_str())
        .filter(|n| !n.is_empty())
        .collect();

    if !type_symbols.is_empty() {
        let name = Path::new(file_path).file_stem().and_then(|s| s.to_str()).unwrap_or("domain");
        return (
            CommitType::Feat,
            20,
            format!("define {} data structures and models", name),
        );
    }

    // Check functions
    let fn_symbols: Vec<&str> = all_symbols
        .iter()
        .filter(|s| s.kind == SymbolKind::Function)
        .map(|s| s.name.as_str())
        .filter(|n| !n.is_empty())
        .collect();

    if !fn_symbols.is_empty() {
        let name = Path::new(file_path).file_stem().and_then(|s| s.to_str()).unwrap_or("module");
        return (
            CommitType::Feat,
            30,
            format!("implement {} capabilities", name),
        );
    }

    if total_dels > 0 && total_adds <= 5 {
        return (CommitType::Refactor, 50, "refactor and streamline code layout".to_string());
    }

    let stem = Path::new(file_path).file_stem().and_then(|s| s.to_str()).unwrap_or("core");
    (CommitType::Feat, 30, format!("implement {} functionality", stem))
}

fn generate_details(
    path: &str,
    ctype: CommitType,
    summary: &str,
    symbols: &[StructuralSymbol],
    adds: usize,
    dels: usize,
) -> (String, Vec<String>) {
    let mut details = Vec::new();

    let fn_names: Vec<&str> = symbols.iter().filter(|s| s.kind == SymbolKind::Function).map(|s| s.name.as_str()).filter(|n| !n.is_empty()).collect();
    let type_names: Vec<&str> = symbols.iter().filter(|s| matches!(s.kind, SymbolKind::Struct | SymbolKind::Class | SymbolKind::Enum | SymbolKind::Interface)).map(|s| s.name.as_str()).filter(|n| !n.is_empty()).collect();

    if !type_names.is_empty() {
        details.push(format!("add data structures: {}", type_names.join(", ")));
    }
    if !fn_names.is_empty() {
        details.push(format!("implement functions: {}", fn_names.join(", ")));
    }
    details.push(format!("modified {} (+{} -{})", path, adds, dels));

    let final_summary = if !summary.is_empty() {
        summary.to_string()
    } else {
        match ctype {
            CommitType::Chore => "update dependencies and build setup".to_string(),
            CommitType::Feat => "add feature implementation".to_string(),
            CommitType::Fix => "fix error and edge case behavior".to_string(),
            CommitType::Refactor => "restructure internal components".to_string(),
            CommitType::Test => "add automated test coverage".to_string(),
            CommitType::Docs => "update documentation".to_string(),
            CommitType::Style => "improve code format and style".to_string(),
        }
    };

    (final_summary, details)
}
