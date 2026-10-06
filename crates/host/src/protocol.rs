//! The socket protocol: one JSON request line in; one JSON header line and the
//! raw payload out. The policy is a function over bytes with the clipboard
//! passed in, so it is provable without Wayland.

use crate::{Result, clipboard, clipboard::Query, image::Format};
use serde_json::{Value, json};
use std::io::{self, BufRead, BufReader, IoSlice, Read, Write};

/// The longest request line, newline included.
const REQUEST_LIMIT: usize = 4096;
const NOT_OFFERED: &str = "requested image type is not in host clipboard";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Request {
    /// List the offered supported images.
    Types,
    /// Read one image: the named format, or the most preferred one offered.
    Read(Option<Format>),
}

impl Request {
    /// Parses a request object. Its keys are within `op` and `type`, and for
    /// `types` the `type` is ignored whatever it holds.
    pub(crate) fn parse(line: &[u8]) -> Result<Self> {
        let request: Value = serde_json::from_slice(line)?;
        let object = request.as_object().ok_or("request must be an object")?;
        if object.keys().any(|key| key != "op" && key != "type") {
            return Err("unsupported request field".into());
        }
        match object
            .get("op")
            .and_then(Value::as_str)
            .ok_or("missing operation")?
        {
            "types" => Ok(Self::Types),
            "read" => Ok(Self::Read(match object.get("type") {
                None | Some(Value::Null) => None,
                Some(Value::String(mime)) if mime == "image" => None,
                Some(Value::String(mime)) => {
                    Some(Format::from_mime(mime.as_bytes()).ok_or(NOT_OFFERED)?)
                }
                Some(_) => return Err(NOT_OFFERED.into()),
            })),
            _ => Err("unsupported operation".into()),
        }
    }
}

/// The supported formats `listing` offers, in preference order, each once.
/// A line that is not a supported type is skipped, whatever its bytes.
fn offered(listing: &[u8]) -> impl Iterator<Item = Format> + '_ {
    Format::ALL.into_iter().filter(move |format| {
        clipboard::lines(listing).any(|line| line == format.mime().as_bytes())
    })
}

/// Answers one request line: one listing, then at most one read, whose bytes
/// must carry the signature of the format that was listed.
pub(crate) fn respond_with(
    request: &[u8],
    mut clipboard: impl FnMut(Query) -> Result<Vec<u8>>,
) -> Result<Vec<u8>> {
    let request = Request::parse(request)?;
    let listing = clipboard(Query::Types)?;
    let mut offered = offered(&listing);
    let format = match request {
        Request::Types => {
            let mut response = Vec::new();
            for format in offered {
                response.extend_from_slice(format.mime().as_bytes());
                response.push(b'\n');
            }
            return Ok(response);
        }
        Request::Read(None) => offered.next().ok_or("no image in host clipboard")?,
        Request::Read(Some(wanted)) => {
            if !offered.any(|format| format == wanted) {
                return Err(NOT_OFFERED.into());
            }
            wanted
        }
    };
    let data = clipboard(Query::Image(format))?;
    if !format.matches(&data) {
        return Err("clipboard changed or returned invalid image bytes".into());
    }
    Ok(data)
}

/// Reads one request line of at most [`REQUEST_LIMIT`] bytes, newline included.
pub(crate) fn read_request(reader: impl Read) -> Result<Vec<u8>> {
    let mut request = Vec::new();
    BufReader::new(reader)
        .take(REQUEST_LIMIT as u64 + 1)
        .read_until(b'\n', &mut request)?;
    if request.len() > REQUEST_LIMIT || !request.ends_with(b"\n") {
        return Err("invalid or oversized request".into());
    }
    Ok(request)
}

/// Writes the header line and the payload. The header is serialised into its
/// own buffer and leaves with the payload in one vectored write where the
/// writer takes it all, so the number of writes does not depend on the
/// header's content, and the payload is never copied to join it.
pub(crate) fn write_response(mut output: impl Write, result: Result<Vec<u8>>) -> io::Result<()> {
    let (header, data) = match result {
        Ok(data) => (json!({"ok":true,"size":data.len()}), data),
        Err(error) => (
            json!({"ok":false,"size":0,"error":error.to_string()}),
            Vec::new(),
        ),
    };
    let mut header = serde_json::to_vec(&header)?;
    header.push(b'\n');
    let mut parts = [IoSlice::new(&header), IoSlice::new(&data)];
    let mut parts = &mut parts[..];
    while !parts.is_empty() {
        match output.write_vectored(parts) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(written) => IoSlice::advance_slices(&mut parts, written),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    output.flush()
}

#[cfg(test)]
mod tests;
