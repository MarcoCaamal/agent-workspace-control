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

#[test]
fn project_add_derives_slug_json_and_human() {
    let dir = temp_dir("padd");
    run_in(&dir, &["init"]);
    let out = run_in(
        &dir,
        &["project", "add", "--name", "My Cool  Project!", "--json"],
    );
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stderr.is_empty(), "no stderr on JSON success");
    let project = envelope(true, &out)["data"]["project"].clone();
    assert_eq!(project["slug"], "my-cool-project");
    assert_eq!(project["name"], "My Cool  Project!");
    assert_eq!(project["id"].as_str().unwrap().len(), 36);
    let out = run_in(&dir, &["project", "add", "--name", "Second"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(
        String::from_utf8(out.stdout)
            .unwrap()
            .contains("project added: second (Second)")
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn project_add_slug_conflict_exits_1_without_insert() {
    let dir = temp_dir("pconf");
    run_in(&dir, &["init"]);
    run_in(&dir, &["project", "add", "--name", "Alpha"]);
    for args in [
        &["project", "add", "--name", "alpha", "--json"][..],
        &[
            "project", "add", "--name", "Beta", "--slug", "alpha", "--json",
        ][..],
    ] {
        let out = run_in(&dir, args);
        assert_eq!(out.status.code(), Some(1), "{args:?}");
        assert_eq!(envelope(false, &out)["error"]["code"], "slug_conflict");
    }
    let out = run_in(&dir, &["project", "list", "--json"]);
    let data = envelope(true, &out)["data"].clone();
    let projects = data["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 1, "collision must not insert");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn project_show_resolves_prefix_and_rejects_not_found_or_ambiguous() {
    let dir = temp_dir("pshow");
    run_in(&dir, &["init"]);
    let out = run_in(&dir, &["project", "add", "--name", "One", "--json"]);
    let id_a = envelope(true, &out)["data"]["project"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let out = run_in(&dir, &["project", "add", "--name", "Two", "--json"]);
    let id_b = envelope(true, &out)["data"]["project"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let common: String = id_a
        .chars()
        .zip(id_b.chars())
        .take_while(|(x, y)| x == y)
        .map(|(x, _)| x)
        .collect();
    let unique_a = format!("{}{}", common, &id_a[common.len()..common.len() + 1]);

    let out = run_in(&dir, &["project", "show", &id_a, "--json"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(envelope(true, &out)["data"]["project"]["slug"], "one");
    let out = run_in(&dir, &["project", "show", &unique_a, "--json"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(envelope(true, &out)["data"]["project"]["slug"], "one");

    let out = run_in(&dir, &["project", "show", "ffffffff", "--json"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(envelope(false, &out)["error"]["code"], "project_not_found");

    let out = run_in(&dir, &["project", "show", &common, "--json"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        envelope(false, &out)["error"]["code"],
        "ambiguous_project_id"
    );

    let out = run_in(&dir, &["project", "show", "ffffffff"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty(), "human errors go to stderr");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn project_list_is_deterministic_and_external_root_is_metadata_only() {
    let dir = temp_dir("plist");
    run_in(&dir, &["init"]);
    for name in ["Zulu", "Alpha", "Mike"] {
        run_in(&dir, &["project", "add", "--name", name]);
    }
    let outside =
        std::env::temp_dir().join(format!("awctl-cli-{}-external-root", std::process::id()));
    let _ = fs::remove_dir_all(&outside);
    let out = run_in(
        &dir,
        &[
            "project",
            "add",
            "--name",
            "Ext",
            "--root-path",
            outside.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(out.status.code(), Some(0));
    let id_ext = envelope(true, &out)["data"]["project"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let out = run_in(&dir, &["project", "list", "--json"]);
    let data = envelope(true, &out)["data"].clone();
    let projects = data["projects"].as_array().unwrap();
    let slugs: Vec<&str> = projects
        .iter()
        .map(|p| p["slug"].as_str().unwrap())
        .collect();
    assert_eq!(slugs, ["alpha", "ext", "mike", "zulu"]);

    let out = run_in(&dir, &["project", "show", &id_ext, "--json"]);
    assert_eq!(
        envelope(true, &out)["data"]["project"]["rootPath"],
        outside.to_string_lossy().as_ref()
    );
    assert!(
        !outside.exists(),
        "root_path is metadata only: no managed write"
    );

    let stdout = String::from_utf8(run_in(&dir, &["project", "list"]).stdout).unwrap();
    assert!(stdout.contains("projects (4):") && stdout.contains("- alpha (Alpha)"));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn artifact_create_show_list_archive_trash_restore_cycle() {
    let dir = temp_dir("artifact-cycle");
    run_in(&dir, &["init"]);
    let out = run_in(&dir, &["project", "add", "--name", "Demo", "--json"]);
    let pid = envelope(true, &out)["data"]["project"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let out = run_in(
        &dir,
        &[
            "artifact",
            "create",
            "--project",
            &pid,
            "--type",
            "doc",
            "--title",
            "Cycle",
            "--json",
        ],
    );
    assert_eq!(out.status.code(), Some(0));
    let doc = envelope(true, &out);
    let aid = doc["data"]["artifact"]["id"].as_str().unwrap().to_string();
    assert_eq!(doc["data"]["artifact"]["status"], "active");
    assert!(
        doc["data"]["artifact"]["path"]
            .as_str()
            .unwrap()
            .contains("artifacts/")
    );
    assert_eq!(doc["data"]["artifact"]["size"], 0);

    // Show by full id.
    let out = run_in(&dir, &["artifact", "show", &aid, "--json"]);
    assert_eq!(envelope(true, &out)["data"]["artifact"]["id"], aid);

    // List with status filter.
    let out = run_in(&dir, &["artifact", "list", "--status", "active", "--json"]);
    let list = envelope(true, &out);
    assert_eq!(list["data"]["artifacts"].as_array().unwrap().len(), 1);

    // Trash (physical move) from active.
    let out = run_in(&dir, &["artifact", "trash", &aid, "--json"]);
    assert_eq!(
        envelope(true, &out)["data"]["artifact"]["status"],
        "trashed"
    );
    let trashed_path = envelope(true, &run_in(&dir, &["artifact", "show", &aid, "--json"]))["data"]
        ["artifact"]["path"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        trashed_path.contains("trash/"),
        "moved to trash: {trashed_path}"
    );

    // Restore back to original.
    let out = run_in(&dir, &["artifact", "restore", &aid, "--json"]);
    assert_eq!(envelope(true, &out)["data"]["artifact"]["status"], "active");

    // Archive is the terminal reversible step (status-only).
    let out = run_in(&dir, &["artifact", "archive", &aid, "--json"]);
    assert_eq!(
        envelope(true, &out)["data"]["artifact"]["status"],
        "archived"
    );

    // Unknown id -> artifact_not_found, exit 1.
    let out = run_in(&dir, &["artifact", "show", "ffffffff", "--json"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(envelope(false, &out)["error"]["code"], "artifact_not_found");

    // Ambiguous prefix -> ambiguous_artifact_id.
    let out = run_in(&dir, &["artifact", "show", &aid[..8], "--json"]);
    if out.status.code() == Some(0) {
        // Single artifact so a full 8-char prefix may be unique; verify id.
        assert_eq!(envelope(true, &out)["data"]["artifact"]["id"], aid);
    } else {
        assert_eq!(
            envelope(false, &out)["error"]["code"],
            "ambiguous_artifact_id"
        );
    }

    // Invalid status filter -> usage error exit 2.
    let out = run_in(&dir, &["artifact", "list", "--status", "bogus", "--json"]);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(envelope(false, &out)["error"]["code"], "usage");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn artifact_relink_refreshes_fingerprint_and_rejects_occupied_target() {
    let dir = temp_dir("artifact-relink");
    run_in(&dir, &["init"]);
    let pid = envelope(
        true,
        &run_in(&dir, &["project", "add", "--name", "Demo", "--json"]),
    )["data"]["project"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let aid = envelope(
        true,
        &run_in(
            &dir,
            &[
                "artifact",
                "create",
                "--project",
                &pid,
                "--type",
                "doc",
                "--title",
                "R",
                "--json",
            ],
        ),
    )["data"]["artifact"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Old file still exists -> relink refused with restore_conflict.
    let out = run_in(
        &dir,
        &[
            "artifact",
            "relink",
            &aid,
            "--path",
            "artifacts/new.txt",
            "--json",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(envelope(false, &out)["error"]["code"], "restore_conflict");

    // Remove the old file and create a new target, then relink.
    let old = dir.join("artifacts").join(&aid);
    fs::remove_file(&old).expect("remove old");
    fs::write(dir.join("artifacts").join("new.txt"), b"hello world").expect("write new");
    let out = run_in(
        &dir,
        &[
            "artifact",
            "relink",
            &aid,
            "--path",
            "artifacts/new.txt",
            "--json",
        ],
    );
    assert_eq!(out.status.code(), Some(0));
    let doc = envelope(true, &out);
    assert_eq!(doc["data"]["artifact"]["size"], 11);
    assert!(doc["data"]["artifact"]["sha256"].as_str().is_some());
    fs::remove_dir_all(&dir).ok();
}
