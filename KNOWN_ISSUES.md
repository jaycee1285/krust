# Krust — Known Issues

## Layout toggle leaves stale cells (r3bl_tui framework limitation)

**Symptom**: When toggling between single-column (editor only) and two-column (tree + editor) layouts via Ctrl-T, the previous layout's content remains visible as ghost text until a character is typed into the editor component.

**Root cause**: r3bl_tui uses a diff-based rendering pipeline. Each frame, the app's render pipeline is composed into an offscreen buffer, diffed against the previous frame's buffer, and only changed cells are repainted. When the layout changes, cells that no component writes to (e.g. where the tree was) remain stale in the offscreen buffer, and the diff sees them as unchanged.

**What we tried**:

1. **Offscreen buffer invalidation** — Setting `global_data.maybe_saved_ofs_buf = None` in the ToggleSidebar signal handler forces a full repaint (no diff). Didn't fix it — the full paint still only writes cells that the render pipeline covers, and the EditorComponent doesn't paint every cell in its box.

2. **ClearScreen render op** — Injecting `RenderOpCommon::ClearScreen` at `ZOrder::Glass` before the layout renders. Clears the terminal but the new frame doesn't repaint correctly after.

3. **Two separate layout states** — Instead of conditional branching inside one layout, created two independent `SurfaceRender` implementations (`SingleColLayout` and `TwoColLayout`), each building its own complete `Surface`. Combined with offscreen buffer invalidation. Same result — the framework's diff/paint cycle still doesn't repaint cells the new layout doesn't explicitly write to.

4. **Forced Render signals** — Sending `TerminalWindowMainThreadSignal::Render(None)` (one and two) after the toggle to trigger additional render+paint cycles. The signals are processed but don't produce a visible repaint.

5. **Focus mode dialog** — Showing an r3bl `DialogComponent` on Ctrl-T that requires y/n to confirm. The keystroke dismissing the dialog should trigger a render cycle. Still needed a second character after dismissal.

6. **Eat next input** — Consuming the next input event after toggle with `ConsumedRender`. The framework doesn't fully repaint on `ConsumedRender` alone — it needs a component to actually process input and mutate its buffer.

7. **Tree on right, editor on left** — Tested whether the EditorComponent being in the leftmost (first-rendered) box fixes the issue, since the editor starts at col 0 in both single and two-col layouts. Same behavior.

8. **Tree fullscreen as single-col** — Swapped single-col to render the tree component at 100% instead of the editor. Toggle to/from tree fullscreen worked perfectly — no stale cells. Confirms the issue is specific to the EditorComponent's rendering, which doesn't repaint every cell in its box on layout change.

**Conclusion**: The EditorComponent (r3bl_tui built-in) only paints cells that contain content. It doesn't fill its entire box with background on each frame. Combined with the diff-based renderer, this means layout changes that resize the editor's box leave unpainted regions. The tree component (custom) doesn't have this problem because it explicitly paints every row including blank fill.

**Current workaround**: The two-column layout is the default and works correctly. The single-column toggle is functional but requires one character typed into the editor to trigger a clean repaint after switching. This character goes into the editor buffer and can be undone.

**Possible future fixes**:
- Patch EditorComponent to fill its entire box with background on each render
- Use a custom editor component that wraps EditorComponent and fills remaining cells
- File an issue upstream with r3bl_tui about layout-change repaint behavior

---

## Save doesn't visually confirm

**Symptom**: Ctrl-S saves the file but there's no visual feedback (flash, message, etc).

**Status**: The dirty indicator in the status bar (`~[filename.md]`) clears on save, which is the current feedback mechanism. A transient "Saved!" message in the status bar would be better UX.

---

## Arrow key left from col 0 moves to previous line in editor

**Symptom**: In the editor component, pressing left arrow at column 0 moves the cursor to the end of the previous line. This is standard editor behavior but can feel unexpected in a TUI context.

**Status**: This is EditorComponent's built-in behavior, not a krust bug.
