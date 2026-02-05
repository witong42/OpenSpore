use crate::MemorySystem;
use std::process::Command;

impl MemorySystem {
    pub fn init_git(&self) {
        if !self.memory_root.join(".git").exists() {
            let _ = Command::new("git")
                .arg("init")
                .current_dir(&self.memory_root)
                .output();
            let _ = Command::new("git")
                .args(["add", "."])
                .current_dir(&self.memory_root)
                .output();
            let _ = Command::new("git")
                .args(["commit", "-m", "Initial memory snapshot"])
                .current_dir(&self.memory_root)
                .output();
        }
    }

    pub fn commit(&self, message: &str) {
        let _ = Command::new("git")
            .args(["add", "."])
            .current_dir(&self.memory_root)
            .output();
        let _ = Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(&self.memory_root)
            .output();
    }
}
