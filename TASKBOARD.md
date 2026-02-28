# Krust QoL Taskboard

Effort tags: `S` (small), `M` (medium), `L` (large)

## Now

- [ ] (`S`) Upstream PR: editor render cache should key on viewport size (not terminal window size)
  - Repo: `repos/r3bl-open-core`
  - File: `tui/src/tui/editor/editor_buffer/render_cache.rs`

- [ ] (`M`) Upstream PR: optional flat Markdown renderer path (`syntect`) vs fancy `r3bl` path
  - Repo: `repos/r3bl-open-core`
  - File: `tui/src/tui/editor/editor_engine/engine_public_api.rs`

- [ ] (`M`) Upstream PR / patch cleanup: embedded Ayu themes + `R3BL_TUI_THEME`
  - Repo: `repos/r3bl-open-core`
  - Files:
    - `tui/src/tui/syntax_highlighting/r3bl_syntect_theme.rs`
    - `tui/src/tui/syntax_highlighting/global_syntax_resources.rs`
    - `tui/src/tui/syntax_highlighting/assets/ayu-light.tmTheme`
    - `tui/src/tui/syntax_highlighting/assets/ayu-mirage.tmTheme`

- [ ] (`S`) Print tarball hash after `release.sh` build (`sha256sum` or `nix hash file`)
  - Makes `tauri.nix` updates faster
  - File: `release.sh`

- [ ] (`S`) Document local dev setup for patched `r3bl_tui`
  - Why `[patch.crates-io]` exists and when to remove it
  - File: `StackBuild.md` or `README.md` (if added)

## Next

- [ ] (`M`) Wire `Config::load()` into app startup and store config in app state
  - Files: `src/main.rs`, `src/state.rs`, `src/app.rs`

- [ ] (`S`) Add `appearance.syntax_highlight = true/false`
  - Disable editor syntax highlighting without code changes
  - Files: `src/config.rs`, `config.example.toml`, `src/app.rs`

- [ ] (`M`) Add `appearance.syntax_theme = "ayu-mirage" | "ayu-light" | "r3bl"`
  - Prefer config over env var, keep env var as override
  - Files: `src/config.rs`, `config.example.toml`, `src/main.rs`, local `r3bl-open-core`

- [ ] (`M`) Add `appearance.markdown_renderer = "syntect" | "r3bl"`
  - Switch flat vs fancy Markdown render path from config
  - Files: `src/config.rs`, `config.example.toml`, local `r3bl-open-core` integration point

- [ ] (`M`) Make all hardcoded UI colors configurable
  - Tree pane colors (`src/tree/component.rs`)
  - Status bar colors (`src/app.rs`)
  - Layout/editor pane background styles (`src/app.rs`)

- [ ] (`S`) Sidebar width from config (`behavior.sidebar_width`) should actually drive layout width
  - It exists in config schema but is not applied
  - Files: `src/config.rs`, `src/app.rs`

- [ ] (`S`) Clean warnings in local `r3bl-open-core` patch branch
  - Unused `window_size` in render cache after viewport-key fix
  - Dead code warnings from forcing syntect path (gate with config instead of hard switch)

## Later

- [ ] (`S`) Save feedback message in status bar (`Saved`, `Save failed`)
  - Current save only clears dirty indicator
  - Files: `src/state.rs`, `src/app.rs`

- [ ] (`M`) Add keybinding help panel / modal (`?` or `Ctrl-/`)
  - Quick discoverability for tree/editor shortcuts

- [ ] (`S`) Respect focus visually more clearly in editor/tree panes
  - Border/title/background differences

- [ ] (`S`) Add `release.sh` flag for theme default / env var notes in archive docs
  - Include a short `README-release.txt` in tarball with runtime tips
  - File: `release.sh`

- [ ] (`S`) Add `--no-upload` / `--upload` CLI flags (instead of env vars only)
  - File: `release.sh`

- [ ] (`M`) Add regression test for viewport-size cache invalidation
  - Reproduce layout resize / viewport change and assert cache miss
  - Repo: `repos/r3bl-open-core`
  - File: `tui/src/tui/editor/editor_buffer/render_cache.rs`

- [ ] (`S`) Remove `krust` local `[patch.crates-io]` override after upstream merge/release
  - File: `Cargo.toml`
