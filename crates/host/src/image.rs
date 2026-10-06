//! The image formats the bridge hands to the container, and how each is
//! recognised by its leading bytes.

/// The supported formats, declared in preference order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Format {
    Png,
    Jpeg,
    /// Requested from the clipboard by the exact string it offered, so it is
    /// a format of its own although its bytes are JPEG's.
    Jpg,
    Gif,
    Webp,
}

impl Format {
    /// Every format in declaration order, which is preference order: the order
    /// of a `types` response, and the choice of a read that names no type.
    pub(crate) const ALL: [Self; 5] = [Self::Png, Self::Jpeg, Self::Jpg, Self::Gif, Self::Webp];

    pub(crate) fn mime(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Jpg => "image/jpg",
            Self::Gif => "image/gif",
            Self::Webp => "image/webp",
        }
    }

    pub(crate) fn from_mime(mime: &[u8]) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|format| format.mime().as_bytes() == mime)
    }

    /// Whether `bytes` start with this format's signature.
    pub(crate) fn matches(self, bytes: &[u8]) -> bool {
        match self {
            Self::Png => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
            Self::Jpeg | Self::Jpg => bytes.starts_with(b"\xff\xd8\xff"),
            Self::Gif => bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"),
            Self::Webp => {
                bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP".as_slice())
            }
        }
    }
}

#[cfg(test)]
mod tests;
