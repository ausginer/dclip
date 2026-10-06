use super::*;
use std::cell::RefCell;

const TEXT: &[u8] = b"copied\n\x00text";

/// A Wayland clipboard whose successive listings are `listings`, offering
/// `TEXT`, and which records every query it is asked.
struct Wayland<'a> {
    listings: RefCell<Vec<&'a [u8]>>,
    queries: RefCell<Vec<Query>>,
}

impl<'a> Wayland<'a> {
    fn new(listings: &[&'a [u8]]) -> Self {
        let mut listings = listings.to_vec();
        listings.reverse();
        Self {
            listings: RefCell::new(listings),
            queries: RefCell::new(Vec::new()),
        }
    }

    fn query(&self, query: Query) -> Result<Vec<u8>> {
        self.queries.borrow_mut().push(query);
        match query {
            Query::Types => Ok(self
                .listings
                .borrow_mut()
                .pop()
                .ok_or("listed too often")?
                .to_vec()),
            Query::Text => Ok(TEXT.to_vec()),
            Query::Image(_) => Err("asked for an image".into()),
        }
    }
}

/// Runs the sequence and returns what it wrote to X11, if anything.
fn sync(state: Option<&str>, wayland: &Wayland, x11: Result<Vec<u8>>) -> Option<Vec<u8>> {
    let mut written = None;
    sync_text(
        state,
        |query| wayland.query(query),
        || x11,
        |text| {
            written = Some(text);
            Ok(())
        },
    )
    .unwrap();
    written
}

#[test]
fn should_not_sync_text_when_an_image_is_offered() {
    assert!(!may_sync_text(b"image/png\ntext/plain\n"));
}

#[test]
fn should_sync_plain_text_offers() {
    assert!(may_sync_text(b"text/plain;charset=utf-8\ntext/plain\n"));
}

#[test]
fn should_not_sync_text_without_a_plain_text_offer() {
    assert!(!may_sync_text(b"text/html\n"));
}

#[test]
fn should_write_exactly_the_text_that_was_read() {
    let wayland = Wayland::new(&[b"text/plain\n", b"text/plain\n"]);
    assert_eq!(
        sync(None, &wayland, Ok(b"older".to_vec())),
        Some(TEXT.to_vec())
    );
}

#[test]
fn should_skip_the_write_when_x11_already_holds_the_text() {
    let wayland = Wayland::new(&[b"text/plain\n", b"text/plain\n"]);
    assert_eq!(sync(None, &wayland, Ok(TEXT.to_vec())), None);
}

#[test]
fn should_treat_a_failed_x11_read_as_different_text() {
    let wayland = Wayland::new(&[b"text/plain\n", b"text/plain\n"]);
    assert_eq!(
        sync(None, &wayland, Err("xsel failed".into())),
        Some(TEXT.to_vec())
    );
}

#[test]
fn should_skip_the_write_when_an_image_appears_by_the_second_listing() {
    let wayland = Wayland::new(&[b"text/plain\n", b"image/png\ntext/plain\n"]);
    assert_eq!(sync(None, &wayland, Ok(b"older".to_vec())), None);
    assert_eq!(
        *wayland.queries.borrow(),
        [Query::Types, Query::Text, Query::Types]
    );
}

#[test]
fn should_sync_only_in_the_data_and_sensitive_states() {
    for (state, synced) in [
        (Some("data"), true),
        (Some("sensitive"), true),
        (Some("clear"), false),
        (Some("nil"), false),
        (None, true),
    ] {
        let wayland = Wayland::new(&[b"text/plain\n", b"text/plain\n"]);
        assert_eq!(
            sync(state, &wayland, Ok(b"older".to_vec())).is_some(),
            synced,
            "{state:?}"
        );
    }
}

#[test]
fn should_sync_plain_text_when_another_listing_line_is_not_utf8() {
    assert!(may_sync_text(b"text/x-\xff\ntext/plain\n"));
}
