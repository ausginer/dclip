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

#[test]
fn should_refuse_a_peer_whose_uid_is_not_allowed() {
    let (stream, _peer) = UnixStream::pair().unwrap();
    let uid = sys::uid();
    let refusal = admit(&stream, &HashSet::new(), &AtomicUsize::new(0))
        .err()
        .unwrap();
    assert_eq!(
        refusal.to_string(),
        format!("host UID {uid} is not allowed; add --allow-uid {uid} on Fedora")
    );
}

#[test]
fn should_refuse_a_connection_while_every_place_is_taken() {
    let (stream, _peer) = UnixStream::pair().unwrap();
    let active = AtomicUsize::new(WORKERS);
    let refusal = admit(&stream, &HashSet::from([sys::uid()]), &active)
        .err()
        .unwrap();
    assert_eq!(
        refusal.to_string(),
        "too many concurrent clipboard requests"
    );
    assert_eq!(active.load(Ordering::Relaxed), WORKERS);
}

#[test]
fn should_release_a_place_when_its_connection_is_done() {
    let (stream, _peer) = UnixStream::pair().unwrap();
    let active = AtomicUsize::new(WORKERS - 1);
    let slot = admit(&stream, &HashSet::from([sys::uid()]), &active).unwrap();
    assert_eq!(active.load(Ordering::Relaxed), WORKERS);
    drop(slot);
    assert_eq!(active.load(Ordering::Relaxed), WORKERS - 1);
}

/// A pipe that is readable from the start, as the wake pipe is once shutdown
/// has been requested.
fn shutdown_requested() -> io::PipeReader {
    let (reader, mut writer) = io::pipe().unwrap();
    writer.write_all(&[1]).unwrap();
    reader
}

#[test]
fn should_turn_away_a_request_still_arriving_at_shutdown() {
    let stream = with_peer(|mut far| {
        far.write_all(b"{\"op\":").unwrap();
        thread::sleep(Duration::from_secs(3));
    });
    let shutdown = shutdown_requested();
    let started = Instant::now();
    let refusal = read_request(Deadline::new(&stream, BOUND).or_until_shutdown(shutdown.as_fd()))
        .err()
        .unwrap();
    assert_eq!(refusal.to_string(), "bridge is shutting down");
    assert!(started.elapsed() < BUDGET, "{:?}", started.elapsed());
}

#[test]
fn should_read_a_request_that_arrived_before_shutdown() {
    let stream = with_peer(|mut far| {
        far.write_all(b"{\"op\":\"types\"}\n").unwrap();
        thread::sleep(Duration::from_secs(3));
    });
    // The whole line is in the socket before the read starts.
    thread::sleep(Duration::from_millis(50));
    let shutdown = shutdown_requested();
    let request =
        read_request(Deadline::new(&stream, BOUND).or_until_shutdown(shutdown.as_fd())).unwrap();
    assert_eq!(request, b"{\"op\":\"types\"}\n");
}
