use crate::state::State;
use crate::tree::component::TreeComponent;
use crate::tree::navigation::Navigation;
use r3bl_tui::{
    App, BoxedSafeApp, CommonResult, ComponentRegistry, ComponentRegistryMap,
    DialogBuffer, DialogChoice, DialogComponent, DialogEngineConfigOptions,
    DialogEngineMode, EditMode, EditorBuffer, EditorComponent, EditorEngineConfig,
    EventPropagation, FlexBox, FlexBoxId, GlobalData, HasEditorBuffers,
    HasFocus, InputEvent, Key, KeyPress, LayoutDirection, LayoutManagement, LengthOps,
    LineMode, ModifierKeysMask, PerformPositioningAndSizing, RenderOpCommon, RenderOpIR,
    RenderOpIRVec, RenderPipeline, RgbValue, SPACER_GLYPH, Size, Surface,
    SurfaceProps, SurfaceRender, SyntaxHighlightMode, TerminalWindowMainThreadSignal,
    TuiColor, TuiStylesheet, ZOrder, box_end, box_start, col, get_tui_style, height,
    new_style, render_component_in_current_box, render_component_in_given_box,
    render_tui_styled_texts_into, req_size_pc, row, send_signal, surface, throws,
    throws_with_return, tui_color, tui_styled_text, tui_styled_texts, tui_stylesheet,
};
use std::path::PathBuf;
use std::sync::Mutex;

/// Signals that can be sent to the app.
#[derive(Default, Clone, Debug)]
#[non_exhaustive]
pub enum AppSignal {
    SaveFile,
    AskForFilenameToSaveFile,
    OpenFile(PathBuf),
    ToggleSidebar,
    TreeMoveUp,
    TreeMoveDown,
    TreeExpand,
    TreeCollapse,
    TreeEnter,
    TreeGoToParent,
    MarkDirty,
    #[default]
    Noop,
}

/// Constants for the ids.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Id {
    Container = 1,
    ComponentEditor = 2,
    ComponentTree = 3,
    ComponentSaveDialog = 4,
    ComponentFocusDialog = 5,

    StyleContainer = 10,
    StyleEditorDefault = 11,
    StyleTreeDefault = 12,
    StyleDialogBorder = 13,
    StyleDialogTitle = 14,
    StyleDialogEditor = 15,
    StyleDialogResultsPanel = 16,
    StyleFocusDialogBorder = 17,
    StyleFocusDialogTitle = 18,
    StyleFocusDialogEditor = 19,
    StyleFocusDialogResultsPanel = 20,
}

mod id_impl {
    use super::{FlexBoxId, Id};

    impl From<Id> for u8 {
        fn from(id: Id) -> u8 {
            id as u8
        }
    }

    impl From<Id> for FlexBoxId {
        fn from(id: Id) -> FlexBoxId {
            FlexBoxId::new(id)
        }
    }
}

/// The main app struct. Holds the tree Navigation at runtime behind a Mutex
/// so AppMain can be Send+Sync as required by r3bl_tui.
pub struct AppMain {
    navigation: Mutex<Option<Navigation>>,
}

impl std::fmt::Debug for AppMain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppMain").finish()
    }
}

impl Default for AppMain {
    fn default() -> Self {
        Self {
            navigation: Mutex::new(None),
        }
    }
}

impl AppMain {
    pub fn new_boxed() -> BoxedSafeApp<State, AppSignal> {
        Box::new(Self::default())
    }

    fn with_navigation<F, R>(&self, state: &State, f: F) -> R
    where
        F: FnOnce(&mut Option<Navigation>) -> R,
    {
        let mut nav_guard = self.navigation.lock().unwrap();
        if nav_guard.is_none() {
            match Navigation::new(
                state.tree_root_path.clone(),
                state.show_files,
                state.show_hidden,
                state.follow_symlinks,
            ) {
                Ok(nav) => *nav_guard = Some(nav),
                Err(e) => tracing::error!("Failed to init tree navigation: {}", e),
            }
        }
        f(&mut nav_guard)
    }

