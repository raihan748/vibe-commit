use serde::{Deserialize, Serialize};
use std::path::Path;

/// Line type in a unified diff hunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffLineType {
    Context,
    Addition,
    Deletion,
}

/// A single line in a diff hunk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffLine {
    pub line_type: DiffLineType,
    pub content: String,
    pub old_line_no: Option<usize>,
    pub new_line_no: Option<usize>,
}

/// A hunk within a file patch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hunk {
    pub old_start: usize,
    pub old_lines: usize,
    pub new_start: usize,
    pub new_lines: usize,
    pub heading: Option<String>,
    pub lines: Vec<DiffLine>,
    pub additions: usize,
    pub deletions: usize,
}

impl Hunk {
    /// Returns the raw unified diff format of this hunk.
    pub fn raw_hunk(&self) -> String {
        let mut out = String::new();
        let heading_suffix = match &self.heading {
            Some(h) if !h.is_empty() => format!(" {}", h),
            _ => String::new(),
        };
        out.push_str(&format!(
            "@@ -{},{} +{},{} @@{}\n",
            self.old_start, self.old_lines, self.new_start, self.new_lines, heading_suffix
        ));
        for line in &self.lines {
            match line.line_type {
                DiffLineType::Context => {
                    out.push(' ');
                    out.push_str(&line.content);
                    out.push('\n');
                }
                DiffLineType::Addition => {
                    out.push('+');
                    out.push_str(&line.content);
                    out.push('\n');
                }
                DiffLineType::Deletion => {
                    out.push('-');
                    out.push_str(&line.content);
                    out.push('\n');
                }
            }
        }
        out
    }

    /// Extracts structural semantic keywords found in addition lines of this hunk.
    pub fn extracted_symbols(&self) -> Vec<StructuralSymbol> {
        let mut symbols = Vec::new();
        for line in &self.lines {
            if line.line_type == DiffLineType::Addition {
                if let Some(sym) = StructuralSymbol::detect(&line.content, line.new_line_no.unwrap_or(0)) {
                    symbols.push(sym);
                }
            }
        }
        symbols
    }
}

/// Structural semantic boundaries in code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuralSymbol {
    pub kind: SymbolKind,
    pub name: String,
    pub line_no: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SymbolKind {
    Function,
    Struct,
    Class,
    Interface,
    Enum,
    TypeAlias,
    Import,
    Config,
}

