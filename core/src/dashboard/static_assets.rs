use include_dir::{include_dir, Dir};
use std::borrow::Cow;

static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/dashboard/static");

pub struct Asset {
    pub body: &'static [u8],
    pub content_type: Cow<'static, str>,
}

fn normalize(path: &str) -> &str {
    path.trim_start_matches('/')
}

fn guess_mime(path: &str) -> Cow<'static, str> {
    mime_guess::from_path(path)
        .first_raw()
        .map(|value| Cow::Owned(value.to_string()))
        .unwrap_or_else(|| Cow::Borrowed("application/octet-stream"))
}

pub fn get(path: &str) -> Option<Asset> {
    let normalized = normalize(path);
    let file = STATIC_DIR.get_file(normalized)?;
    Some(Asset {
        body: file.contents(),
        content_type: guess_mime(normalized),
    })
}

pub fn get_text(path: &str) -> Option<&'static str> {
    STATIC_DIR.get_file(normalize(path))?.contents_utf8()
}