    /// Build the pre-rendered tree lines from navigation state.
    fn build_tree_render_lines(nav: &Navigation, dotdot_selected: bool) -> Vec<String> {
        let mut lines = Vec::new();

        // Show ".." at top if root has a parent
        let has_parent = nav.root.lock().unwrap().path.parent().is_some();
        if has_parent {
            let marker = if dotdot_selected { ">" } else { " " };
            lines.push(format!("{}\u{f07b}  ..", marker));
        }

        for (idx, node_ref) in nav.flat_list.iter().enumerate() {
            let node = node_ref.lock().unwrap();
            let is_selected = !dotdot_selected && idx == nav.selected;

            let indent = "  ".repeat(node.depth);
            let icon = if node.is_dir {
                if node.is_expanded {
                    "\u{f07c} " // open folder
                } else {
                    "\u{f07b} " // closed folder
                }
            } else {
                "\u{f15b} " // file icon
            };

            let marker = if is_selected { ">" } else { " " };

            let line = format!("{}{}{}{}", marker, indent, icon, node.name);
            lines.push(line);
        }

        lines
    }

    fn has_dotdot(nav: &Navigation) -> bool {
        nav.root.lock().unwrap().path.parent().is_some()
    }
}

mod app_main_impl_app_trait {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl App for AppMain {
        type S = State;
        type AS = AppSignal;

        fn app_init(
            &mut self,
            component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            has_focus: &mut HasFocus,
        ) {
            populate_component_registry::create_components(
                component_registry_map,
                has_focus,
            );
        }

        fn app_handle_input_event(
            &mut self,
            input_event: InputEvent,
            global_data: &mut GlobalData<State, AppSignal>,
            component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            has_focus: &mut HasFocus,
        ) -> CommonResult<EventPropagation> {
            // Ctrl-T: toggle sidebar
            if input_event.matches_keypress(KeyPress::WithModifiers {
                key: Key::Character('t'),
                mask: ModifierKeysMask::new().with_ctrl(),
            }) {
                send_signal!(
                    global_data.main_thread_channel_sender,
                    TerminalWindowMainThreadSignal::ApplyAppSignal(
                        AppSignal::ToggleSidebar
                    )
                );
                return Ok(EventPropagation::Consumed);
            }

            // Ctrl-S: save file
            if input_event.matches_keypress(KeyPress::WithModifiers {
                key: Key::Character('s'),
                mask: ModifierKeysMask::new().with_ctrl(),
            }) {
                send_signal!(
                    global_data.main_thread_channel_sender,
                    TerminalWindowMainThreadSignal::ApplyAppSignal(AppSignal::SaveFile)
                );
                return Ok(EventPropagation::Consumed);
            }

            // Ctrl-F: focus tree
            if input_event.matches_keypress(KeyPress::WithModifiers {
                key: Key::Character('f'),
                mask: ModifierKeysMask::new().with_ctrl(),
            }) {
                if global_data.state.sidebar_visible {
                    has_focus.set_id(FlexBoxId::from(Id::ComponentTree));
                    return Ok(EventPropagation::ConsumedRender);
                }
                return Ok(EventPropagation::Consumed);
            }

            // Ctrl-E: focus editor
            if input_event.matches_keypress(KeyPress::WithModifiers {
                key: Key::Character('e'),
                mask: ModifierKeysMask::new().with_ctrl(),
            }) {
                has_focus.set_id(FlexBoxId::from(Id::ComponentEditor));
                return Ok(EventPropagation::ConsumedRender);
            }

            // Route to focused component
            ComponentRegistry::route_event_to_focused_component(
                global_data,
                input_event,
                component_registry_map,
                has_focus,
            )
        }

