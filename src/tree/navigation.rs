#![allow(dead_code)]

use crate::tree::tree_node::{TreeNode, TreeNodeRef};
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub struct Navigation {
    pub root: TreeNodeRef,
    pub flat_list: Vec<TreeNodeRef>,
    pub selected: usize,
    pub show_hidden: bool,
    pub follow_symlinks: bool,
    path_to_index: HashMap<PathBuf, usize>,
}

impl Navigation {
    pub fn new(
        start_path: PathBuf,
        show_files: bool,
        show_hidden: bool,
        follow_symlinks: bool,
    ) -> Result<Self> {
        let mut root = TreeNode::new(start_path, 0)?;
        root.load_children(show_files, show_hidden, follow_symlinks)?;
        root.is_expanded = true;
        let root = Arc::new(Mutex::new(root));

        let mut nav = Self {
            root,
            flat_list: Vec::new(),
            selected: 0,
            show_hidden,
            follow_symlinks,
            path_to_index: HashMap::new(),
        };

        nav.rebuild_flat_list();
        Ok(nav)
    }

    pub fn rebuild_flat_list(&mut self) {
        self.flat_list.clear();
        self.path_to_index.clear();
        Self::collect_visible_nodes(&self.root, &mut self.flat_list);

        for (idx, node) in self.flat_list.iter().enumerate() {
            let path = node.lock().unwrap().path.clone();
            self.path_to_index.insert(path, idx);
        }
    }

    fn collect_visible_nodes(node: &TreeNodeRef, result: &mut Vec<TreeNodeRef>) {
        result.push(Arc::clone(node));

        let (is_expanded, children_count) = {
            let node_locked = node.lock().unwrap();
            (node_locked.is_expanded, node_locked.children.len())
        };

        if is_expanded {
            for i in 0..children_count {
                let child = Arc::clone(&node.lock().unwrap().children[i]);
                Self::collect_visible_nodes(&child, result);
            }
        }
    }

    pub fn get_selected_node(&self) -> Option<TreeNodeRef> {
        self.flat_list.get(self.selected).map(Arc::clone)
    }

