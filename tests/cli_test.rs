//! Tests for the command line binary.
//!
//! Each case runs the compiled binary with arguments or piped stdin and checks
//! the exit code, the parsed JSON on stdout, and the error text on stderr.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_json::{json, Value};

fn bin() -> PathBuf {
    // Cargo points this at the binary built for the integration test.
    PathBuf::from(env!("CARGO_BIN_EXE_json-fieldmask"))
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

struct Output {
    code: i32,
    stdout: String,
    stderr: String,
}

fn run(args: &[&str], stdin: Option<&str>) -> Output {
    let mut cmd = Command::new(bin());
    cmd.args(args);
    cmd.stdin(if stdin.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    });
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().expect("spawn binary");
    if let Some(data) = stdin {
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(data.as_bytes())
            .expect("write stdin");
    }
    let out = child.wait_with_output().expect("wait");
    Output {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
    }
}

fn parse(stdout: &str) -> Value {
    serde_json::from_str(stdout.trim()).expect("stdout is JSON")
}

#[test]
fn missing_fields_argument() {
    let out = run(&[], None);
    assert_eq!(out.code, 1);
    assert_eq!(out.stderr.trim(), "Fields argument missing");
    assert!(out.stdout.to_lowercase().contains("usage:"));
}

#[test]
fn missing_input_with_no_file_and_no_pipe() {
    let out = run(&["mask"], None);
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr.trim(),
        "Either pipe input into json-fieldmask or specify a file as second argument"
    );
    assert!(out.stdout.to_lowercase().contains("usage:"));
}

#[test]
fn reads_a_file_given_as_second_argument() {
    let path = fixture("activities.json");
    let out = run(&["kind", path.to_str().unwrap()], None);
    assert_eq!(out.code, 0);
    assert_eq!(parse(&out.stdout), json!({"kind": "plus#activity"}));
}

#[test]
fn invalid_json_file_input() {
    let path = fixture("invalid.json");
    let out = run(&["object", path.to_str().unwrap()], None);
    assert_eq!(out.code, 1);
    // The parse error text differs from other runtimes, so check for any error
    // on stderr and the usage banner on stdout instead of an exact message.
    assert!(!out.stderr.trim().is_empty());
    assert!(out.stdout.to_lowercase().contains("usage:"));
}

#[test]
fn invalid_json_from_stdin() {
    let out = run(&["s"], Some("\n"));
    assert_eq!(out.code, 1);
    assert!(!out.stderr.trim().is_empty());
    assert!(out.stdout.to_lowercase().contains("usage:"));
}

#[test]
fn masks_piped_json() {
    let out = run(&["s"], Some("{\"s\":\"foo\",\"n\":666}"));
    assert_eq!(out.code, 0);
    assert_eq!(parse(&out.stdout), json!({"s": "foo"}));
}

#[test]
fn scalar_input_prints_null() {
    // A truthy scalar drops, which the top level coerces to null. The binary
    // prints the literal null and exits 0.
    let out = run(&["a"], Some("5"));
    assert_eq!(out.code, 0);
    assert_eq!(out.stdout.trim(), "null");
}

#[test]
fn null_input_prints_null() {
    let out = run(&["a"], Some("null"));
    assert_eq!(out.code, 0);
    assert_eq!(out.stdout.trim(), "null");
}

#[test]
fn missing_key_prints_empty_object() {
    // A missing key leaves an empty object, not null.
    let out = run(&["a"], Some("{\"b\":1}"));
    assert_eq!(out.code, 0);
    assert_eq!(out.stdout.trim(), "{}");
}

#[test]
fn masks_piped_fixture_kind() {
    let raw = std::fs::read_to_string(fixture("activities.json")).unwrap();
    let out = run(&["kind"], Some(&raw));
    assert_eq!(out.code, 0);
    assert_eq!(parse(&out.stdout), json!({"kind": "plus#activity"}));
}

#[test]
fn masks_piped_fixture_object_sub_selection() {
    let raw = std::fs::read_to_string(fixture("activities.json")).unwrap();
    let out = run(&["object(objectType)"], Some(&raw));
    assert_eq!(out.code, 0);
    assert_eq!(
        parse(&out.stdout),
        json!({"object": {"objectType": "note"}})
    );
}

#[test]
fn masks_piped_fixture_nested_path() {
    let raw = std::fs::read_to_string(fixture("activities.json")).unwrap();
    let out = run(&["url,object(content,attachments/url)"], Some(&raw));
    assert_eq!(out.code, 0);
    assert_eq!(
        parse(&out.stdout),
        json!({
            "url": "https://plus.google.com/102817283354809142195/posts/F97fqZwJESL",
            "object": {
                "content": "Congratulations! You have successfully fetched an explicit public activity. The attached video is your reward. :)",
                "attachments": [{"url": "http://www.youtube.com/watch?v=dQw4w9WgXcQ"}]
            }
        })
    );
}
