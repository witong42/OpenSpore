use crate::Watchman;
use std::path::Path;
use tracing::info;

impl Watchman {
    /// Load additional ignore rules from .watchmanignore
    pub fn load_ignore_rules(&mut self) {
        let ignore_file = self.project_root.join(".watchmanignore");
        if ignore_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&ignore_file) {
                for line in content.lines() {
                    let line = line.trim();
                    if !line.is_empty() && !line.starts_with('#') {
                        self.ignore_rules.insert(line.to_string());
                    }
                }
            }
        }
        info!("👀 Watchman Ignore Rules: {:?}", self.ignore_rules);
    }

    /// Check if a file path should be ignored
    pub(crate) fn should_ignore(&self, path: &Path) -> bool {
        let rel_path = path.strip_prefix(&self.project_root).unwrap_or(path);
        let rel_path_str = rel_path.to_string_lossy();

        // Check each ignore rule
        for rule in &self.ignore_rules {
            // Handle glob patterns (e.g., *.log)
            if rule.starts_with('*') {
                let pattern = &rule[1..]; // Remove the *
                if let Some(filename) = path.file_name() {
                    if filename.to_string_lossy().ends_with(pattern) {
                        return true;
                    }
                }
            }
            // Handle path patterns (e.g., workspace/context)
            else if rule.contains('/') {
                if rel_path_str.starts_with(rule) || rel_path_str.contains(&format!("/{}", rule)) {
                    return true;
                }
            }
            // Handle directory/file name patterns
            else {
                for component in rel_path.components() {
                    if let std::path::Component::Normal(name) = component {
                        if name.to_string_lossy().as_ref() == rule {
                            return true;
                        }
                    }
                }
            }
        }

        // Check extension allowlist
        if let Some(ext) = path.extension() {
            let ext_str = format!(".{}", ext.to_string_lossy().to_lowercase());
            if !self.allowed_extensions.contains(&ext_str) {
                return true;
            }
        } else {
            return true; // No extension = ignore
        }

        false
    }
}
