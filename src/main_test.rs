use assert_cmd::Command;
use tempfile::{tempdir, TempDir};

#[test]
fn test_mk_ok() {
    let tmpdir = tempdir().unwrap();
    touch(&tmpdir, "input");
    assert_mk_ok(&tmpdir, &["output", ":", "input", "--", "touch", "output"]);
    assert_exists(&tmpdir, "output");
}

#[test]
fn test_mk_no_rebuild() {
    let tmpdir = tempdir().unwrap();
    touch(&tmpdir, "input");
    touch(&tmpdir, "output");
    assert_mk_ok(&tmpdir, &["output", ":", "input", "--", "false"]);
    assert_exists(&tmpdir, "output");
}

#[test]
fn test_mk_command_fails() {
    let tmpdir = tempdir().unwrap();
    touch(&tmpdir, "input");
    assert_mk_fails(&tmpdir, &["output", ":", "input", "--", "false"]);
    assert_not_exists(&tmpdir, "output");
}

#[test]
fn test_mk_missing_input() {
    let tmpdir = tempdir().unwrap();
    assert_mk_fails(&tmpdir, &["output", ":", "input", "--", "touch", "output"]);
    assert_not_exists(&tmpdir, "output");
}

#[test]
fn test_mk_missing_output() {
    let tmpdir = tempdir().unwrap();
    assert_mk_fails(&tmpdir, &["output", ":", "input", "--", "true"]);
    assert_not_exists(&tmpdir, "output");
}

#[test]
fn test_unknown_command() {
    let tmpdir = tempdir().unwrap();
    touch(&tmpdir, "input");
    assert_mk_fails(
        &tmpdir,
        &["output", ":", "input", "--", "thereisdefinitelynocommand"],
    );
    assert_not_exists(&tmpdir, "output");
}

#[test]
fn test_mk_output_with_no_input() {
    let tmpdir = tempdir().unwrap();
    assert_mk_ok(&tmpdir, &["output", "--", "touch", "output"]);
    assert_exists(&tmpdir, "output");
}

#[test]
fn test_mk_output_with_no_input_or_command() {
    let tmpdir = tempdir().unwrap();
    assert_mk_fails(&tmpdir, &["output"]);
    assert_not_exists(&tmpdir, "output");
}

#[test]
fn test_mk_no_input_doesnt_rebuild() {
    let tmpdir = tempdir().unwrap();
    assert_mk_ok(&tmpdir, &["output", "--", "touch", "output"]);
    assert_exists(&tmpdir, "output");
    let old_time = file_time(&tmpdir, "output");
    assert_mk_ok(&tmpdir, &["output", "--", "touch", "output"]);
    let new_time = file_time(&tmpdir, "output");
    assert_eq!(old_time, new_time);
}

#[test]
fn test_mk_input_is_dir() {
    let tmpdir = tempdir().unwrap();
    mkdir(&tmpdir, "input");
    touch(&tmpdir, "input/input");
    assert_mk_ok(&tmpdir, &["output", ":", "input", "--", "touch", "output"]);
    assert_exists(&tmpdir, "output");
    let before = file_time(&tmpdir, "output");
    touch(&tmpdir, "input/input");
    assert_mk_ok(&tmpdir, &["output", ":", "input", "--", "touch", "output"]);
    assert_exists(&tmpdir, "output");
    let after = file_time(&tmpdir, "output");
    assert!(before < after);
}

#[track_caller]
fn assert_mk_ok(tmpdir: &TempDir, args: &[&str]) {
    let mut cmd = Command::cargo_bin("mk").unwrap();
    cmd.env("MK_LOG", "trace");
    cmd.current_dir(tmpdir);
    cmd.args(args);
    let output = cmd.assert().get_output().clone();
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    println!("{}", stdout);
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();
    eprintln!("{}", stderr);
    assert_eq!(output.status.code().unwrap(), 0);
}

#[track_caller]
fn assert_mk_fails(tmpdir: &TempDir, args: &[&str]) {
    let mut cmd = Command::cargo_bin("mk").unwrap();
    cmd.current_dir(tmpdir);
    cmd.env("MK_LOG", "trace");
    cmd.args(args);
    let output = cmd.assert().get_output().clone();
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    println!("{}", stdout);
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();
    eprintln!("{}", stderr);
    assert_ne!(output.status.code().unwrap(), 0);
}

#[track_caller]
fn assert_exists(tmpdir: &TempDir, path: &str) {
    let path = tmpdir.path().join(path);
    assert!(path.exists());
}

#[track_caller]
fn assert_not_exists(tmpdir: &TempDir, path: &str) {
    let path = tmpdir.path().join(path);
    assert!(!path.exists());
}

#[track_caller]
fn mkdir(tmpdir: &TempDir, path: &str) {
    std::fs::create_dir(tmpdir.path().join(path)).unwrap();
}

#[track_caller]
fn touch(tmpdir: &TempDir, path: &str) {
    std::fs::File::create(tmpdir.path().join(path))
        .unwrap()
        .set_modified(std::time::SystemTime::now())
        .unwrap();
}

fn file_time(tmpdir: &TempDir, path: &str) -> std::time::SystemTime {
    let path = tmpdir.path().join(path);
    std::fs::metadata(path).unwrap().modified().unwrap()
}
