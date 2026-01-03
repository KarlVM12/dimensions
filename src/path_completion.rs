use std::path::{Path, PathBuf};
use std::fs;
use std::env;

pub struct PathCompleter;

impl PathCompleter {
    /// Get directory completions for a partial path input
    /// Returns (candidates, common_prefix)
    pub fn complete_directory(input: &str) -> (Vec<String>, String) {
        let expanded = Self::expand_path(input);
        let path = Path::new(&expanded);

        let (search_dir, prefix) = if input.ends_with('/') {
            // User typed trailing slash, search in that directory
            (path.to_path_buf(), String::new())
        } else {
            // Split into parent directory and filename prefix
            let parent = path.parent().unwrap_or(Path::new("."));
            let filename = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            (parent.to_path_buf(), filename)
        };

        let mut candidates = Vec::new();

        // Read directory and find matching entries
        if let Ok(entries) = fs::read_dir(&search_dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    // Only include directories
                    if !metadata.is_dir() {
                        continue;
                    }
                }

                if let Some(name) = entry.file_name().to_str() {
                    // Skip hidden files unless user explicitly typed '.'
                    if name.starts_with('.') && !prefix.starts_with('.') {
                        continue;
                    }

                    // Check if this entry matches the prefix
                    if name.starts_with(&prefix) {
                        // Build full path for the candidate
                        let full_path = if search_dir == Path::new(".") {
                            name.to_string()
                        } else {
                            search_dir.join(name)
                                .to_str()
                                .unwrap_or(name)
                                .to_string()
                        };

                        candidates.push(full_path);
                    }
                }
            }
        }

        // Sort candidates alphabetically
        candidates.sort();

        // Compute common prefix for all candidates
        let common_prefix = Self::common_prefix(&candidates);

        (candidates, common_prefix)
    }

    /// Expand ~ and environment variables in path
    fn expand_path(input: &str) -> String {
        if input.is_empty() {
            return String::from(".");
        }

        let expanded = if input.starts_with("~/") {
            if let Ok(home) = env::var("HOME") {
                input.replacen("~", &home, 1)
            } else {
                input.to_string()
            }
        } else if input.starts_with('~') {
            // Handle ~username - just return as-is (complex to expand)
            input.to_string()
        } else if input.starts_with('$') {
            // Try to expand environment variable
            if let Some(var_end) = input.find('/').or(Some(input.len())) {
                let var_name = &input[1..var_end];
                if let Ok(value) = env::var(var_name) {
                    format!("{}{}", value, &input[var_end..])
                } else {
                    input.to_string()
                }
            } else {
                input.to_string()
            }
        } else {
            input.to_string()
        };

        // Convert to absolute path if relative
        if expanded.starts_with("./") || expanded.starts_with("../") || !expanded.starts_with('/') {
            if let Ok(current_dir) = env::current_dir() {
                return current_dir.join(&expanded)
                    .to_str()
                    .unwrap_or(&expanded)
                    .to_string();
            }
        }

        expanded
    }

    /// Find the longest common prefix among a set of strings
    fn common_prefix(candidates: &[String]) -> String {
        if candidates.is_empty() {
            return String::new();
        }

        if candidates.len() == 1 {
            return candidates[0].clone();
        }

        let first = &candidates[0];
        let mut prefix_len = 0;

        'outer: for (i, ch) in first.chars().enumerate() {
            for candidate in candidates.iter().skip(1) {
                if let Some(candidate_ch) = candidate.chars().nth(i) {
                    if candidate_ch != ch {
                        break 'outer;
                    }
                } else {
                    break 'outer;
                }
            }
            prefix_len = i + 1;
        }

        first.chars().take(prefix_len).collect()
    }

    /// Validate that a path exists and is a directory
    pub fn validate_directory(input: &str) -> Result<PathBuf, String> {
        if input.trim().is_empty() {
            return Err("Directory path cannot be empty".to_string());
        }

        let expanded = Self::expand_path(input);
        let path = PathBuf::from(expanded);

        if !path.exists() {
            return Err(format!("Directory does not exist: {}", input));
        }

        if !path.is_dir() {
            return Err(format!("Path is not a directory: {}", input));
        }

        Ok(path)
    }
}
