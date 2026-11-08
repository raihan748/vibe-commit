use crate::diff::types::{DiffLine, DiffLineType, FilePatch, Hunk, UnifiedDiff};

#[derive(Debug, PartialEq, Eq)]
pub enum DiffParseError {
    EmptyDiff,
    InvalidHunkHeader(String),
}

impl std::fmt::Display for DiffParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiffParseError::EmptyDiff => write!(f, "Diff input is empty"),
            DiffParseError::InvalidHunkHeader(h) => write!(f, "Invalid hunk header: {}", h),
        }
    }
}

impl std::error::Error for DiffParseError {}

/// Unified diff parser for git patches.
pub struct DiffParser;

impl DiffParser {
    /// Parse a unified diff string into a structured `UnifiedDiff`.
    pub fn parse(input: &str) -> Result<UnifiedDiff, DiffParseError> {
        let lines: Vec<&str> = input.lines().collect();
        if lines.is_empty() || (lines.len() == 1 && lines[0].trim().is_empty()) {
            return Ok(UnifiedDiff {
                files: Vec::new(),
                total_additions: 0,
                total_deletions: 0,
            });
        }

        let mut files = Vec::new();
        let mut idx = 0;

        while idx < lines.len() {
            let line = lines[idx];

            if line.starts_with("diff --git ") {
                let (file_patch, next_idx) = Self::parse_file_patch(&lines, idx)?;
                files.push(file_patch);
                idx = next_idx;
            } else if line.starts_with("--- ") {
                // Unified diff without 'diff --git' header
                let (file_patch, next_idx) = Self::parse_file_patch_from_triple_minus(&lines, idx)?;
                files.push(file_patch);
                idx = next_idx;
            } else {
                idx += 1;
            }
        }

        let mut total_additions = 0;
        let mut total_deletions = 0;
        for f in &files {
            total_additions += f.total_additions;
            total_deletions += f.total_deletions;
        }

        Ok(UnifiedDiff {
            files,
            total_additions,
            total_deletions,
        })
    }

    fn parse_file_patch(lines: &[&str], start_idx: usize) -> Result<(FilePatch, usize), DiffParseError> {
        let header_line = lines[start_idx];
        let mut old_path = None;
        let mut new_path = None;
        let mut is_new = false;
        let mut is_deleted = false;
        let mut is_renamed = false;

        // Try to parse diff --git a/path b/path
        if let Some(paths) = header_line.strip_prefix("diff --git ") {
            let parts: Vec<&str> = paths.split_whitespace().collect();
            if parts.len() >= 2 {
                let raw_old = parts[0];
                let raw_new = parts[1];
                old_path = strip_git_prefix(raw_old);
                new_path = strip_git_prefix(raw_new);
            }
        }

        let mut idx = start_idx + 1;
        while idx < lines.len() {
            let line = lines[idx];
            if line.starts_with("diff --git ") {
                break;
            }

            if line.starts_with("new file mode") {
                is_new = true;
                old_path = None;
            } else if line.starts_with("deleted file mode") {
                is_deleted = true;
                new_path = None;
            } else if line.starts_with("rename from ") {
                is_renamed = true;
                old_path = Some(line.trim_start_matches("rename from ").trim().to_string());
            } else if line.starts_with("rename to ") {
                is_renamed = true;
                new_path = Some(line.trim_start_matches("rename to ").trim().to_string());
            } else if line.starts_with("--- ") {
                let p = parse_file_marker(line, "--- ");
                if p != "/dev/null" {
                    old_path = Some(p);
                } else {
                    is_new = true;
                    old_path = None;
                }
            } else if line.starts_with("+++ ") {
                let p = parse_file_marker(line, "+++ ");
                if p != "/dev/null" {
                    new_path = Some(p);
                } else {
                    is_deleted = true;
                    new_path = None;
                }
            } else if line.starts_with("@@ ") {
                // Start of hunks
                break;
            }
            idx += 1;
        }

        let mut hunks = Vec::new();
        while idx < lines.len() {
            let line = lines[idx];
            if line.starts_with("diff --git ") {
                break;
            }
            if line.starts_with("@@ ") {
                let (hunk, next_idx) = Self::parse_hunk(lines, idx)?;
                hunks.push(hunk);
                idx = next_idx;
            } else {
                idx += 1;
            }
        }

        let total_additions = hunks.iter().map(|h| h.additions).sum();
        let total_deletions = hunks.iter().map(|h| h.deletions).sum();

        let patch = FilePatch {
            old_path,
            new_path,
            is_new,
            is_deleted,
            is_renamed,
            hunks,
            total_additions,
            total_deletions,
        };

        Ok((patch, idx))
    }

