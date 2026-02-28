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
            let state = &global_data.state;

            let bg_style = new_style!(color_fg: {tui_color!(hex "#cdd6f4")} color_bg: {tui_color!(hex "#1e1e2e")});
            let selected_style = new_style!(bold color_fg: {tui_color!(hex "#cdd6f4")} color_bg: {tui_color!(hex "#45475a")});
            let title_style = if is_focused {
                new_style!(bold color_fg: {tui_color!(hex "#cdd6f4")} color_bg: {tui_color!(hex "#45475a")})
            } else {
                new_style!(dim color_fg: {tui_color!(hex "#a6adc8")} color_bg: {tui_color!(hex "#313244")})
            };

            // Paint every row — title on row 0, tree content on 1+, blank fill for the rest.
            let blank = " ".repeat(max_cols);

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
            let content_rows = max_rows.saturating_sub(1);
            let lines = state.tree_render_lines.as_deref().unwrap_or(&[]);
            let scroll_offset = state.tree_scroll_offset;

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
                    let truncated = truncate_to_width(line, max_cols);
                    render_ops += RenderOpIR::PaintTextWithAttributes(
                        truncated.into(),
                        Some(style),
                    );
                } else {
                    // Empty row — fill with background
                    render_ops += RenderOpIR::PaintTextWithAttributes(
                        blank.clone().into(),
                        Some(bg_style),
                    );
                }
            }

            let mut pipeline = render_pipeline!();
            pipeline.push(ZOrder::Normal, render_ops);
            pipeline
        });
    }
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
