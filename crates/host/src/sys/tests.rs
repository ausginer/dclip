use super::*;

#[test]
fn should_report_peer_uid_of_current_user() {
    let (left, _right) = UnixStream::pair().unwrap();
    assert_eq!(peer_uid(&left).unwrap(), uid());
}
