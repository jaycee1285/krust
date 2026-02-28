mod app;
mod config;
mod state;
mod tree;

use app::AppMain;
use clap::Parser;
use r3bl_tui::{
    GlobalData, InputDevice, InputEvent, OutputDevice, TerminalWindow, key_press,
    ModifierKeysMask, ok,
};

#[derive(Parser, Debug)]
#[command(name = "krust", about = "TUI Markdown editor with file tree")]
struct Cli {
    /// File to open
    file: Option<String>,
}

#[tokio::main]
async fn main() -> r3bl_tui::CommonResult<()> {
    let cli = Cli::parse();

    // Load config and set syntax theme before r3bl_tui initializes its LazyLock.
    let cfg = config::init();
    // Safety: single-threaded at this point, before tokio spawns any tasks.
    unsafe { std::env::set_var("R3BL_TUI_THEME", &cfg.appearance.syntax_theme); }

    let state = state::constructor::new(cli.file.as_deref());
    let app = AppMain::new_boxed();

    let exit_keys = &[InputEvent::Keyboard(
        key_press! { @char ModifierKeysMask::new().with_ctrl(), 'q' },
    )];

    let _unused: (GlobalData<_, _>, InputDevice, OutputDevice) =
        TerminalWindow::main_event_loop(app, exit_keys, state)?.await?;

    ok!()
}
