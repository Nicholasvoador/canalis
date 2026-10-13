#![cfg_attr(windows, windows_subsystem = "windows")]

mod cli;
mod device_config;
mod hidden_command;
mod link;
mod media;
mod paths;
mod protocol;
mod sensors;
mod service;
mod service_control;
mod settings;
mod status;
mod tray;
mod ui;
#[cfg(windows)]
mod windows_process;

use clap::Parser;
use cli::{Cli, Command};
use settings::SettingsStore;
use std::process::ExitCode;

fn main() -> ExitCode {
    #[cfg(windows)]
    if std::env::args_os().len() > 1 {
        windows_process::attach_parent_console();
    }
    let store = SettingsStore::new(paths::settings_path());
    match Cli::parse().command.unwrap_or(Command::Ui { page: ui::StartPage::Home }) {
        Command::Ui { page } => run_window(store, page),
        Command::Service { port } => run_service(store, port),
        settings_command => print_outcome(cli::run_settings_command(settings_command, &store)),
    }
}

fn run_window(store: SettingsStore, page: ui::StartPage) -> ExitCode {
    match ui::run(store, paths::status_path(), page) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => print_outcome(Err(format!("cannot open window: {error}"))),
    }
}

fn run_service(store: SettingsStore, port: Option<String>) -> ExitCode {
    #[cfg(windows)]
    if let Err(error) = prepare_windows_service() {
        return print_outcome(Err(error));
    }
    let screen_link = std::thread::spawn(move || {
        let status = status::StatusWriter::new(paths::status_path());
        service::PanoramaService::new(store, status, port).run_forever();
    });
    if let Err(error) = tray::run() {
        eprintln!("panorama: {error}; running without a tray icon");
    }
    let _never_finishes = screen_link.join();
    ExitCode::SUCCESS
}

/// Only one service may talk to the screen; with no console, its messages go to a log file.
#[cfg(windows)]
fn prepare_windows_service() -> Result<(), String> {
    if !windows_process::claim_single_instance(r"Local\PanoramaService") {
        return Err("the service is already running".into());
    }
    let log_file = paths::log_path();
    windows_process::redirect_output_to(&log_file).map_err(|error| format!("cannot open log file {}: {error}", log_file.display()))
}

fn print_outcome(outcome: Result<String, String>) -> ExitCode {
    match outcome {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("panorama: {error}");
            ExitCode::from(2)
        }
    }
}
