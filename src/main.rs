//! `compass` entry point.
//!
//! Thin by design: parsing, dispatch and rendering live in the library so the
//! same surface is reachable from tests and from any future embedding.

use compass::cli::{self, EXIT_USAGE};
use compass::cmd;
use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let inv = match cli::parse(&args) {
        Ok(i) => i,
        Err(e) => return fail(&e, EXIT_USAGE),
    };

    match cmd::execute(&inv) {
        Ok(out) => {
            let text = if inv.json {
                out.json.render()
            } else {
                out.text
            };
            // A closed pipe (`compass status | head`) is not an error.
            let _ = std::io::stdout().write_all(text.as_bytes());
            let _ = std::io::stdout().flush();
            ExitCode::from(out.code as u8)
        }
        Err(e) => fail(&e, compass::cli::EXIT_FAILURE),
    }
}

/// Report a failure on stderr so it never contaminates `--json` on stdout.
fn fail(message: &str, code: i32) -> ExitCode {
    let _ = writeln!(std::io::stderr(), "error: {message}");
    ExitCode::from(code as u8)
}
