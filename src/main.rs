//! Shape CLI - Local-first task management for software teams

use std::process::ExitCode;

fn main() -> ExitCode {
    if let Err(e) = shape_cli::cli::run() {
        eprintln!("Error: {:#}", e);
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