        fn app_handle_signal(
            &mut self,
            action: &AppSignal,
            global_data: &mut GlobalData<State, AppSignal>,
            component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            has_focus: &mut HasFocus,
        ) -> CommonResult<EventPropagation> {
            match action {
                AppSignal::ToggleSidebar => {
                    global_data.state.sidebar_visible = !global_data.state.sidebar_visible;
                    global_data.state.eat_next_input = true;

                    // Drop the saved offscreen buffer to force full repaint.
                    if let Some(old_buf) = global_data.maybe_saved_ofs_buf.take() {
                        global_data.offscreen_buffer_pool.give_back(old_buf);
                    }

                    if global_data.state.sidebar_visible {
                        let root_path = self.with_navigation(&global_data.state, |nav_opt| {
                            nav_opt.as_ref().map(|nav| {
                                nav.root.lock().unwrap().path.clone()
                            })
                        });
                        if let Some(path) = root_path {
                            global_data.state.tree_root_display =
                                crate::state::abbreviate_path(&path);
                        }
                        let dd = global_data.state.dotdot_selected;
                        let lines = self.with_navigation(&global_data.state, |nav_opt| {
                            nav_opt.as_ref().map(|nav| Self::build_tree_render_lines(nav, dd))
                        });
                        if let Some(lines) = lines {
                            global_data.state.tree_render_lines = Some(lines);
                            global_data.state.tree_scroll_offset = 0;
                        }
                    } else {
                        has_focus.set_id(FlexBoxId::from(Id::ComponentEditor));
                    }
                }

                AppSignal::TreeMoveUp => {
                    let dd = global_data.state.dotdot_selected;
                    if dd {
                        // Already at top, do nothing
                    } else {
                        // Check if nav.selected is 0 and we have a dotdot
                        let at_top = self.with_navigation(&global_data.state, |nav_opt| {
                            nav_opt.as_ref().map(|nav| {
                                (nav.selected == 0, Self::has_dotdot(nav))
                            })
                        });
                        if let Some((at_zero, has_dd)) = at_top {
                            if at_zero && has_dd {
                                // Move to dotdot
                                global_data.state.dotdot_selected = true;
                            } else {
                                // Normal move up
                                self.with_navigation(&global_data.state, |nav_opt| {
                                    if let Some(nav) = nav_opt.as_mut() {
                                        nav.move_up();
                                    }
                                });
                            }
                        }
                    }
                    let dd = global_data.state.dotdot_selected;
                    let lines = self.with_navigation(&global_data.state, |nav_opt| {
                        nav_opt.as_ref().map(|nav| Self::build_tree_render_lines(nav, dd))
                    });
                    if let Some(lines) = lines {
                        global_data.state.tree_render_lines = Some(lines);
                    }
                }

                AppSignal::TreeMoveDown => {
                    if global_data.state.dotdot_selected {
                        // Move off dotdot to first item (nav.selected should already be 0)
                        global_data.state.dotdot_selected = false;
                    } else {
                        self.with_navigation(&global_data.state, |nav_opt| {
                            if let Some(nav) = nav_opt.as_mut() {
                                nav.move_down();
                            }
                        });
                    }
                    let dd = global_data.state.dotdot_selected;
                    let lines = self.with_navigation(&global_data.state, |nav_opt| {
                        nav_opt.as_ref().map(|nav| Self::build_tree_render_lines(nav, dd))
                    });
                    if let Some(lines) = lines {
                        global_data.state.tree_render_lines = Some(lines);
                    }
                }

                AppSignal::TreeExpand => {
                    if global_data.state.dotdot_selected {
                        // dotdot doesn't expand, treat as enter (go to parent)
                        send_signal!(
                            global_data.main_thread_channel_sender,
                            TerminalWindowMainThreadSignal::ApplyAppSignal(
                                AppSignal::TreeGoToParent
                            )
                        );
                    } else {
                        let show_files = global_data.state.show_files;
                        let dd = global_data.state.dotdot_selected;
                        let lines = self.with_navigation(&global_data.state, |nav_opt| {
                            nav_opt.as_mut().and_then(|nav| {
                                let node = nav.get_selected_node()?;
                                let path = node.lock().unwrap().path.clone();
                                let is_dir = node.lock().unwrap().is_dir;
                                if is_dir && !node.lock().unwrap().is_expanded {
                                    let _ = nav.toggle_node(&path, show_files);
                                    Some(Self::build_tree_render_lines(nav, dd))
                                } else {
                                    None
                                }
                            })
                        });
                        if let Some(lines) = lines {
                            global_data.state.tree_render_lines = Some(lines);
                        }
                    }
                }

                AppSignal::TreeCollapse => {
                    if !global_data.state.dotdot_selected {
                        let show_files = global_data.state.show_files;
                        let dd = global_data.state.dotdot_selected;
                        let lines = self.with_navigation(&global_data.state, |nav_opt| {
                            nav_opt.as_mut().and_then(|nav| {
                                let node = nav.get_selected_node()?;
                                let path = node.lock().unwrap().path.clone();
                                if node.lock().unwrap().is_expanded {
                                    let _ = nav.toggle_node(&path, show_files);
                                    Some(Self::build_tree_render_lines(nav, dd))
                                } else {
                                    None
                                }
                            })
                        });
                        if let Some(lines) = lines {
                            global_data.state.tree_render_lines = Some(lines);
                        }
                    }
                }

                AppSignal::TreeEnter => {
                    if global_data.state.dotdot_selected {
                        // Enter on ".." — go to parent
                        send_signal!(
                            global_data.main_thread_channel_sender,
                            TerminalWindowMainThreadSignal::ApplyAppSignal(
                                AppSignal::TreeGoToParent
                            )
                        );
                    } else {
                        let show_files = global_data.state.show_files;
                        let dd = global_data.state.dotdot_selected;
                        let action_result = self.with_navigation(&global_data.state, |nav_opt| {
                            nav_opt.as_mut().and_then(|nav| {
                                let node = nav.get_selected_node()?;
                                let path = node.lock().unwrap().path.clone();
                                let is_dir = node.lock().unwrap().is_dir;

                                if is_dir {
                                    let _ = nav.toggle_node(&path, show_files);
                                    Some((true, Self::build_tree_render_lines(nav, dd), path))
                                } else {
                                    Some((false, Vec::new(), path))
                                }
                            })
                        });

                        if let Some((is_dir, lines, path)) = action_result {
                            if is_dir {
                                global_data.state.tree_render_lines = Some(lines);
                            } else {
                                send_signal!(
                                    global_data.main_thread_channel_sender,
                                    TerminalWindowMainThreadSignal::ApplyAppSignal(
                                        AppSignal::OpenFile(path)
                                    )
                                );
                            }
                        }
                    }
                }

                AppSignal::TreeGoToParent => {
                    let show_files = global_data.state.show_files;
                    global_data.state.dotdot_selected = false;
                    let result = self.with_navigation(&global_data.state, |nav_opt| {
                        nav_opt.as_mut().map(|nav| {
                            let _ = nav.go_to_parent(show_files);
                            let lines = Self::build_tree_render_lines(nav, false);
                            let root_path = nav.root.lock().unwrap().path.clone();
                            (lines, root_path)
                        })
                    });
                    if let Some((lines, root_path)) = result {
                        global_data.state.tree_render_lines = Some(lines);
                        global_data.state.tree_root_display =
                            crate::state::abbreviate_path(&root_path);
                    }
                }

                AppSignal::OpenFile(path) => {
                    let path_str = path.to_string_lossy().to_string();
                    let file_ext =
                        crate::state::constructor::get_file_extension(Some(&path_str));
                    let content =
                        crate::state::constructor::read_file_content(Some(&path_str));

                    let mut editor_buffer =
                        EditorBuffer::new_empty(Some(&file_ext), Some(&path_str));
                    editor_buffer.init_with(content.lines());

                    global_data.state.editor_buffers.insert(
                        FlexBoxId::from(Id::ComponentEditor),
                        editor_buffer,
                    );

                    // Update status bar info
                    global_data.state.current_file_name = Some(
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("untitled")
                            .to_string(),
                    );
                    global_data.state.is_dirty = false;

                    has_focus.set_id(FlexBoxId::from(Id::ComponentEditor));
                }

                AppSignal::SaveFile => {
                    let state = &global_data.state;
                    if let Some(editor_buffer) =
                        state.editor_buffers.get(&FlexBoxId::from(Id::ComponentEditor))
                    {
                        let maybe_file_path =
                            editor_buffer.content.maybe_file_path.clone();
                        let content = editor_buffer.get_as_string_with_newlines();

                        match maybe_file_path {
                            Some(file_path) => {
                                if let Err(e) = std::fs::write(&*file_path, &content) {
                                    tracing::error!("Failed to save file: {}", e);
                                } else {
                                    global_data.state.is_dirty = false;
                                }
                            }
                            None => {
                                if !editor_buffer.is_empty() {
                                    send_signal!(
                                        global_data.main_thread_channel_sender,
                                        TerminalWindowMainThreadSignal::ApplyAppSignal(
                                            AppSignal::AskForFilenameToSaveFile
                                        )
                                    );
                                }
                            }
                        }
                    }
                }

                AppSignal::AskForFilenameToSaveFile => {
                    ComponentRegistry::reset_component(
                        component_registry_map,
                        FlexBoxId::from(Id::ComponentSaveDialog),
                    );

                    let state = &mut global_data.state;
                    let new_dialog_buffer = {
                        let mut it = DialogBuffer::new_empty();
                        it.title = "File name or path to save content to:".into();
                        it.editor_buffer.init_with("".lines());
                        it
                    };
                    state.dialog_buffers.insert(
                        FlexBoxId::from(Id::ComponentSaveDialog),
                        new_dialog_buffer,
                    );

                    has_focus
                        .try_set_modal_id(FlexBoxId::from(Id::ComponentSaveDialog))
                        .ok();

                    return Ok(EventPropagation::ConsumedRender);
                }

                AppSignal::MarkDirty => {
                    global_data.state.is_dirty = true;
                }

                AppSignal::Noop => {}
            }

            Ok(EventPropagation::ConsumedRender)
        }

