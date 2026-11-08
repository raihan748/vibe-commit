use crate::chunker::classifier::SemanticChunk;
use std::collections::{HashMap, HashSet, VecDeque};

/// Dependency Graph and Topological Sorter for Semantic Chunks.
pub struct DependencySorter;

impl DependencySorter {
    /// Sorts chunks topologically according to semantic build dependencies and priority layers.
    pub fn sort(chunks: Vec<SemanticChunk>) -> Vec<SemanticChunk> {
        if chunks.len() <= 1 {
            return chunks;
        }

        let n = chunks.len();
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut in_degree: Vec<usize> = vec![0; n];

        // Map symbols defined in chunks
        let mut symbol_definitions: HashMap<String, usize> = HashMap::new();
        for (i, chunk) in chunks.iter().enumerate() {
            for sym in &chunk.symbols {
                if !sym.name.is_empty() {
                    symbol_definitions.insert(sym.name.clone(), i);
                }
            }
        }

        // Build directed edges: u -> v means u must come BEFORE v
        for i in 0..n {
            for j in 0..n {
                if i == j {
                    continue;
                }

                let mut depends_on_i = false;

                // Rule 1: Priority rank order (e.g. Chore=10 before Feat=30 before Test=60 before Docs=70)
                if chunks[i].priority_rank < chunks[j].priority_rank {
                    depends_on_i = true;
                }

                // Rule 2: Symbol dependency (if j mentions symbols defined in i)
                if !depends_on_i {
                    for sym in &chunks[j].symbols {
                        if sym.kind == crate::diff::types::SymbolKind::Import {
                            // Check if import mentions file or module of i
                            let mod_name = std::path::Path::new(&chunks[i].file_path)
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("");
                            if !mod_name.is_empty() && sym.name.contains(mod_name) {
                                depends_on_i = true;
                                break;
                            }
                        }
                    }
                }

                // Rule 3: File type hierarchy (lib/types before main/cli, code before tests)
                if !depends_on_i && chunks[i].priority_rank == chunks[j].priority_rank {
                    if is_core_type_file(&chunks[i].file_path) && !is_core_type_file(&chunks[j].file_path) {
                        depends_on_i = true;
                    } else if !is_entrypoint_file(&chunks[i].file_path) && is_entrypoint_file(&chunks[j].file_path) {
                        depends_on_i = true;
                    }
                }

                if depends_on_i {
                    adj[i].push(j);
                    in_degree[j] += 1;
                }
            }
        }

        // Kahn's algorithm with priority queue simulation (deterministic tie-breaking)
        let mut ready: Vec<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
        // Sort ready queue by priority rank, then file path
        ready.sort_by(|&a, &b| {
            chunks[a]
                .priority_rank
                .cmp(&chunks[b].priority_rank)
                .then_with(|| chunks[a].file_path.cmp(&chunks[b].file_path))
                .then_with(|| a.cmp(&b))
        });

        let mut queue: VecDeque<usize> = VecDeque::from(ready);
        let mut visited = HashSet::new();
        let mut sorted_indices = Vec::new();

        while let Some(u) = queue.pop_front() {
            if visited.contains(&u) {
                continue;
            }
            visited.insert(u);
            sorted_indices.push(u);

            let mut newly_ready = Vec::new();
            for &v in &adj[u] {
                if in_degree[v] > 0 {
                    in_degree[v] -= 1;
                    if in_degree[v] == 0 && !visited.contains(&v) {
                        newly_ready.push(v);
                    }
                }
            }

            // Sort newly ready deterministically
            newly_ready.sort_by(|&a, &b| {
                chunks[a]
                    .priority_rank
                    .cmp(&chunks[b].priority_rank)
                    .then_with(|| chunks[a].file_path.cmp(&chunks[b].file_path))
                    .then_with(|| a.cmp(&b))
            });

            for v in newly_ready {
                queue.push_back(v);
            }
        }

        // Fallback for any nodes involved in cycles
        if sorted_indices.len() < n {
            let mut remaining: Vec<usize> = (0..n).filter(|i| !visited.contains(i)).collect();
            remaining.sort_by(|&a, &b| {
                chunks[a]
                    .priority_rank
                    .cmp(&chunks[b].priority_rank)
                    .then_with(|| chunks[a].file_path.cmp(&chunks[b].file_path))
                    .then_with(|| a.cmp(&b))
            });
            sorted_indices.extend(remaining);
        }

        // Map back to chunks
        let mut chunks_map: HashMap<usize, SemanticChunk> = chunks.into_iter().enumerate().collect();
        sorted_indices
            .into_iter()
            .filter_map(|idx| chunks_map.remove(&idx))
            .collect()
    }
}

fn is_core_type_file(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.contains("type") || lower.contains("model") || lower.contains("schema") || lower.contains("entity")
}

fn is_entrypoint_file(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.ends_with("main.rs") || lower.ends_with("main.go") || lower.ends_with("index.ts") || lower.ends_with("index.js")
}
