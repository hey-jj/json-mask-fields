//! Command line front end for the fields query language.
//!
//! Usage: `json-fieldmask <fields> [input.json]`
//!
//! Reads JSON from the file given as the second argument, or from stdin when no
//! file is given. Masks it with the fields query and prints compact JSON. Any
//! error prints a message to stderr, prints the usage banner to stdout, and
//! exits with code 1.

use std::fs;
use std::io::{self, IsTerminal, Read};
use std::process::ExitCode;

use json_fieldmask::mask;

const MISSING_INPUT: &str =
    "Either pipe input into json-fieldmask or specify a file as second argument";

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let fields = args.next();
    let input_path = args.next();

    match run(fields.as_deref(), input_path.as_deref()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            usage(&message);
            ExitCode::from(1)
        }
    }
}

/// Read input, mask it, and print the result.
///
/// Returns an error message on missing fields, missing input, unreadable files,
/// or invalid JSON.
fn run(fields: Option<&str>, input_path: Option<&str>) -> Result<(), String> {
    let fields = match fields {
        Some(f) if !f.is_empty() => f,
        _ => return Err("Fields argument missing".to_string()),
    };

    let input = read_input(input_path)?;
    let json: serde_json::Value =
        serde_json::from_str(&input).map_err(|e| format!("Invalid JSON: {e}"))?;
    let masked = mask(&json, fields);
    println!(
        "{}",
        serde_json::to_string(&masked).map_err(|e| e.to_string())?
    );
    Ok(())
}

/// Resolve the input text.
///
/// Reads the file when a path is given. Otherwise reads stdin unless stdin is a
/// terminal. Empty input is an error.
fn read_input(input_path: Option<&str>) -> Result<String, String> {
    if let Some(path) = input_path {
        return fs::read_to_string(path).map_err(|e| e.to_string());
    }

    if io::stdin().is_terminal() {
        return Err(MISSING_INPUT.to_string());
    }

    let mut buf = String::new();
    io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| e.to_string())?;
    if buf.is_empty() {
        return Err(MISSING_INPUT.to_string());
    }
    Ok(buf)
}

/// Print an error and the usage banner.
fn usage(message: &str) {
    eprintln!("{message}");
    println!("Usage: json-fieldmask <fields> [input.json]");
    println!("Examples:");
    println!("  json-fieldmask \"url,object(content,attachments/url)\" input.json");
    println!("  cat input.json | json-fieldmask \"url,object(content,attachments/url)\"");
}