        fn app_render(
            &mut self,
            global_data: &mut GlobalData<State, AppSignal>,
            component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            has_focus: &mut HasFocus,
        ) -> CommonResult<RenderPipeline> {
            throws_with_return!({
                // Ensure tree render lines exist
                if global_data.state.tree_render_lines.is_none() {
                    let dd = global_data.state.dotdot_selected;
                    let lines = self.with_navigation(&global_data.state, |nav_opt| {
                        nav_opt.as_ref().map(|nav| Self::build_tree_render_lines(nav, dd))
                    });
                    if let Some(lines) = lines {
                        global_data.state.tree_render_lines = Some(lines);
                    }
                }

                let window_size = global_data.window_size;
                let surface_size = window_size.col_width
                    + (window_size.row_height - height(1));

                // Two completely separate layout paths. Each owns its
                // entire surface — no conditional branching inside layouts.
                let mut surface = if global_data.state.sidebar_visible {
                    // === TWO-COLUMN LAYOUT ===
                    let mut it = surface!(stylesheet: stylesheet::create_stylesheet()?);
                    it.surface_start(SurfaceProps {
                        pos: row(0) + col(0),
                        size: surface_size,
                    })?;
                    layout_two_col::TwoColLayout.render_in_surface(
                        &mut it,
                        global_data,
                        component_registry_map,
                        has_focus,
                    )?;
                    it.surface_end()?;
                    it
                } else {
                    // === SINGLE-COLUMN LAYOUT ===
                    let mut it = surface!(stylesheet: stylesheet::create_stylesheet()?);
                    it.surface_start(SurfaceProps {
                        pos: row(0) + col(0),
                        size: surface_size,
                    })?;
                    layout_single_col::SingleColLayout.render_in_surface(
                        &mut it,
                        global_data,
                        component_registry_map,
                        has_focus,
                    )?;
                    it.surface_end()?;
                    it
                };

                status_bar::render_status_bar(
                    &mut surface.render_pipeline,
                    window_size,
                    &global_data.state,
                );

                surface.render_pipeline
            });
        }
    }
}

