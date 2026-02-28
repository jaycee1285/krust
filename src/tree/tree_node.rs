use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub type TreeNodeRef = Arc<Mutex<TreeNode>>;

pub struct TreeNode {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub is_expanded: bool,
    pub depth: usize,
    pub children: Vec<TreeNodeRef>,
    pub has_error: bool,
    pub error_message: Option<String>,
    is_sorted: bool,
}

impl TreeNode {
    pub fn new(path: PathBuf, depth: usize) -> Result<Self> {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let is_dir = path.is_dir();

        Ok(TreeNode {
            path,
            name,
            is_dir,
            is_expanded: false,
            depth,
            children: Vec::new(),
            has_error: false,
            error_message: None,
            is_sorted: false,
        })
    }

    pub fn load_children(
        &mut self,
        show_files: bool,
        show_hidden: bool,
        follow_symlinks: bool,
    ) -> Result<()> {
        if !self.is_dir || (!self.children.is_empty() && self.is_sorted) {
            return Ok(());
        }

        if !self.children.is_empty() {
            self.children.clear();
            self.is_sorted = false;
        }

        let entries = match fs::read_dir(&self.path) {
            Ok(entries) => entries,
            Err(e) => {
                self.has_error = true;
                self.error_message = Some(format!("Cannot read: {}", e));
                return Ok(());
            }
        };

        let mut error_count = 0;
        let mut skipped_entries = Vec::new();

        for entry in entries {
            match entry {
                Ok(entry) => {
                    let path = entry.path();

                    if !follow_symlinks {
                        if let Ok(metadata) = fs::symlink_metadata(&path) {
                            if metadata.is_symlink() {
                                continue;
                            }
                        }
                    }

                    let is_dir = path.is_dir();

                    if !show_hidden {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            if name.starts_with('.') {
                                continue;
                            }
                        }
                    }

                    if is_dir || show_files {
                        match TreeNode::new(path.clone(), self.depth + 1) {
                            Ok(node) => {
                                self.children.push(Arc::new(Mutex::new(node)));
                            }
                            Err(e) => {
                                error_count += 1;
                                skipped_entries.push(format!(
                                    "{}: {}",
                                    path.file_name().unwrap_or_default().to_string_lossy(),
                                    e
                                ));
                            }
                        }
                    }
                }
                Err(e) => {
                    error_count += 1;
                    skipped_entries.push(format!("unknown entry: {}", e));
                }
            }
        }

        if error_count > 0 {
            self.has_error = true;
            if error_count <= 3 {
                self.error_message = Some(skipped_entries.join(", "));
            } else {
                self.error_message = Some(format!("{} entries inaccessible", error_count));
            }
        }

        self.children.sort_by(|a, b| {
            let a_locked = a.lock().unwrap();
            let b_locked = b.lock().unwrap();
            match (a_locked.is_dir, b_locked.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a_locked.name.cmp(&b_locked.name),
            }
        });

        self.is_sorted = true;

        Ok(())
    }

    pub fn toggle_expand(
        &mut self,
        show_files: bool,
        show_hidden: bool,
        follow_symlinks: bool,
    ) -> Result<()> {
        if !self.is_dir {
            return Ok(());
        }

        if self.is_expanded {
            self.is_expanded = false;
        } else {
            self.load_children(show_files, show_hidden, follow_symlinks)?;
            if !self.has_error {
                self.is_expanded = true;
            }
        }

        Ok(())
    }
}
