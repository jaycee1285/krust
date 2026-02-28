# StackBuild - Crustdown

## Dev Stack
- **Language:** Go 1.25.5
- **TUI Framework:** Bubble Tea 1.3.10
- **Components:** Bubbles 0.21.1 (viewport, textarea, textinput)
- **Styling:** Lipgloss 1.1.1
- **Markdown Rendering:** Glamour 0.10.0
- **Markdown Parsing:** Goldmark 1.7.16 (with frontmatter xtension)

## Target
- **Desktop** (Linux/NixOS terminal)

## Additional Key Libraries (UI)
- Bubble Tea (TUI framework — alt screen mode, mouse support, tea.Model pattern)
- Bubbles (viewport for scrolling viewer, textarea for editor, textinput for omnibox)
- Lipgloss (terminal styling, layout, horizontal joins)
- Glamour (markdown → styled ANSI terminal output, dark/light themes)
- Goldmark (markdown AST parsing, heading/link extraction)
- goldmark-frontmatter (YAML front matter stripping)ss
- atotto/clipboard (system clipboard for editor cut/copy/paste)
- adrg/xdg (XDG Base Directory paths for config/data)

## Key Features
Terminal markdown browser and editor for Linux/NixOS. View local files or remote URLs with styled rendering, navigate via sidebar, bookmarks, history, and link hints.

- Unlike web-based markdown viewers — terminal-native, keyboard-driven, no browser needed
- Unlike `glow` — full browser with navigation history, bookmarks, sidebar, link following
- Inspired by Frogmouth (Textualize)
- Glamour-rendered markdown with dark/light theme toggle (F10)
- Link hint mode (Alt+L) — two-character labels for keyboard link navigation
- Omnibox command bar (/, :) — paths, URLs, commands, git forge shortcuts
- 4-tab sidebar: table of contents, local file browser, bookmarks, history
- Git forge integration: GitHub, GitLab, Bitbucket, Codeberg README fetching
- Edit mode (Ctrl+E) — raw markdown editing with textarea, local files only
- Line-level clipboard: Ctrl+C copy line, Ctrl+X cut line, Ctrl+V paste
- Save with Ctrl+S, viewer auto-updates on save or exit

---

## Building Instructions

### Nix develop?
Yes — `flake.nix` with Go + gopls + gotools + go-tools.
```bash
nix develop                    # Enter dev environment
```
### Dev server?
N/A (TUI app, not web-based)

### Tauri dev server?
N/A (pure Go, no Tauri)

### Commands to run
```bash
go mod tidy                    # Ensure dependencies
go run .                       # Run directly
go run . README.md             # Open a specific file
go run . https://example.com/file.md  # Open a URL
go build -o crustdown .        # Build binary
go vet ./...                   # Static analysis
```

---

## Android Build
N/A — terminal application only.

## Desktop Build
```bash
go build -o crustdown .        # Build binary
```

- **Release script?** No
- **Nix package:** `nix build` (flake packages.default, vendorHash needs updating)

## Web Build
N/A — terminal application only.