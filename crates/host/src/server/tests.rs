use super::*;
use crate::protocol::{read_request, write_response};

const BUDGET: Duration = Duration::from_millis(200);
/// Comfortably above `BUDGET` and far below the time either peer below keeps
/// going for, so only a total deadline passes.
const BOUND: Duration = Duration::from_secs(1);

/// Runs `peer` on the far end of a socket pair for at most three seconds.
fn with_peer(peer: impl FnOnce(UnixStream) + Send + 'static) -> UnixStream {
    let (near, far) = UnixStream::pair().unwrap();
    thread::spawn(move || peer(far));
    near.set_nonblocking(true).unwrap();
    near
}

#[test]
fn should_fail_a_trickled_request_within_its_deadline() {
    let stream = with_peer(|mut far| {
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(3) && far.write_all(b"{").is_ok() {
            thread::sleep(Duration::from_millis(20));
        }
    });
    let started = Instant::now();
    assert!(read_request(Deadline::new(&stream, BUDGET)).is_err());
    assert!(started.elapsed() < BOUND, "{:?}", started.elapsed());
}

#[test]
fn should_fail_a_slowly_read_response_within_its_deadline() {
    let stream = with_peer(|mut far| {
        let started = Instant::now();
        // Fast enough that no single write waits out the budget, too slow to
        // take 8 MiB before the peer gives up.
        let mut chunk = [0; 16 * 1024];
        while started.elapsed() < Duration::from_secs(3) && far.read(&mut chunk).is_ok() {
            thread::sleep(Duration::from_millis(10));
        }
    });
    let started = Instant::now();
    assert!(write_response(Deadline::new(&stream, BUDGET), Ok(vec![0; 8 << 20])).is_err());
    assert!(started.elapsed() < BOUND, "{:?}", started.elapsed());
}
