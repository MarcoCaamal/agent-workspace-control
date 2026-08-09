//! CLI contract tests: exits 0/1/2/3, JSON envelope (schemaVersion + ok,
//! exactly `data` or `error`), one newline-terminated JSON document, stderr
//! for human errors, no mutation on workspace-not-found.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("awctl-cli-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn run_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_awctl"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("run awctl")
}

/// stdout must be exactly one newline-terminated JSON document.
fn json_stdout(out: &Output) -> Value {
    let s = String::from_utf8(out.stdout.clone()).expect("utf8 stdout");
    assert_eq!(s.matches('\n').count(), 1, "exactly one trailing newline");
    serde_json::from_str(s.trim_end()).expect("stdout is one JSON document")
}

/// Asserts schemaVersion, ok, exactly data xor error.
fn envelope(ok: bool, out: &Output) -> Value {
    let doc = json_stdout(out);
    assert_eq!(doc["schemaVersion"], 1);
    assert_eq!(doc["ok"], ok);
    assert_eq!(doc.get("data").is_some(), ok, "data present iff ok");
    assert_eq!(doc.get("error").is_some(), !ok, "error present iff not ok");
    doc
}

#[test]
fn init_json_exits_0_with_data_and_silent_stderr() {
    let dir = temp_dir("init-json");
    let out = run_in(&dir, &["init", "--json"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stderr.is_empty(), "no stderr on JSON success");
    let data = envelope(true, &out)["data"].clone();
    assert_eq!(data["databaseOk"], true);
    assert_eq!(data["schemaOk"], true);
    assert!(data["root"].as_str().unwrap().ends_with("init-json"));
    assert!(dir.join(".awc/config.toml").exists());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn human_init_then_status_and_doctor_json_from_nested_dir() {
    let dir = temp_dir("nested");
    let out = run_in(&dir, &["init"]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.starts_with("AWC workspace at"));

    let nested = dir.join("a").join("b");
    fs::create_dir_all(&nested).unwrap();
    let doc = envelope(true, &run_in(&nested, &["status", "--json"]));
    let data = doc["data"].clone();
    assert_eq!(data["databaseOk"], true);
    assert_eq!(data["schemaOk"], true);

    let doc = envelope(true, &run_in(&nested, &["doctor", "--quick", "--json"]));
    let checks = doc["data"]["checks"].as_array().unwrap();
    let names: Vec<&str> = checks.iter().map(|c| c["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["path", "config", "database", "schema"]);
    assert!(checks.iter().all(|c| c["ok"] == true));
    assert!(
        checks.iter().all(|c| c.get("message").is_none()),
        "ok checks omit message"
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn workspace_not_found_exits_3_with_json_error_and_no_mutation() {
    let dir = temp_dir("nows");
    let sub = dir.join("sub");
    fs::create_dir_all(&sub).unwrap();
    let out = run_in(&sub, &["status", "--json"]);
    assert_eq!(out.status.code(), Some(3));
    let doc = envelope(false, &out);
    assert_eq!(doc["error"]["code"], "workspace_not_found");
    assert!(!doc["error"]["message"].as_str().unwrap().is_empty());
    assert!(
        !dir.join(".awc").exists(),
        "failed read-only command must not create .awc"
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn usage_errors_exit_2_with_stderr_only() {
    let dir = temp_dir("usage");
    for args in [&["bogus"][..], &["doctor"][..], &["status", "--nope"][..]] {
        let out = run_in(&dir, args);
        assert_eq!(out.status.code(), Some(2), "usage exit 2 for {args:?}");
        assert!(!out.stderr.is_empty(), "usage error on stderr");
        assert!(out.stdout.is_empty(), "no stdout for usage errors");
    }
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn human_errors_go_to_stderr_not_stdout() {
    let dir = temp_dir("human-err");
    let out = run_in(&dir, &["status"]);
    assert_eq!(out.status.code(), Some(3));
    assert!(out.stdout.is_empty(), "human error never writes stdout");
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains("no AWC workspace")
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn operational_failure_exits_1_with_json_error() {
    let dir = temp_dir("opfail");
    let state = dir.join(".awc");
    fs::create_dir_all(&state).unwrap();
    fs::write(state.join("config.toml"), b"schema_version = [bad").unwrap();

    for args in [&["init", "--json"][..], &["status", "--json"][..]] {
        let out = run_in(&dir, args);
        assert_eq!(
            out.status.code(),
            Some(1),
            "operational exit 1 for {args:?}"
        );
        let doc = envelope(false, &out);
        assert_eq!(doc["error"]["code"], "invalid_config");
        assert!(!doc["error"]["message"].as_str().unwrap().is_empty());
    }
    fs::remove_dir_all(&dir).ok();
}
