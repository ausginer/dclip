use super::*;
use std::{io::IoSlice, os::unix::net::UnixStream, thread};

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\x00\xff\x00hello\n";

fn mock(query: Query) -> Result<Vec<u8>> {
    Ok(match query {
        Query::Types => b"image/png\ntext/plain\n".to_vec(),
        Query::Image(_) | Query::Text => PNG.to_vec(),
    })
}

#[test]
fn should_list_only_supported_image_types() {
    assert_eq!(
        respond_with(br#"{"op":"types"}"#, mock).unwrap(),
        b"image/png\n"
    );
}

#[test]
fn should_return_exact_bytes_for_requested_type() {
    assert_eq!(
        respond_with(br#"{"op":"read","type":"image/png"}"#, mock).unwrap(),
        PNG
    );
}

#[test]
fn should_read_first_offered_image_when_type_is_null() {
    assert_eq!(
        respond_with(br#"{"op":"read","type":null}"#, mock).unwrap(),
        PNG
    );
}

#[test]
fn should_reject_invalid_requests() {
    for request in [
        "bad",
        "[]",
        r#"{"op":"write"}"#,
        r#"{"op":"read","type":42}"#,
        r#"{"op":"read","type":"image/jpeg"}"#,
        r#"{"op":"types","command":"evil"}"#,
    ] {
        assert!(respond_with(request.as_bytes(), mock).is_err(), "{request}");
    }
}

#[test]
fn should_reject_payload_with_wrong_magic() {
    assert!(
        respond_with(br#"{"op":"read","type":"image/png"}"#, |query| Ok(
            if query == Query::Types {
                b"image/png\n".to_vec()
            } else {
                b"text".to_vec()
            }
        ))
        .is_err()
    );
}

#[test]
fn should_frame_response_with_exact_size() {
    let mut output = Vec::new();
    write_response(&mut output, Ok(PNG.to_vec())).unwrap();
    let split = output.iter().position(|byte| *byte == b'\n').unwrap();
    let header: Value = serde_json::from_slice(&output[..split]).unwrap();
    assert_eq!(header["size"], json!(PNG.len()));
    assert_eq!(&output[split + 1..], PNG);
}

#[test]
fn should_preserve_binary_payload_across_a_socket() {
    let (mut left, mut right) = UnixStream::pair().unwrap();
    let writer = thread::spawn(move || write_response(&mut right, Ok(PNG.to_vec())).unwrap());
    let mut output = Vec::new();
    left.read_to_end(&mut output).unwrap();
    writer.join().unwrap();
    let split = output.iter().position(|byte| *byte == b'\n').unwrap();
    assert_eq!(&output[split + 1..], PNG);
}

#[test]
fn should_serve_an_image_when_another_listing_line_is_not_utf8() {
    let response = respond_with(br#"{"op":"read","type":"image/png"}"#, |query| {
        Ok(match query {
            Query::Types => b"image/png\ntext/x-\xff\n".to_vec(),
            Query::Image(_) | Query::Text => PNG.to_vec(),
        })
    });
    assert_eq!(response.unwrap(), PNG);
}

/// A writer that accepts everything it is offered and counts its calls.
#[derive(Default)]
struct Counting {
    bytes: Vec<u8>,
    calls: usize,
}

impl Write for Counting {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.calls += 1;
        self.bytes.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.calls += 1;
        for buf in bufs {
            self.bytes.extend_from_slice(buf);
        }
        Ok(bufs.iter().map(|buf| buf.len()).sum())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn should_frame_any_response_in_one_write_to_a_writer_that_takes_it_all() {
    for result in [
        Ok(PNG.to_vec()),
        Err("a long error message with \"quotes\" and \u{e9}".into()),
    ] {
        let expected_size = result.as_ref().map_or(0, Vec::len);
        let mut output = Counting::default();
        write_response(&mut output, result).unwrap();
        assert_eq!(output.calls, 1);
        let split = output.bytes.iter().position(|byte| *byte == b'\n').unwrap();
        let header: Value = serde_json::from_slice(&output.bytes[..split]).unwrap();
        assert_eq!(header["size"], json!(expected_size));
        assert_eq!(output.bytes.len() - split - 1, expected_size);
    }
}