    pub fn move_down(&mut self) {
        if self.selected < self.flat_list.len().saturating_sub(1) {
            self.selected += 1;
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn toggle_node(&mut self, path: &Path, show_files: bool) -> Result<Option<String>> {
        if let Some(index) = self.path_to_index.get(path).copied() {
            if index < self.flat_list.len() {
                let node = &self.flat_list[index];
                let was_expanded = node.lock().unwrap().is_expanded;

                let error_msg = {
                    let mut node_locked = node.lock().unwrap();
                    node_locked.toggle_expand(
                        show_files,
                        self.show_hidden,
                        self.follow_symlinks,
                    )?;
                    if node_locked.has_error {
                        node_locked.error_message.clone()
                    } else {
                        None
                    }
                };

                let is_expanded = node.lock().unwrap().is_expanded;

                if was_expanded && !is_expanded {
                    self.remove_descendants_from_flat_list(index);
                } else if !was_expanded && is_expanded {
                    self.insert_children_into_flat_list(index);
                }

                return Ok(error_msg);
            }
        }

        let error_msg = Self::toggle_node_recursive(
            &self.root,
            path,
            show_files,
            self.show_hidden,
            self.follow_symlinks,
        )?;
        self.rebuild_flat_list();
        Ok(error_msg)
    }

    fn toggle_node_recursive(
        node: &TreeNodeRef,
        target_path: &Path,
        show_files: bool,
        show_hidden: bool,
        follow_symlinks: bool,
    ) -> Result<Option<String>> {
        {
            let mut node_locked = node.lock().unwrap();
            if node_locked.path == target_path {
                node_locked.toggle_expand(show_files, show_hidden, follow_symlinks)?;
                let error_msg = if node_locked.has_error {
                    node_locked.error_message.clone()
                } else {
                    None
                };
                return Ok(error_msg);
            }
        }

        let children_count = node.lock().unwrap().children.len();
        for i in 0..children_count {
            let child = Arc::clone(&node.lock().unwrap().children[i]);
            if let Some(error_msg) = Self::toggle_node_recursive(
                &child,
                target_path,
                show_files,
                show_hidden,
                follow_symlinks,
            )? {
                return Ok(Some(error_msg));
            }
        }

        Ok(None)
    }

    pub fn reload_tree(&mut self, show_files: bool) -> Result<()> {
        Self::reload_node_recursive(
            &self.root,
            show_files,
            self.show_hidden,
            self.follow_symlinks,
        )?;
        self.rebuild_flat_list();
        Ok(())
    }

    fn reload_node_recursive(
        node: &TreeNodeRef,
        show_files: bool,
        show_hidden: bool,
        follow_symlinks: bool,
    ) -> Result<()> {
        let should_reload = {
            let node_locked = node.lock().unwrap();
            node_locked.is_expanded && node_locked.is_dir
        };

        if should_reload {
            {
                let mut node_locked = node.lock().unwrap();
                node_locked.children.clear();
                node_locked.load_children(show_files, show_hidden, follow_symlinks)?;
            }

            let children_count = node.lock().unwrap().children.len();
            for i in 0..children_count {
                let child = Arc::clone(&node.lock().unwrap().children[i]);
                Self::reload_node_recursive(&child, show_files, show_hidden, follow_symlinks)?;
            }
        }
        Ok(())
    }

    pub fn go_to_parent(&mut self, show_files: bool) -> Result<()> {
        let parent_path = {
            let root_locked = self.root.lock().unwrap();
            root_locked.path.parent().map(|p| p.to_path_buf())
        };

        if let Some(parent_path) = parent_path {
            let current_path = self.root.lock().unwrap().path.clone();

            let mut new_root = TreeNode::new(parent_path, 0)?;
            new_root.load_children(show_files, self.show_hidden, self.follow_symlinks)?;
            new_root.is_expanded = true;

            self.root = Arc::new(Mutex::new(new_root));
            self.rebuild_flat_list();

            if let Some(&idx) = self.path_to_index.get(&current_path) {
                self.selected = idx;
            }
        }

        Ok(())
    }

    pub fn expand_path_to_node(&mut self, target_path: &PathBuf, show_files: bool) -> Result<()> {
        Self::expand_path_recursive(
            &self.root,
            target_path,
            show_files,
            self.show_hidden,
            self.follow_symlinks,
        )?;
        self.rebuild_flat_list();

        if let Some(&idx) = self.path_to_index.get(target_path) {
            self.selected = idx;
        }

        Ok(())
    }

    fn expand_path_recursive(
        node: &TreeNodeRef,
        target_path: &PathBuf,
        show_files: bool,
        show_hidden: bool,
        follow_symlinks: bool,
    ) -> Result<bool> {
        {
            let mut node_locked = node.lock().unwrap();

            if &node_locked.path == target_path {
                return Ok(true);
            }

            if !target_path.starts_with(&node_locked.path) {
                return Ok(false);
            }

            if node_locked.children.is_empty() && node_locked.is_dir {
                node_locked.load_children(show_files, show_hidden, follow_symlinks)?;
            }

            node_locked.is_expanded = true;
        }

        let children_count = node.lock().unwrap().children.len();
        for i in 0..children_count {
            let child = Arc::clone(&node.lock().unwrap().children[i]);
            if Self::expand_path_recursive(
                &child,
                target_path,
                show_files,
                show_hidden,
                follow_symlinks,
            )? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn remove_descendants_from_flat_list(&mut self, parent_index: usize) {
        let parent_depth = self.flat_list[parent_index].lock().unwrap().depth;

        let mut remove_count = 0;
        for i in (parent_index + 1)..self.flat_list.len() {
            if self.flat_list[i].lock().unwrap().depth > parent_depth {
                remove_count += 1;
            } else {
                break;
            }
        }

        if remove_count > 0 {
            self.flat_list
                .drain((parent_index + 1)..(parent_index + 1 + remove_count));
        }

        self.rebuild_path_index();
    }

    fn insert_children_into_flat_list(&mut self, parent_index: usize) {
        let node = &self.flat_list[parent_index];

        let mut new_nodes = Vec::new();
        let (is_expanded, children_count) = {
            let node_locked = node.lock().unwrap();
            (node_locked.is_expanded, node_locked.children.len())
        };

        if is_expanded {
            for i in 0..children_count {
                let child = Arc::clone(&node.lock().unwrap().children[i]);
                Self::collect_visible_nodes(&child, &mut new_nodes);
            }
        }

        if !new_nodes.is_empty() {
            let insert_pos = parent_index + 1;
            self.flat_list.splice(insert_pos..insert_pos, new_nodes);
        }

        self.rebuild_path_index();
    }

    fn rebuild_path_index(&mut self) {
        self.path_to_index.clear();
        for (idx, node) in self.flat_list.iter().enumerate() {
            let path = node.lock().unwrap().path.clone();
            self.path_to_index.insert(path, idx);
        }
    }
}
