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
