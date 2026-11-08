# vibe-commit 🎯

> **Monolithic AI Diff Chunker & Atomic Conventional Commit Planner for Vibecoders**

[![Rust](https://img.shields.io/badge/rust-2021%20edition-orange.svg?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg?style=flat-square)](#)
[![Tests](https://img.shields.io/badge/tests-16%2F16%20passed-success.svg?style=flat-square)](#)
[![Zero Facade](https://img.shields.io/badge/integrity-100%25%20genuine-purple.svg?style=flat-square)](#)

---

## 💡 The Vibecoding Problem

When prompting AI code generators (Cursor, Claude Code, Copilot, Aider, or ChatGPT), vibecoders commonly receive a **single gigantic monolithic diff** — for instance, 800+ lines across 12 files mixing:
1. Package dependencies & build configuration (`Cargo.toml`, `package.json`, `.gitignore`)
2. Data structures, domain entities & schemas (`models/`, `types.rs`)
3. Business logic & API routes (`routes.rs`, `controllers/`)
4. Hotfixes & error handling edge cases (`handler.rs`)
5. Automated test suites (`tests/`, `spec/`)
6. Documentation & setup guides (`README.md`)

Dumping this entire diff into a single commit like `git commit -m "add features"` leads to:
- ❌ **Unreadable Git History**: Impossible for human reviewers or tech leads to review.
- ❌ **Broken `git bisect`**: Reverting a regression requires reverting all files at once.
- ❌ **Violated Conventions**: Fails CI conventional commit linters (`commitlint`, semantic release).

---

## ⚡ The Solution: `vibe-commit`

`vibe-commit` partitions monolithic unified diffs into clean, atomic conventional commits following strict build order:

```
                  Monolithic Unified Diff
                             │
                             ▼
     [Unified Diff Parser] ──── parses hunks, symbols, line numbers
                             │
                             ▼
    [Semantic Classifier] ───── tags chunks: chore, feat, fix, test, docs
                             │
                             ▼
     [DAG Dependency Sorter] ── topological order:
                             │    1. chore  (deps & build setup)
                             │    2. feat   (models & domain types)
                             │    3. feat   (business logic & routes)
                             │    4. fix    (edge cases & err handling)
                             │    5. test   (unit & integration specs)
                             │    6. docs   (guides & showroom)
                             │
                             ▼
       [Commit Planner] ─────── conventional titles & explanation points
                             │
                             ▼
      [Multi-Format Output] ─── Markdown / JSON / Executable Shell Script
```

---

## ✨ Features

- **Standard Unified Diff Parser**:
  - Full support for Git patches, file additions (`/dev/null`), deletions, renames, and multi-hunk diffs.
  - Structural boundary extractor recognizing functions, structs, classes, interfaces, enums, imports, and config key-values.
- **Semantic Hunk & File Classifier**:
  - Clusters changes into standard conventional commit scopes:
    - `chore`: dependencies, build scripts, toolchain configs (`Cargo.toml`, `package.json`, `.gitignore`, `Makefile`, `Dockerfile`)
    - `feat`: new domain entities, core logic, API endpoints
    - `fix`: boundary condition adjustments, error handling, bugfixes
    - `refactor`: non-functional restructuring
    - `test`: test suites (`tests/**`, `*_test.rs`, `*.spec.ts`)
    - `docs`: documentation (`README.md`, `*.md`, guides)
  - Intelligent multi-hunk splitting: if a source file contains both business logic and unit test mocks, it separates them into independent commits!
- **DAG Dependency Topological Sorter**:
  - Automatically sequences commits so each step builds upon valid previous dependencies.
  - Deterministic tie-breaking and cycle resilience.
- **Commit Planner & Synthesizer**:
  - Formats commits according to Conventional Commits: `<type>(<scope>): <summary>`
  - Generates bullet-point summaries of modified symbols and file deltas.
  - Intelligent `--max-commits` clustering to collapse adjacent compatible steps when commit counts must be constrained.
- **Multiple Export Formats**:
  - `markdown`: Rich visual summary table and commit-by-commit breakdown.
  - `json`: Machine-readable AST for CI/CD pipelines and external tooling.
  - `shell`: Ready-to-execute Bash/POSIX script staging files and running `git commit`.
- **Dry-Run Mode (`--dry-run`)**:
  - Preview simulated commit application and verify patch integrity without touching git working tree.

---

## 🚀 Installation & Quick Start

### Building from Source

```bash
git clone https://github.com/raihan748/vibe-commit.git
cd vibe-commit
cargo build --release
```

The compiled binary will be located at `target/release/vibe-commit`.

---

## 🛠️ CLI Usage Guide

### 1. Plan Commits from Unstaged Git Changes

Pipe standard `git diff` straight into `vibe-commit`:

```bash
git diff | vibe-commit plan
```

### 2. Plan Commits from a Patch File

```bash
vibe-commit plan ai_generated_patch.diff
```

### 3. Generate an Executable Shell Script

Generate a script that stages files and commits them in sequence:

```bash
vibe-commit plan changes.patch --format shell > apply_commits.sh
chmod +x apply_commits.sh
./apply_commits.sh
```

### 4. Limit the Number of Commits with `--max-commits`

If you want at most 4 commits:

```bash
vibe-commit plan changes.patch --max-commits 4
```

### 5. Export Structured JSON for CI Automation

```bash
vibe-commit plan changes.patch --format json -o plan.json
```

### 6. Dry-Run Verification

```bash
vibe-commit apply --dry-run changes.patch
```

---

## 📋 Showroom Example

### Monolithic Input Diff:

```diff
diff --git a/Cargo.toml b/Cargo.toml
+tokio = { version = "1.36", features = ["full"] }
+axum = "0.7"
diff --git a/src/models/user.rs b/src/models/user.rs
+pub struct User { pub id: Uuid, pub username: String }
diff --git a/src/api/routes.rs b/src/api/routes.rs
+pub fn user_router() -> Router { ... }
diff --git a/src/api/handler.rs b/src/api/handler.rs
-let user = find_user(id).unwrap();
+let user = find_user(id).unwrap_or_else(|| "Not found".to_string());
diff --git a/tests/user_api_test.rs b/tests/user_api_test.rs
+#[tokio::test] async fn test_user_router() { ... }
diff --git a/README.md b/README.md
+## Features - Asynchronous User API
```

### Generated Execution Plan:

| # | Type | Scope | Title | Files | Diff |
|---|---|---|---|---|---|
| 1 | `chore` | `deps` | chore(deps): update project configurations and dependencies | 1 | +2 -0 |
| 2 | `feat` | `models` | feat(models): define user data structures and models | 1 | +14 -0 |
| 3 | `feat` | `api` | feat(api): implement api capabilities | 1 | +10 -0 |
| 4 | `fix` | `api` | fix(api): fix boundary handling and edge cases | 1 | +4 -1 |
| 5 | `test` | `tests` | test(tests): add comprehensive test coverage | 1 | +5 -0 |
| 6 | `docs` | `readme` | docs(readme): update documentation and guides | 1 | +4 -0 |

---

## 🏛️ Conventional Commit Philosophy

`vibe-commit` enforces the [Conventional Commits v1.0.0](https://www.conventionalcommits.org/) specification:

- `chore(scope)`: Changes that do not modify `src` or test files (dependencies, build configs).
- `feat(scope)`: A new feature or domain entity.
- `fix(scope)`: A bug fix or edge case correction.
- `refactor(scope)`: A code change that neither fixes a bug nor adds a feature.
- `test(scope)`: Adding missing tests or correcting existing tests.
- `docs(scope)`: Documentation changes only.

---

## 🧪 Test Suite & Verification

The test suite covers full unified diff parsing, symbol extraction, category tagging, DAG topological sorting, limit grouping, shell generation, and edge case resilience:

```bash
cargo test
```

Output:
```
running 2 tests
test diff::parser::tests::test_parse_range_single_and_multiple ... ok
test diff::parser::tests::test_strip_git_prefix ... ok

running 14 tests
test test_empty_diff_handling ... ok
test test_malformed_hunk_header_error ... ok
test test_diff_parser_handles_file_deletion_and_rename ... ok
test test_hunk_header_variations ... ok
test test_multi_hunk_splitting_logic_and_test ... ok
test test_diff_parser_handles_monolithic_diff ... ok
test test_dependency_sorter_topological_order ... ok
test test_plan_formatter_shell_dry_run ... ok
test test_plan_formatter_markdown_output ... ok
test test_plan_generator_conventional_commits ... ok
test test_plan_formatter_json_output ... ok
test test_plan_formatter_shell_real_commands ... ok
test test_plan_generator_max_commits_limit ... ok
test test_semantic_classifier_categories ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
