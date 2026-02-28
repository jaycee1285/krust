#![allow(clippy::too_many_arguments)]

use crate::tree::tree_node::TreeNodeRef;
use crossbeam_channel::{bounded, unbounded, Receiver, Sender};
use std::path::PathBuf;
use std::thread::{self, JoinHandle};

#[derive(Debug, Clone)]
pub enum SearchMessage {
    Result(PathBuf, bool, Option<i64>, Option<Vec<usize>>),
    Progress(usize),
    Done,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub path: PathBuf,
    pub is_dir: bool,
    pub score: Option<i64>,
    pub match_indices: Option<Vec<usize>>,
}

pub struct Search {
    pub mode: bool,
    pub query: String,
    pub fuzzy_mode: bool,
    pub results: Vec<SearchResult>,
    pub selected: usize,
    pub show_results: bool,
    pub focus_on_results: bool,
    pub is_searching: bool,
    pub scanned_count: usize,
    search_thread: Option<JoinHandle<()>>,
    cancel_sender: Option<Sender<()>>,
    result_receiver: Option<Receiver<SearchMessage>>,
}

impl Default for Search {
    fn default() -> Self {
        Self::new()
    }
}

impl Search {
    pub fn new() -> Self {
        Self {
            mode: false,
            query: String::new(),
            fuzzy_mode: false,
            results: Vec::new(),
            selected: 0,
            show_results: false,
            focus_on_results: false,
            is_searching: false,
            scanned_count: 0,
            search_thread: None,
            cancel_sender: None,
            result_receiver: None,
        }
    }

    pub fn enter_mode(&mut self) {
        self.mode = true;
        self.query.clear();
        self.fuzzy_mode = false;
    }

    pub fn exit_mode(&mut self) {
        self.mode = false;
        self.query.clear();
        self.fuzzy_mode = false;
    }

    pub fn add_char(&mut self, c: char) {
        self.query.push(c);
        self.update_fuzzy_mode();
    }

    pub fn backspace(&mut self) {
        self.query.pop();
        self.update_fuzzy_mode();
    }

    fn update_fuzzy_mode(&mut self) {
        self.fuzzy_mode = self.query.starts_with('/');
    }

    fn get_search_query(&self) -> &str {
        if self.fuzzy_mode && self.query.len() > 1 {
            &self.query[1..]
        } else if self.fuzzy_mode {
            ""
        } else {
            &self.query
        }
    }

    pub fn perform_search(
        &mut self,
        root: &TreeNodeRef,
        show_files: bool,
        show_hidden: bool,
        follow_symlinks: bool,
    ) {
        self.cancel_search();

        self.results.clear();
        self.selected = 0;
        self.scanned_count = 0;

        let search_query = self.get_search_query();

        if search_query.is_empty() {
            self.show_results = false;
            self.is_searching = false;
            return;
        }

        let query_lower = search_query.to_lowercase();
        let is_fuzzy = self.fuzzy_mode;

        self.search_loaded_nodes(root, &query_lower, show_files, show_hidden, is_fuzzy);

        self.spawn_deep_search(
            root,
            query_lower,
            show_files,
            show_hidden,
            follow_symlinks,
            is_fuzzy,
        );

        self.show_results = true;
        self.focus_on_results = true;
        self.mode = false;
        self.is_searching = true;
    }

    fn search_loaded_nodes(
        &mut self,
        node: &TreeNodeRef,
        query: &str,
        show_files: bool,
        show_hidden: bool,
        fuzzy: bool,
    ) {
        use fuzzy_matcher::skim::SkimMatcherV2;
        use fuzzy_matcher::FuzzyMatcher;

        let node_locked = node.lock().unwrap();
        let name_lower = node_locked.name.to_lowercase();

        let is_hidden = node_locked.name.starts_with('.');
        if !show_hidden && is_hidden {
            return;
        }

        if show_files || node_locked.is_dir {
            if fuzzy {
                let matcher = SkimMatcherV2::default();
                if let Some((score, indices)) = matcher.fuzzy_indices(&name_lower, query) {
                    self.results.push(SearchResult {
                        path: node_locked.path.clone(),
                        is_dir: node_locked.is_dir,
                        score: Some(score),
                        match_indices: Some(indices),
                    });
                }
            } else if name_lower.contains(query) {
                self.results.push(SearchResult {
                    path: node_locked.path.clone(),
                    is_dir: node_locked.is_dir,
                    score: None,
                    match_indices: None,
                });
            }
        }

        if node_locked.is_expanded {
            let children = node_locked.children.clone();
            drop(node_locked);

            for child in &children {
                self.search_loaded_nodes(child, query, show_files, show_hidden, fuzzy);
            }
        }
    }