/// Single-column layout: editor only, 100% width. Standalone edi-style.
mod layout_single_col {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub struct SingleColLayout;

    impl SurfaceRender<State, AppSignal> for SingleColLayout {
        fn render_in_surface(
            &mut self,
            surface: &mut Surface,
            global_data: &mut GlobalData<State, AppSignal>,
            component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            has_focus: &mut HasFocus,
        ) -> CommonResult<()> {
            throws!({
                let id_editor = FlexBoxId::from(Id::ComponentEditor);

                box_start!(
                    in: surface,
                    id: id_editor,
                    dir: LayoutDirection::Vertical,
                    requested_size_percent: req_size_pc!(width: 100, height: 100),
                    styles: [Id::StyleEditorDefault]
                );
                render_component_in_current_box!(
                    in: surface,
                    component_id: id_editor,
                    from: component_registry_map,
                    global_data: global_data,
                    has_focus: has_focus
                );
                box_end!(in: surface);

                // Modal dialog overlays
                if has_focus.is_modal_id(FlexBoxId::from(Id::ComponentSaveDialog)) {
                    render_component_in_given_box! {
                        in: surface,
                        box: FlexBox::default(),
                        component_id: FlexBoxId::from(Id::ComponentSaveDialog),
                        from: component_registry_map,
                        global_data: global_data,
                        has_focus: has_focus
                    };
                }
                if has_focus.is_modal_id(FlexBoxId::from(Id::ComponentFocusDialog)) {
                    render_component_in_given_box! {
                        in: surface,
                        box: FlexBox::default(),
                        component_id: FlexBoxId::from(Id::ComponentFocusDialog),
                        from: component_registry_map,
                        global_data: global_data,
                        has_focus: has_focus
                    };
                }
            });
        }
    }
}

/// Two-column layout: tree (25%) + editor (75%).
mod layout_two_col {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub struct TwoColLayout;

