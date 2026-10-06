use super::*;

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\x00\xff\x00hello\n";

#[test]
fn should_time_out_and_reap_a_slow_tool() {
    assert!(capture("sh", &["-c", "sleep 5"], None, Duration::from_millis(50)).is_err());
}

#[test]
fn should_preserve_nuls_and_newlines_in_tool_output() {
    assert_eq!(
        capture("cat", &[], Some(PNG.to_vec()), Duration::from_secs(2)).unwrap(),
        PNG
    );
}

#[test]
fn should_refuse_output_over_the_limit() {
    let limit = (LIMIT + 1).to_string();
    assert!(
        capture(
            "head",
            &["-c", &limit, "/dev/zero"],
            None,
            Duration::from_secs(10)
        )
        .is_err()
    );
}

/// Runs `script` under `sh` through `capture`, after it has put a background
/// `sleep` in its own process group, and returns the capture's outcome and
/// whether that `sleep` is still alive afterwards.
fn capture_leaving_a_group_member(script: &str) -> (Result<Vec<u8>>, bool) {
    let dir =
        std::env::temp_dir().join(format!("cch-group-{}-{}", std::process::id(), script.len()));
    std::fs::create_dir_all(&dir).unwrap();
    let pid_file = dir.join("pid");
    let script = format!(
        "sleep 5 </dev/null >/dev/null 2>&1 & echo $! > '{}'; {script}",
        pid_file.display()
    );
    let outcome = capture("sh", &["-c", &script], None, Duration::from_secs(2));
    let pid = std::fs::read_to_string(&pid_file).unwrap();
    let pid = pid.trim();
    let alive = Command::new("sh")
        .args(["-c", &format!("kill -0 {pid} 2>/dev/null")])
        .status()
        .unwrap()
        .success();
    let _ = Command::new("sh")
        .args(["-c", &format!("kill {pid} 2>/dev/null")])
        .status();
    let _ = std::fs::remove_dir_all(&dir);
    (outcome, alive)
}

#[test]
fn should_not_signal_the_group_of_a_reaped_child() {
    let (outcome, alive) = capture_leaving_a_group_member("exit 1");
    assert!(outcome.is_err());
    assert!(alive, "the group of a reaped child was signalled");
}

#[test]
fn should_leave_the_group_of_a_successful_tool_alone() {
    let (outcome, alive) = capture_leaving_a_group_member("printf done");
    assert_eq!(outcome.unwrap(), b"done");
    assert!(alive, "the group of a successful tool was signalled");
}
