use crate::support::Scratch;

/// A PNG signature followed by NULs, newlines and a high byte, so that a
/// payload which is truncated at a NUL or a newline, or reencoded, differs.
const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\x00\n\xff\x00\n\nend";

#[test]
fn should_list_offered_supported_images_in_preference_order() {
    let scratch = Scratch::new();
    scratch.clipboard(
        b"text/plain\nimage/webp\nimage/bmp\nimage/png\nimage/gif\nimage/png\n",
        b"",
    );
    let server = scratch.serve(&[]);
    let response = server.request(b"{\"op\":\"types\"}\n");
    assert_eq!(response.header, serde_json::json!({"ok": true, "size": 31}));
    assert_eq!(response.payload, b"image/png\nimage/gif\nimage/webp\n");
}

#[test]
fn should_read_the_exact_image_bytes() {
    let scratch = Scratch::new();
    scratch.clipboard(b"image/png\n", PNG);
    let server = scratch.serve(&[]);
    let response = server.request(b"{\"op\":\"read\",\"type\":\"image/png\"}\n");
    assert_eq!(response.header["size"], PNG.len());
    assert_eq!(response.payload, PNG);
}

#[test]
fn should_refuse_a_read_of_a_type_that_is_not_offered() {
    let scratch = Scratch::new();
    scratch.clipboard(b"image/png\n", PNG);
    let server = scratch.serve(&[]);
    let response = server.request(b"{\"op\":\"read\",\"type\":\"image/gif\"}\n");
    assert_eq!(response.header["ok"], false);
    assert_eq!(response.payload, b"");
}

#[test]
fn should_refuse_a_payload_whose_magic_does_not_match() {
    let scratch = Scratch::new();
    scratch.clipboard(b"image/png\n", b"GIF89a not a png");
    let server = scratch.serve(&[]);
    let response = server.request(b"{\"op\":\"read\",\"type\":\"image/png\"}\n");
    assert_eq!(response.header["ok"], false);
    assert_eq!(response.payload, b"");
}

#[test]
fn should_refuse_a_request_over_4096_bytes() {
    let scratch = Scratch::new();
    scratch.clipboard(b"image/png\n", PNG);
    let server = scratch.serve(&[]);
    let mut line = br#"{"op":"types","type":""#.to_vec();
    line.resize(4097 - 3, b'x');
    line.extend_from_slice(b"\"}\n");
    assert_eq!(line.len(), 4097);
    let response = server.request(&line);
    assert_eq!(response.header["ok"], false);
}

#[test]
fn should_deliver_an_image_at_the_64_mib_limit_byte_exact() {
    let scratch = Scratch::new();
    let mut image = vec![0; 64 * 1024 * 1024];
    image[..PNG.len()].copy_from_slice(PNG);
    let last = image.len() - 1;
    image[last] = 0xff;
    scratch.clipboard(b"image/png\n", &image);
    let server = scratch.serve(&[]);
    let response = server.request(b"{\"op\":\"read\",\"type\":\"image/png\"}\n");
    assert_eq!(
        response.header,
        serde_json::json!({"ok": true, "size": image.len()})
    );
    assert!(response.payload == image, "payload differs");
}