    impl SurfaceRender<State, AppSignal> for TwoColLayout {
        fn render_in_surface(
            &mut self,
            surface: &mut Surface,
            global_data: &mut GlobalData<State, AppSignal>,
            component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            has_focus: &mut HasFocus,
        ) -> CommonResult<()> {
            throws!({
                let id_container = FlexBoxId::from(Id::Container);
                let id_tree = FlexBoxId::from(Id::ComponentTree);
                let id_editor = FlexBoxId::from(Id::ComponentEditor);

                // Container
                box_start!(
                    in: surface,
                    id: id_container,
                    dir: LayoutDirection::Horizontal,
                    requested_size_percent: req_size_pc!(width: 100, height: 100),
                    styles: [Id::StyleContainer]
                );

                // Tree pane — 25% (left)
                {
                    box_start!(
                        in: surface,
                        id: id_tree,
                        dir: LayoutDirection::Vertical,
                        requested_size_percent: req_size_pc!(width: 25, height: 100),
                        styles: [Id::StyleTreeDefault]
                    );
                    render_component_in_current_box!(
                        in: surface,
                        component_id: id_tree,
                        from: component_registry_map,
                        global_data: global_data,
                        has_focus: has_focus
                    );
                    box_end!(in: surface);
                }

                // Editor pane — 75% (right)
                {
                    box_start!(
                        in: surface,
                        id: id_editor,
                        dir: LayoutDirection::Vertical,
                        requested_size_percent: req_size_pc!(width: 75, height: 100),
                        styles: [Id::StyleEditorDefault]
                    );
                    render_component_in_current_box!(
                        in: surface,
                        component_id: id_editor,
                        from: component_registry_map,
                        global_data: global_data,
                        has_focus: has_focus
                    );
                    box_end!(in: surface);
                }

                box_end!(in: surface);

                // Modal dialog overlays
                if has_focus.is_modal_id(FlexBoxId::from(Id::ComponentSaveDialog)) {
                    render_component_in_given_box! {
                        in: surface,
                        box: FlexBox::default(),
                        component_id: FlexBoxId::from(Id::ComponentSaveDialog),
                        from: component_registry_map,
                        global_data: global_data,
                        has_focus: has_focus
                    };
                }
                if has_focus.is_modal_id(FlexBoxId::from(Id::ComponentFocusDialog)) {
                    render_component_in_given_box! {
                        in: surface,
                        box: FlexBox::default(),
                        component_id: FlexBoxId::from(Id::ComponentFocusDialog),
                        from: component_registry_map,
                        global_data: global_data,
                        has_focus: has_focus
                    };
                }
            });
        }
    }
}

mod populate_component_registry {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub fn create_components(
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
        has_focus: &mut HasFocus,
    ) {
        insert_editor_component(component_registry_map);
        insert_tree_component(component_registry_map);
        insert_save_dialog_component(component_registry_map);
        insert_focus_dialog_component(component_registry_map);

        has_focus.set_id(FlexBoxId::from(Id::ComponentEditor));
    }

    fn insert_editor_component(
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
    ) {
        let id = FlexBoxId::from(Id::ComponentEditor);
        let boxed_editor_component = {
            #[allow(clippy::needless_pass_by_value)]
            fn on_buffer_change(
                my_id: FlexBoxId,
                main_thread_channel_sender: tokio::sync::mpsc::Sender<
                    TerminalWindowMainThreadSignal<AppSignal>,
                >,
            ) {
                send_signal!(
                    main_thread_channel_sender,
                    TerminalWindowMainThreadSignal::ApplyAppSignal(
                        AppSignal::MarkDirty
                    )
                );
                send_signal!(
                    main_thread_channel_sender,
                    TerminalWindowMainThreadSignal::Render(Some(my_id))
                );
            }

            let config_options = EditorEngineConfig::default();
            EditorComponent::new_boxed(id, config_options, on_buffer_change)
        };

        ComponentRegistry::put(component_registry_map, id, boxed_editor_component);
    }

    fn insert_tree_component(
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
    ) {
        let id = FlexBoxId::from(Id::ComponentTree);
        let component = TreeComponent::new_boxed(id);
        ComponentRegistry::put(component_registry_map, id, component);
    }