impl StructuralSymbol {
    pub fn detect(line: &str, line_no: usize) -> Option<Self> {
        let trimmed = line.trim();
        // Rust / Go / JS / Python function patterns
        if trimmed.starts_with("pub fn ")
            || trimmed.starts_with("pub async fn ")
            || trimmed.starts_with("fn ")
            || trimmed.starts_with("async fn ")
            || trimmed.starts_with("func ")
            || trimmed.starts_with("def ")
            || trimmed.starts_with("function ")
        {
            let name = extract_identifier(trimmed, &["pub async fn ", "pub fn ", "async fn ", "fn ", "func ", "def ", "function "]);
            return Some(StructuralSymbol {
                kind: SymbolKind::Function,
                name,
                line_no,
            });
        }

        // Struct / Class / Interface
        if trimmed.starts_with("pub struct ") || trimmed.starts_with("struct ") {
            let name = extract_identifier(trimmed, &["pub struct ", "struct "]);
            return Some(StructuralSymbol {
                kind: SymbolKind::Struct,
                name,
                line_no,
            });
        }
        if trimmed.starts_with("pub class ") || trimmed.starts_with("class ") || trimmed.starts_with("export class ") {
            let name = extract_identifier(trimmed, &["export class ", "pub class ", "class "]);
            return Some(StructuralSymbol {
                kind: SymbolKind::Class,
                name,
                line_no,
            });
        }
        if trimmed.starts_with("pub interface ") || trimmed.starts_with("interface ") || trimmed.starts_with("export interface ") {
            let name = extract_identifier(trimmed, &["export interface ", "pub interface ", "interface "]);
            return Some(StructuralSymbol {
                kind: SymbolKind::Interface,
                name,
                line_no,
            });
        }
        if trimmed.starts_with("pub enum ") || trimmed.starts_with("enum ") || trimmed.starts_with("export enum ") {
            let name = extract_identifier(trimmed, &["export enum ", "pub enum ", "enum "]);
            return Some(StructuralSymbol {
                kind: SymbolKind::Enum,
                name,
                line_no,
            });
        }
        if trimmed.starts_with("pub type ") || trimmed.starts_with("type ") || trimmed.starts_with("export type ") {
            let name = extract_identifier(trimmed, &["export type ", "pub type ", "type "]);
            return Some(StructuralSymbol {
                kind: SymbolKind::TypeAlias,
                name,
                line_no,
            });
        }

        // Imports / Dependencies
        if trimmed.starts_with("use ")
            || trimmed.starts_with("import ")
            || trimmed.starts_with("from ")
            || trimmed.starts_with("require(")
        {
            return Some(StructuralSymbol {
                kind: SymbolKind::Import,
                name: trimmed.to_string(),
                line_no,
            });
        }

        // Config lines (key = value or key: value)
        if (trimmed.contains('=') || trimmed.contains(':'))
            && !trimmed.starts_with("//")
            && !trimmed.starts_with('#')
            && !trimmed.starts_with("/*")
            && !trimmed.contains("let ")
            && !trimmed.contains("const ")
            && !trimmed.contains("var ")
        {
            if let Some((k, _)) = trimmed.split_once(&['=', ':'][..]) {
                let k_trim = k.trim();
                if !k_trim.is_empty() && k_trim.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.') {
                    return Some(StructuralSymbol {
                        kind: SymbolKind::Config,
                        name: k_trim.to_string(),
                        line_no,
                    });
                }
            }
        }

        None
    }
}

fn extract_identifier(line: &str, prefixes: &[&str]) -> String {
    for prefix in prefixes {
        if let Some(rest) = line.strip_prefix(prefix) {
            let rest_trimmed = rest.trim_start();
            let ident: String = rest_trimmed
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !ident.is_empty() {
                return ident;
            }
        }
    }
    String::new()
}

/// Represents the patch of a single file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilePatch {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub is_new: bool,
    pub is_deleted: bool,
    pub is_renamed: bool,
    pub hunks: Vec<Hunk>,
    pub total_additions: usize,
    pub total_deletions: usize,
}

impl FilePatch {
    /// Return the primary display path of the file.
    pub fn display_path(&self) -> &str {
        if let Some(ref path) = self.new_path {
            path
        } else if let Some(ref path) = self.old_path {
            path
        } else {
            "unknown"
        }
    }

    /// File extension if present.
    pub fn extension(&self) -> Option<&str> {
        let p = self.display_path();
        Path::new(p).extension().and_then(|ext| ext.to_str())
    }

    /// Generate unified diff string for this single file.
    pub fn raw_patch(&self) -> String {
        let mut out = String::new();
        let old_p = self.old_path.as_deref().unwrap_or("/dev/null");
        let new_p = self.new_path.as_deref().unwrap_or("/dev/null");

        out.push_str(&format!("diff --git a/{} b/{}\n", self.display_path(), self.display_path()));
        if self.is_new {
            out.push_str("new file mode 100644\n");
        } else if self.is_deleted {
            out.push_str("deleted file mode 100644\n");
        }
        out.push_str(&format!("--- {}\n", if old_p == "/dev/null" { "/dev/null".to_string() } else { format!("a/{}", old_p) }));
        out.push_str(&format!("+++ {}\n", if new_p == "/dev/null" { "/dev/null".to_string() } else { format!("b/{}", new_p) }));

        for hunk in &self.hunks {
            out.push_str(&hunk.raw_hunk());
        }
        out
    }
}

/// A parsed unified diff containing multiple files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnifiedDiff {
    pub files: Vec<FilePatch>,
    pub total_additions: usize,
    pub total_deletions: usize,
}

impl UnifiedDiff {
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}