    fn spawn_deep_search(
        &mut self,
        root: &TreeNodeRef,
        query: String,
        show_files: bool,
        show_hidden: bool,
        follow_symlinks: bool,
        fuzzy: bool,
    ) {
        let (result_tx, result_rx) = unbounded();
        let (cancel_tx, cancel_rx) = bounded(1);

        let root_path = root.lock().unwrap().path.clone();

        let handle = thread::spawn(move || {
            Self::deep_search_recursive(
                &root_path,
                &query,
                &result_tx,
                &cancel_rx,
                show_files,
                show_hidden,
                follow_symlinks,
                fuzzy,
                &mut 0,
            );
            let _ = result_tx.send(SearchMessage::Done);
        });

        self.search_thread = Some(handle);
        self.cancel_sender = Some(cancel_tx);
        self.result_receiver = Some(result_rx);
    }

    fn deep_search_recursive(
        path: &PathBuf,
        query: &str,
        result_tx: &Sender<SearchMessage>,
        cancel_rx: &Receiver<()>,
        show_files: bool,
        show_hidden: bool,
        follow_symlinks: bool,
        fuzzy: bool,
        scanned: &mut usize,
    ) {
        use fuzzy_matcher::skim::SkimMatcherV2;
        use fuzzy_matcher::FuzzyMatcher;

        if cancel_rx.try_recv().is_ok() {
            return;
        }

        if !follow_symlinks {
            if let Ok(metadata) = std::fs::symlink_metadata(path) {
                if metadata.is_symlink() {
                    return;
                }
            }
        }

        let is_dir = path.is_dir();

        if !is_dir && !show_files {
            return;
        }

        if !show_hidden {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.') {
                    return;
                }
            }
        }

        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            let name_lower = name.to_lowercase();

            if fuzzy {
                let matcher = SkimMatcherV2::default();
                if let Some((score, indices)) = matcher.fuzzy_indices(&name_lower, query) {
                    let _ = result_tx.send(SearchMessage::Result(
                        path.clone(),
                        is_dir,
                        Some(score),
                        Some(indices),
                    ));
                }
            } else if name_lower.contains(query) {
                let _ = result_tx.send(SearchMessage::Result(path.clone(), is_dir, None, None));
            }
        }

        if is_dir {
            *scanned += 1;

            if *scanned % 100 == 0 {
                let _ = result_tx.send(SearchMessage::Progress(*scanned));
            }

            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    if cancel_rx.try_recv().is_ok() {
                        return;
                    }

                    let child_path = entry.path();
                    Self::deep_search_recursive(
                        &child_path,
                        query,
                        result_tx,
                        cancel_rx,
                        show_files,
                        show_hidden,
                        follow_symlinks,
                        fuzzy,
                        scanned,
                    );
                }
            }
        }
    }

    pub fn poll_results(&mut self) -> bool {
        let mut has_updates = false;
        let mut search_done = false;

        if let Some(ref rx) = self.result_receiver {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    SearchMessage::Result(path, is_dir, score, match_indices) => {
                        if !self.results.iter().any(|r| r.path == path) {
                            self.results.push(SearchResult {
                                path,
                                is_dir,
                                score,
                                match_indices,
                            });
                            has_updates = true;
                        }
                    }
                    SearchMessage::Progress(count) => {
                        self.scanned_count = count;
                        has_updates = true;
                    }
                    SearchMessage::Done => {
                        search_done = true;
                        has_updates = true;
                    }
                }
            }
        }

        if search_done {
            self.is_searching = false;
            self.search_thread = None;
            self.cancel_sender = None;
            self.result_receiver = None;

            if self.fuzzy_mode {
                self.results.sort_by(|a, b| match (a.score, b.score) {
                    (Some(score_a), Some(score_b)) => score_b.cmp(&score_a),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                });
            }
        }

        has_updates
    }

    pub fn cancel_search(&mut self) {
        if let Some(cancel_tx) = self.cancel_sender.take() {
            let _ = cancel_tx.send(());
        }

        if let Some(_handle) = self.search_thread.take() {
            // Thread detaches and terminates on its own via cancel_rx
        }

        self.result_receiver = None;
        self.is_searching = false;
    }

    pub fn move_down(&mut self) {
        if self.selected < self.results.len().saturating_sub(1) {
            self.selected += 1;
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn get_selected_result(&self) -> Option<PathBuf> {
        self.results.get(self.selected).map(|r| r.path.clone())
    }

    pub fn close_results(&mut self) {
        self.cancel_search();
        self.show_results = false;
        self.results.clear();
        self.selected = 0;
        self.focus_on_results = false;
        self.scanned_count = 0;
    }
}

impl Drop for Search {
    fn drop(&mut self) {
        self.cancel_search();
    }
}