    fn insert_save_dialog_component(
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
    ) {
        let result_stylesheet = stylesheet::create_stylesheet();

        let dialog_options = DialogEngineConfigOptions {
            mode: DialogEngineMode::ModalSimple,
            maybe_style_border: get_tui_style! { @from_result: result_stylesheet, Id::StyleDialogBorder },
            maybe_style_title: get_tui_style! { @from_result: result_stylesheet, Id::StyleDialogTitle },
            maybe_style_editor: get_tui_style! { @from_result: result_stylesheet, Id::StyleDialogEditor },
            maybe_style_results_panel: get_tui_style! { @from_result: result_stylesheet, Id::StyleDialogResultsPanel },
            ..Default::default()
        };

        let editor_options = EditorEngineConfig {
            multiline_mode: LineMode::SingleLine,
            syntax_highlight: SyntaxHighlightMode::Disable,
            edit_mode: EditMode::ReadWrite,
        };

        let boxed_dialog_component = {
            fn on_dialog_press_handler(
                dialog_choice: DialogChoice,
                state: &mut State,
                main_thread_channel_sender: &mut tokio::sync::mpsc::Sender<
                    TerminalWindowMainThreadSignal<AppSignal>,
                >,
            ) {
                if let DialogChoice::Yes(text) = dialog_choice {
                    let user_input_file_path = text.trim();
                    if !user_input_file_path.is_empty() {
                        if let Some(editor_buffer) = state
                            .get_mut_editor_buffer(FlexBoxId::from(Id::ComponentEditor))
                        {
                            editor_buffer.content.maybe_file_path =
                                Some(user_input_file_path.into());

                            editor_buffer.content.maybe_file_extension = {
                                let ext = crate::state::constructor::get_file_extension(
                                    Some(user_input_file_path),
                                );
                                Some(ext.into())
                            };

                            send_signal!(
                                main_thread_channel_sender,
                                TerminalWindowMainThreadSignal::ApplyAppSignal(
                                    AppSignal::SaveFile
                                )
                            );
                        }
                    }
                }
            }

            fn on_dialog_editor_change_handler(
                _state: &mut State,
                _main_thread_channel_sender: &mut tokio::sync::mpsc::Sender<
                    TerminalWindowMainThreadSignal<AppSignal>,
                >,
            ) {
            }

            DialogComponent::new_boxed(
                FlexBoxId::from(Id::ComponentSaveDialog),
                dialog_options,
                editor_options,
                on_dialog_press_handler,
                on_dialog_editor_change_handler,
            )
        };

        ComponentRegistry::put(
            component_registry_map,
            FlexBoxId::from(Id::ComponentSaveDialog),
            boxed_dialog_component,
        );
    }

    fn insert_focus_dialog_component(
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
    ) {
        let result_stylesheet = stylesheet::create_stylesheet();

        let dialog_options = DialogEngineConfigOptions {
            mode: DialogEngineMode::ModalSimple,
            maybe_style_border: get_tui_style! { @from_result: result_stylesheet, Id::StyleFocusDialogBorder },
            maybe_style_title: get_tui_style! { @from_result: result_stylesheet, Id::StyleFocusDialogTitle },
            maybe_style_editor: get_tui_style! { @from_result: result_stylesheet, Id::StyleFocusDialogEditor },
            maybe_style_results_panel: get_tui_style! { @from_result: result_stylesheet, Id::StyleFocusDialogResultsPanel },
            ..Default::default()
        };

        let editor_options = EditorEngineConfig {
            multiline_mode: LineMode::SingleLine,
            syntax_highlight: SyntaxHighlightMode::Disable,
            edit_mode: EditMode::ReadWrite,
        };

        let boxed_dialog_component = {
            fn on_dialog_press_handler(
                dialog_choice: DialogChoice,
                _state: &mut State,
                main_thread_channel_sender: &mut tokio::sync::mpsc::Sender<
                    TerminalWindowMainThreadSignal<AppSignal>,
                >,
            ) {
                if let DialogChoice::Yes(_) = dialog_choice {
                    // Queue: toggle state, then force two render passes.
                    // The dialog dismiss itself is one keystroke. These
                    // signals fire after the dialog closes, each triggering
                    // a full event loop iteration with render + paint.
                    send_signal!(
                        main_thread_channel_sender,
                        TerminalWindowMainThreadSignal::ApplyAppSignal(
                            AppSignal::ToggleSidebar
                        )
                    );
                    send_signal!(
                        main_thread_channel_sender,
                        TerminalWindowMainThreadSignal::Render(None)
                    );
                    send_signal!(
                        main_thread_channel_sender,
                        TerminalWindowMainThreadSignal::Render(None)
                    );
                }
            }

            fn on_dialog_editor_change_handler(
                _state: &mut State,
                _main_thread_channel_sender: &mut tokio::sync::mpsc::Sender<
                    TerminalWindowMainThreadSignal<AppSignal>,
                >,
            ) {
            }

            DialogComponent::new_boxed(
                FlexBoxId::from(Id::ComponentFocusDialog),
                dialog_options,
                editor_options,
                on_dialog_press_handler,
                on_dialog_editor_change_handler,
            )
        };

        ComponentRegistry::put(
            component_registry_map,
            FlexBoxId::from(Id::ComponentFocusDialog),
            boxed_dialog_component,
        );
    }
}

mod stylesheet {
    use super::*;

