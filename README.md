# krust

A TUI Markdown editor with a file tree, built in Rust.

## Usage

```
krust [file]
```

Exit with `Ctrl+Q`.

## Configuration

Copy `config.example.toml` to `~/.config/krust/config.toml` and edit as needed.

Key options:

```toml
[appearance]
syntax_theme = "ayu-mirage"  # options: "r3bl", "ayu-light", "ayu-mirage"

[appearance.colors]
sidebar_bg = "#1e1e2e"
sidebar_fg = "#cdd6f4"
editor_bg  = "#1e1e2e"
highlight  = "#45475a"

[behavior]
show_hidden    = false
follow_symlinks = false
sidebar_width  = 25  # percent
```

## Third-party attribution

This project includes a modified version of **r3bl_tui**, originally licensed
under the **Apache License 2.0**.

> Copyright © 2022–2025 R3BL LLC.
> Original project: <https://github.com/r3bl-org/r3bl-open-core>
> Changes were made for this project. See `NOTICE` for details.

The modified fork is at <https://github.com/jaycee1285/r3bl-open-core>.
