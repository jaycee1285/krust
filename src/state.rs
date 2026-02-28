use crate::tree::file_ops::TreeClipboard;
use r3bl_tui::{
    DialogBuffer, EditorBuffer, FlexBoxId, HasDialogBuffers, HasEditorBuffers,
};
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::path::PathBuf;

/// Main application state.
/// Navigation lives in AppMain (not here) because Rc<RefCell<>> can't derive Clone/PartialEq.
/// Pre-rendered tree lines are stored here for the component to render.
#[derive(Clone, PartialEq)]
pub struct State {
    pub editor_buffers: HashMap<FlexBoxId, EditorBuffer>,
    pub dialog_buffers: HashMap<FlexBoxId, DialogBuffer>,
    pub sidebar_visible: bool,
    pub tree_root_path: PathBuf,
    pub show_files: bool,
    pub show_hidden: bool,
    pub follow_symlinks: bool,
    /// Pre-rendered tree lines for the tree component to display.
    pub tree_render_lines: Option<Vec<String>>,
    pub tree_scroll_offset: usize,
    pub clipboard: Option<TreeClipboard>,
    /// Whether ".." is the currently selected entry in the tree
    pub dotdot_selected: bool,
    /// Display string for current tree root (e.g. "~/repos/krust")
    pub tree_root_display: String,
    /// Currently open filename for status bar display
    pub current_file_name: Option<String>,
    /// Whether the editor buffer has been modified since last save
    pub is_dirty: bool,
    /// When true, the next input event is silently consumed to force a re-render
    /// after a layout switch.
    pub eat_next_input: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            editor_buffers: HashMap::new(),
            dialog_buffers: HashMap::new(),
            sidebar_visible: false,
            tree_root_path: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            show_files: true,
            show_hidden: false,
            follow_symlinks: false,
            tree_render_lines: None,
            tree_scroll_offset: 0,
            clipboard: None,
            dotdot_selected: false,
            tree_root_display: abbreviate_path(
                &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            ),
            current_file_name: None,
            is_dirty: false,
            eat_next_input: false,
        }
    }
}

/// Abbreviate a path, replacing $HOME with ~
pub fn abbreviate_path(path: &std::path::Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(stripped) = path.strip_prefix(&home) {
            return format!("~/{}", stripped.display());
        }
    }
    path.display().to_string()
}

impl Debug for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "State[editors={}, dialogs={}, sidebar={}]",
            self.editor_buffers.len(),
            self.dialog_buffers.len(),
            self.sidebar_visible,
        )
    }
}

impl Display for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "State[editors={}, dialogs={}, sidebar={}]",
            self.editor_buffers.len(),
            self.dialog_buffers.len(),
            self.sidebar_visible,
        )
    }
}

impl HasEditorBuffers for State {
    fn get_mut_editor_buffer(&mut self, id: FlexBoxId) -> Option<&mut EditorBuffer> {
        self.editor_buffers.get_mut(&id)
    }

    fn insert_editor_buffer(&mut self, id: FlexBoxId, buffer: EditorBuffer) {
        self.editor_buffers.insert(id, buffer);
    }

    fn contains_editor_buffer(&self, id: FlexBoxId) -> bool {
        self.editor_buffers.contains_key(&id)
    }
}

impl HasDialogBuffers for State {
    fn get_mut_dialog_buffer(&mut self, id: FlexBoxId) -> Option<&mut DialogBuffer> {
        self.dialog_buffers.get_mut(&id)
    }
}

pub mod constructor {
    use super::*;
    use crate::app::Id;
    use r3bl_tui::DEFAULT_SYN_HI_FILE_EXT;
    use std::ffi::OsStr;
    use std::path::Path;

    pub fn new(maybe_file_path: Option<&str>) -> State {
        let mut state = State::default();

        let editor_buffer = {
            let file_ext = get_file_extension(maybe_file_path);
            let mut editor_buffer =
                EditorBuffer::new_empty(Some(&file_ext), maybe_file_path);

            let content = read_file_content(maybe_file_path);
            editor_buffer.init_with(content.lines());
            editor_buffer
        };

        state
            .editor_buffers
            .insert(FlexBoxId::from(Id::ComponentEditor), editor_buffer);

        // Set current filename for status bar
        if let Some(fp) = maybe_file_path {
            state.current_file_name = Some(
                std::path::Path::new(fp)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(fp)
                    .to_string(),
            );
        }

        state
    }

    pub fn get_file_extension(maybe_file_path: Option<&str>) -> String {
        if let Some(file_path) = maybe_file_path {
            let maybe_extension =
                Path::new(file_path).extension().and_then(OsStr::to_str);
            if let Some(extension) = maybe_extension {
                if !extension.is_empty() {
                    return extension.to_string();
                }
            }
        }
        DEFAULT_SYN_HI_FILE_EXT.to_string()
    }

    pub fn read_file_content(maybe_file_path: Option<&str>) -> String {
        if let Some(file_path) = maybe_file_path {
            match std::fs::read_to_string(file_path) {
                Ok(content) => return content,
                Err(e) => {
                    tracing::error!("Failed to read file {}: {}", file_path, e);
                }
            }
        }
        String::new()
    }
}
