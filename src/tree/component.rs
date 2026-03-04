use crate::app::AppSignal;
use crate::state::State;
use r3bl_tui::{
    BoxedSafeComponent, CommonResult, Component, EventPropagation, FlexBox, FlexBoxId,
    GlobalData, HasFocus, InputEvent, Key, KeyPress,
    RenderOpCommon, RenderOpIR, RenderOpIRVec, RenderPipeline, SpecialKey, SurfaceBounds,
    TerminalWindowMainThreadSignal, ZOrder, col, new_style, render_pipeline, row,
    send_signal, throws_with_return, tui_color,
};

#[derive(Debug, Clone, Default)]
pub struct TreeComponent {
    pub id: FlexBoxId,
}

impl TreeComponent {
    pub fn new_boxed(id: FlexBoxId) -> BoxedSafeComponent<State, AppSignal> {
        Box::new(Self { id })
    }
}

impl Component<State, AppSignal> for TreeComponent {
    fn reset(&mut self) {}

    fn get_id(&self) -> FlexBoxId {
        self.id
    }

    fn handle_event(
        &mut self,
        global_data: &mut GlobalData<State, AppSignal>,
        input_event: InputEvent,
        _has_focus: &mut HasFocus,
    ) -> CommonResult<EventPropagation> {
        throws_with_return!({
            let mut event_consumed = false;

            if let InputEvent::Keyboard(keypress) = input_event {
                match keypress {
                    // Arrow keys for navigation
                    KeyPress::Plain {
                        key: Key::SpecialKey(special_key),
                    } => {
                        match special_key {
                            SpecialKey::Up => {
                                event_consumed = true;
                                send_signal!(
                                    global_data.main_thread_channel_sender,
                                    TerminalWindowMainThreadSignal::ApplyAppSignal(
                                        AppSignal::TreeMoveUp
                                    )
                                );
                            }
                            SpecialKey::Down => {
                                event_consumed = true;
                                send_signal!(
                                    global_data.main_thread_channel_sender,
                                    TerminalWindowMainThreadSignal::ApplyAppSignal(
                                        AppSignal::TreeMoveDown
                                    )
                                );
                            }
                            SpecialKey::Right => {
                                event_consumed = true;
                                send_signal!(
                                    global_data.main_thread_channel_sender,
                                    TerminalWindowMainThreadSignal::ApplyAppSignal(
                                        AppSignal::TreeExpand
                                    )
                                );
                            }
                            SpecialKey::Left => {
                                event_consumed = true;
                                send_signal!(
                                    global_data.main_thread_channel_sender,
                                    TerminalWindowMainThreadSignal::ApplyAppSignal(
                                        AppSignal::TreeCollapse
                                    )
                                );
                            }
                            SpecialKey::Enter => {
                                event_consumed = true;
                                send_signal!(
                                    global_data.main_thread_channel_sender,
                                    TerminalWindowMainThreadSignal::ApplyAppSignal(
                                        AppSignal::TreeEnter
                                    )
                                );
                            }
                            SpecialKey::Backspace => {
                                event_consumed = true;
                                send_signal!(
                                    global_data.main_thread_channel_sender,
                                    TerminalWindowMainThreadSignal::ApplyAppSignal(
                                        AppSignal::TreeGoToParent
                                    )
                                );
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }

            if event_consumed {
                EventPropagation::Consumed
            } else {
                EventPropagation::Propagate
            }
        });
    }

    fn render(
        &mut self,
        global_data: &mut GlobalData<State, AppSignal>,
        current_box: FlexBox,
        _surface_bounds: SurfaceBounds,
        has_focus: &mut HasFocus,
    ) -> CommonResult<RenderPipeline> {
        throws_with_return!({
            let mut render_ops = RenderOpIRVec::new();
            let box_origin_pos = current_box.style_adjusted_origin_pos;
            let box_bounds_size = current_box.style_adjusted_bounds_size;
            let max_rows = box_bounds_size.row_height.as_usize();
            let max_cols = box_bounds_size.col_width.as_usize();

            let is_focused = has_focus.does_current_box_have_focus(current_box);
            let colors = &crate::config::get().appearance.colors;

            let content_rows = max_rows.saturating_sub(1);
            let lines_len = global_data
                .state
                .tree_render_lines
                .as_ref()
                .map_or(0, Vec::len);
            global_data.state.tree_viewport_rows = content_rows;
            crate::app::AppMain::sync_tree_scroll_offset(&mut global_data.state);
            let state = &global_data.state;

            let bg_style = new_style!(
                color_fg: {tui_color!(hex &colors.sidebar_fg)}
                color_bg: {tui_color!(hex &colors.sidebar_bg)}
            );
            let selected_style = new_style!(
                bold
                color_fg: {tui_color!(hex &colors.sidebar_fg)}
                color_bg: {tui_color!(hex &colors.highlight)}
            );
            let title_style = if is_focused {
                new_style!(
                    bold
                    color_fg: {tui_color!(hex &colors.sidebar_fg)}
                    color_bg: {tui_color!(hex &colors.highlight)}
                )
            } else {
                new_style!(
                    dim
                    color_fg: {tui_color!(hex &colors.sidebar_fg)}
                    color_bg: {tui_color!(hex &colors.sidebar_bg)}
                )
            };
            let scrollbar_track_style = new_style!(
                color_fg: {tui_color!(hex &colors.sidebar_fg)}
                color_bg: {tui_color!(hex &colors.sidebar_bg)}
            );
            let scrollbar_thumb_style = new_style!(
                bold
                color_fg: {tui_color!(hex &colors.sidebar_fg)}
                color_bg: {tui_color!(hex &colors.highlight)}
            );

            // Row 0: title
            let title = format!(" {}", state.tree_root_display);
            let title_truncated = truncate_to_width(&title, max_cols);
            render_ops += RenderOpCommon::MoveCursorPositionRelTo(
                box_origin_pos,
                col(0) + row(0),
            );
            render_ops += RenderOpIR::PaintTextWithAttributes(
                title_truncated.into(),
                Some(title_style),
            );

            // Rows 1..max_rows: tree lines or blank fill
            let lines = state.tree_render_lines.as_deref().unwrap_or(&[]);
            let scroll_offset = state.tree_scroll_offset;
            let needs_scrollbar = content_rows > 0 && lines_len > content_rows && max_cols > 1;
            let content_cols = if needs_scrollbar {
                max_cols.saturating_sub(1)
            } else {
                max_cols
            };
            let (thumb_start, thumb_end) =
                scrollbar_thumb_bounds(lines_len, content_rows, scroll_offset);

            for row_idx in 0..content_rows {
                render_ops += RenderOpCommon::MoveCursorPositionRelTo(
                    box_origin_pos,
                    col(0) + row((row_idx + 1) as u16),
                );

                if let Some(line) = lines.get(scroll_offset + row_idx) {
                    let style = if line.starts_with('>') {
                        selected_style
                    } else {
                        bg_style
                    };

                    // Truncate to max_cols display width, then pad with spaces
                    let truncated = truncate_to_width(line, content_cols);
                    render_ops += RenderOpIR::PaintTextWithAttributes(
                        truncated.into(),
                        Some(style),
                    );
                } else {
                    // Empty row — fill with background
                    render_ops += RenderOpIR::PaintTextWithAttributes(
                        truncate_to_width("", content_cols).into(),
                        Some(bg_style),
                    );
                }

                if needs_scrollbar {
                    render_ops += RenderOpCommon::MoveCursorPositionRelTo(
                        box_origin_pos,
                        col(content_cols as u16) + row((row_idx + 1) as u16),
                    );
                    let thumb_style = if row_idx >= thumb_start && row_idx < thumb_end {
                        scrollbar_thumb_style
                    } else {
                        scrollbar_track_style
                    };
                    render_ops += RenderOpIR::PaintTextWithAttributes(
                        " ".into(),
                        Some(thumb_style),
                    );
                }
            }

            let mut pipeline = render_pipeline!();
            pipeline.push(ZOrder::Normal, render_ops);
            pipeline
        });
    }
}

fn scrollbar_thumb_bounds(
    total_lines: usize,
    viewport_rows: usize,
    scroll_offset: usize,
) -> (usize, usize) {
    if viewport_rows == 0 || total_lines <= viewport_rows {
        return (0, viewport_rows);
    }

    let thumb_size = ((viewport_rows * viewport_rows) / total_lines)
        .max(1)
        .min(viewport_rows);
    let max_start = viewport_rows.saturating_sub(thumb_size);
    let scrollable = total_lines.saturating_sub(viewport_rows).max(1);
    let start = (scroll_offset.min(scrollable) * max_start) / scrollable;
    (start.min(max_start), start.min(max_start) + thumb_size)
}

/// Truncate a string to exactly `max_width` display columns.
/// Pads with spaces if shorter, truncates with ellipsis if longer.
/// Uses unicode_width for correct multi-byte/wide char handling.
fn truncate_to_width(s: &str, max_width: usize) -> String {
    let mut result = String::with_capacity(max_width);
    let mut width = 0;

    for ch in s.chars() {
        // Most chars are width 1, CJK are width 2, some control chars are 0.
        // For Nerd Font private use area chars, assume width 1.
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
        if width + ch_width > max_width {
            break;
        }
        result.push(ch);
        width += ch_width;
    }

    // Pad remainder with spaces
    while width < max_width {
        result.push(' ');
        width += 1;
    }

    result
}