    fn parse_file_patch_from_triple_minus(lines: &[&str], start_idx: usize) -> Result<(FilePatch, usize), DiffParseError> {
        let mut idx = start_idx;
        let mut old_path = None;
        let mut new_path = None;
        let mut is_new = false;
        let mut is_deleted = false;

        if lines[idx].starts_with("--- ") {
            let p = parse_file_marker(lines[idx], "--- ");
            if p == "/dev/null" {
                is_new = true;
            } else {
                old_path = Some(p);
            }
            idx += 1;
        }

        if idx < lines.len() && lines[idx].starts_with("+++ ") {
            let p = parse_file_marker(lines[idx], "+++ ");
            if p == "/dev/null" {
                is_deleted = true;
            } else {
                new_path = Some(p);
            }
            idx += 1;
        }

        let mut hunks = Vec::new();
        while idx < lines.len() {
            let line = lines[idx];
            if line.starts_with("--- ") || line.starts_with("diff --git ") {
                break;
            }
            if line.starts_with("@@ ") {
                let (hunk, next_idx) = Self::parse_hunk(lines, idx)?;
                hunks.push(hunk);
                idx = next_idx;
            } else {
                idx += 1;
            }
        }

        let total_additions = hunks.iter().map(|h| h.additions).sum();
        let total_deletions = hunks.iter().map(|h| h.deletions).sum();

        let patch = FilePatch {
            old_path,
            new_path,
            is_new,
            is_deleted,
            is_renamed: false,
            hunks,
            total_additions,
            total_deletions,
        };

        Ok((patch, idx))
    }

    fn parse_hunk(lines: &[&str], start_idx: usize) -> Result<(Hunk, usize), DiffParseError> {
        let header = lines[start_idx];
        let (old_start, old_lines, new_start, new_lines, heading) = parse_hunk_header(header)?;

        let mut diff_lines = Vec::new();
        let mut additions = 0;
        let mut deletions = 0;

        let mut curr_old = old_start;
        let mut curr_new = new_start;

        let mut idx = start_idx + 1;
        while idx < lines.len() {
            let line = lines[idx];
            if line.starts_with("@@ ") || line.starts_with("diff --git ") || (line.starts_with("--- ") && idx + 1 < lines.len() && lines[idx + 1].starts_with("+++ ")) {
                break;
            }

            if line.starts_with('\\') {
                // e.g. \ No newline at end of file
                idx += 1;
                continue;
            }

            if let Some(content) = line.strip_prefix('+') {
                additions += 1;
                diff_lines.push(DiffLine {
                    line_type: DiffLineType::Addition,
                    content: content.to_string(),
                    old_line_no: None,
                    new_line_no: Some(curr_new),
                });
                curr_new += 1;
            } else if let Some(content) = line.strip_prefix('-') {
                deletions += 1;
                diff_lines.push(DiffLine {
                    line_type: DiffLineType::Deletion,
                    content: content.to_string(),
                    old_line_no: Some(curr_old),
                    new_line_no: None,
                });
                curr_old += 1;
            } else {
                // Context line (can start with ' ' or be empty in loose diffs)
                let content = if let Some(c) = line.strip_prefix(' ') {
                    c
                } else {
                    line
                };
                diff_lines.push(DiffLine {
                    line_type: DiffLineType::Context,
                    content: content.to_string(),
                    old_line_no: Some(curr_old),
                    new_line_no: Some(curr_new),
                });
                curr_old += 1;
                curr_new += 1;
            }

            idx += 1;
        }

        Ok((
            Hunk {
                old_start,
                old_lines,
                new_start,
                new_lines,
                heading,
                lines: diff_lines,
                additions,
                deletions,
            },
            idx,
        ))
    }
}

