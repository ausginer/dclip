use super::*;

#[test]
fn should_list_formats_in_declaration_order() {
    for (index, format) in Format::ALL.into_iter().enumerate() {
        assert_eq!(format as usize, index, "{format:?}");
    }
}

#[test]
fn should_accept_only_each_formats_own_signature() {
    let samples: [(Format, &[u8]); 5] = [
        (Format::Png, b"\x89PNG\r\n\x1a\nrest"),
        (Format::Jpeg, b"\xff\xd8\xff\xe0rest"),
        (Format::Jpg, b"\xff\xd8\xff\xe0rest"),
        (Format::Gif, b"GIF87arest"),
        (Format::Webp, b"RIFF\x00\x00\x00\x00WEBPrest"),
    ];
    for format in Format::ALL {
        for (owner, bytes) in samples {
            let shares_signature = matches!(
                (format, owner),
                (Format::Jpeg | Format::Jpg, Format::Jpeg | Format::Jpg)
            );
            assert_eq!(
                format.matches(bytes),
                format == owner || shares_signature,
                "{format:?} on {owner:?}"
            );
        }
    }
}

#[test]
fn should_accept_both_gif_versions() {
    assert!(Format::Gif.matches(b"GIF89arest"));
}

#[test]
fn should_parse_only_exact_supported_mime_types() {
    for format in Format::ALL {
        assert_eq!(Format::from_mime(format.mime().as_bytes()), Some(format));
    }
    for mime in [&b"image/bmp"[..], b"image/PNG", b"image/png ", b"image"] {
        assert_eq!(Format::from_mime(mime), None, "{mime:?}");
    }
}
