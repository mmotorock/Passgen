//! A graphical and command-line password generator written in Rust.

// This line hides the console window on Windows in release builds,
// but allows it to appear for command-line use and debugging.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

use crate::app::cli::{run_char_cli_mode, run_word_cli_mode};
use crate::app::state::CliArgs;
use crate::app::ui::run_gui_mode;
use clap::Parser;

/// The main entry point of the application.
fn main() {
    let args = CliArgs::parse();

    if let Some(length) = args.n {
        run_char_cli_mode(length);
    } else if let Some(count) = args.w {
        run_word_cli_mode(count);
    } else {
        run_gui_mode();
    }
}