fn strip_git_prefix(path: &str) -> Option<String> {
    if path == "/dev/null" {
        return None;
    }
    if let Some(p) = path.strip_prefix("a/") {
        Some(p.to_string())
    } else if let Some(p) = path.strip_prefix("b/") {
        Some(p.to_string())
    } else {
        Some(path.to_string())
    }
}

fn parse_file_marker(line: &str, prefix: &str) -> String {
    let rest = line.trim_start_matches(prefix).trim();
    // Path might have timestamp after tab or whitespace
    let path_str = rest.split(['\t', ' ']).next().unwrap_or(rest);
    if path_str == "/dev/null" {
        return "/dev/null".to_string();
    }
    if let Some(p) = path_str.strip_prefix("a/") {
        p.to_string()
    } else if let Some(p) = path_str.strip_prefix("b/") {
        p.to_string()
    } else {
        path_str.to_string()
    }
}

fn parse_hunk_header(header: &str) -> Result<(usize, usize, usize, usize, Option<String>), DiffParseError> {
    // @@ -old_start,old_lines +new_start,new_lines @@ heading
    if !header.starts_with("@@ ") {
        return Err(DiffParseError::InvalidHunkHeader(header.to_string()));
    }

    let end_marker = header[3..].find("@@");
    if end_marker.is_none() {
        return Err(DiffParseError::InvalidHunkHeader(header.to_string()));
    }
    let end_idx = 3 + end_marker.unwrap();
    let range_str = header[3..end_idx].trim();
    let heading = header[end_idx + 2..].trim();
    let heading_opt = if heading.is_empty() {
        None
    } else {
        Some(heading.to_string())
    };

    let parts: Vec<&str> = range_str.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(DiffParseError::InvalidHunkHeader(header.to_string()));
    }

    let (old_start, old_lines) = parse_range(parts[0], '-')?;
    let (new_start, new_lines) = parse_range(parts[1], '+')?;

    Ok((old_start, old_lines, new_start, new_lines, heading_opt))
}

fn parse_range(s: &str, expected_prefix: char) -> Result<(usize, usize), DiffParseError> {
    if !s.starts_with(expected_prefix) {
        return Err(DiffParseError::InvalidHunkHeader(s.to_string()));
    }
    let s_clean = &s[1..];
    if let Some((start_s, count_s)) = s_clean.split_once(',') {
        let start = start_s.parse::<usize>().map_err(|_| DiffParseError::InvalidHunkHeader(s.to_string()))?;
        let count = count_s.parse::<usize>().map_err(|_| DiffParseError::InvalidHunkHeader(s.to_string()))?;
        Ok((start, count))
    } else {
        let start = s_clean.parse::<usize>().map_err(|_| DiffParseError::InvalidHunkHeader(s.to_string()))?;
        Ok((start, 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_range_single_and_multiple() {
        assert_eq!(parse_range("-42,10", '-').unwrap(), (42, 10));
        assert_eq!(parse_range("+1", '+').unwrap(), (1, 1));
        assert!(parse_range("invalid", '+').is_err());
    }

    #[test]
    fn test_strip_git_prefix() {
        assert_eq!(strip_git_prefix("a/src/main.rs").as_deref(), Some("src/main.rs"));
        assert_eq!(strip_git_prefix("b/lib/core.go").as_deref(), Some("lib/core.go"));
        assert_eq!(strip_git_prefix("/dev/null"), None);
    }
}

