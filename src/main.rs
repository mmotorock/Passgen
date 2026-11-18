//! A graphical and command-line password generator written in Rust.

// This line hides the console window on Windows in release builds,
// but allows it to appear for command-line use and debugging.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

use crate::app::cli::run_cli;
use crate::app::state::CliArgs;
use crate::app::ui::run_gui_mode;
use clap::Parser;
use std::process::ExitCode;

/// The main entry point of the application.
fn main() -> ExitCode {
    let args = CliArgs::parse();

    if args.wants_cli() {
        run_cli(&args)
    } else {
        run_gui_mode();
        ExitCode::SUCCESS
    }
}