    pub fn create_stylesheet() -> CommonResult<TuiStylesheet> {
        let colors = &crate::config::get().appearance.colors;
        let sidebar_bg = TuiColor::Rgb(RgbValue::from_hex(&colors.sidebar_bg));
        throws_with_return!({
            tui_stylesheet! {
                new_style!(
                    id: {Id::StyleContainer}
                    padding: {0}
                ),
                new_style!(
                    id: {Id::StyleEditorDefault}
                    padding: {1}
                ),
                new_style!(
                    id: {Id::StyleTreeDefault}
                    padding: {0}
                    color_bg: {sidebar_bg}
                ),
                new_style!(
                    id: {Id::StyleDialogTitle}
                    bold
                    color_fg: {tui_color!(yellow)}
                ),
                new_style!(
                    id: {Id::StyleDialogBorder}
                    dim
                    color_fg: {tui_color!(green)}
                ),
                new_style!(
                    id: {Id::StyleDialogEditor}
                    bold
                    color_fg: {tui_color!(magenta)}
                ),
                new_style!(
                    id: {Id::StyleDialogResultsPanel}
                    color_fg: {tui_color!(blue)}
                ),
                new_style!(
                    id: {Id::StyleFocusDialogTitle}
                    bold
                    color_fg: {tui_color!(yellow)}
                ),
                new_style!(
                    id: {Id::StyleFocusDialogBorder}
                    dim
                    color_fg: {tui_color!(cyan)}
                ),
                new_style!(
                    id: {Id::StyleFocusDialogEditor}
                    bold
                    color_fg: {tui_color!(white)}
                ),
                new_style!(
                    id: {Id::StyleFocusDialogResultsPanel}
                    color_fg: {tui_color!(blue)}
                )
            }
        })
    }
}

mod status_bar {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub fn render_status_bar(
        pipeline: &mut RenderPipeline,
        size: Size,
        state: &State,
    ) {
        let colors = &crate::config::get().appearance.colors;
        let color_bg = tui_color!(hex "#313244");
        let color_fg = TuiColor::Rgb(RgbValue::from_hex(&colors.sidebar_fg));
        let dim_fg = tui_color!(hex "#6c7086");
        let dirty_fg = tui_color!(hex "#f9e2af"); // yellow for dirty indicator

        // Build file label: "~[filename.md]" if dirty, "[filename.md]" if clean
        let file_label = match &state.current_file_name {
            Some(name) if state.is_dirty => format!("~[{}]", name),
            Some(name) => format!("[{}]", name),
            None if state.is_dirty => "~[untitled]".to_string(),
            None => "[untitled]".to_string(),
        };

        let sidebar_hint = if state.sidebar_visible {
            "^T:Tree"
        } else {
            "^T:Tree"
        };

        let focus_hint = if state.sidebar_visible {
            " ^F:Focus Tree ^E:Focus Editor"
        } else {
            ""
        };

        let file_label_style = if state.is_dirty {
            new_style!(bold color_fg: {dirty_fg} color_bg: {color_bg})
        } else {
            new_style!(color_fg: {color_fg} color_bg: {color_bg})
        };

        let styled_texts = tui_styled_texts! {
            tui_styled_text! {
                @style: file_label_style,
                @text: &file_label
            },
            tui_styled_text! {
                @style: new_style!(dim color_fg: {dim_fg} color_bg: {color_bg}),
                @text: " | "
            },
            tui_styled_text! {
                @style: new_style!(color_fg: {color_fg} color_bg: {color_bg}),
                @text: "^S:Save ^Q:Quit"
            },
            tui_styled_text! {
                @style: new_style!(dim color_fg: {dim_fg} color_bg: {color_bg}),
                @text: " | "
            },
            tui_styled_text! {
                @style: new_style!(color_fg: {color_fg} color_bg: {color_bg}),
                @text: sidebar_hint
            },
            tui_styled_text! {
                @style: new_style!(color_fg: {color_fg} color_bg: {color_bg}),
                @text: focus_hint
            },
        };

        let row_bottom = size.row_height.convert_to_index();

        let mut render_ops = RenderOpIRVec::new();
        render_ops += RenderOpCommon::MoveCursorPositionAbs(col(0) + row_bottom);
        render_ops += RenderOpCommon::ResetColor;
        render_ops += RenderOpCommon::SetBgColor(color_bg);
        render_ops += RenderOpIR::PaintTextWithAttributes(
            SPACER_GLYPH.repeat(size.col_width.as_usize()).into(),
            None,
        );
        render_ops += RenderOpCommon::ResetColor;
        render_ops += RenderOpCommon::MoveCursorPositionAbs(col(0) + row_bottom);
        render_tui_styled_texts_into(&styled_texts, &mut render_ops);
        pipeline.push(ZOrder::Normal, render_ops);
    }
}
