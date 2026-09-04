// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Answers `app://` requests: the embedded UI (or `src/` during development),
//! the embedded font, and cartridge content read from the folder beside the
//! exe. Picks the MIME type and builds the HTTP response.

// ########## THE APP:// PROTOCOL ##########

use std::borrow::Cow;
use std::fs;
use std::path::Path;

use common::cartridge as contract;
use rust_embed::RustEmbed;
use wry::http::{Request, Response, header::CONTENT_TYPE};

use crate::constants::UI_ASSET_EXTENSIONS;

/// The UI assets, baked into the exe at compile time and served over the
/// `app://` protocol at runtime. They live in `src/ui/`, apart from the Rust
/// sources and the seed files; the include list is what still holds if a
/// non-web file lands there — `isUiAsset` mirrors it at runtime.
#[derive(RustEmbed)]
#[folder = "src/ui/"]
#[include = "*.html"]
#[include = "*.css"]
#[include = "*.js"]
struct UiAssets;

/// The typeface, baked in the same way. A second struct because rust-embed
/// takes one folder each, and the font lives in `launcher/assets/fonts/` rather
/// than in `src/` beside the Rust sources.
///
/// Embedded rather than shipped on the cartridge: a font beside the exe can be
/// deleted or missed by a hand-copy, and the launcher would then silently fall
/// back to a system face.
#[derive(RustEmbed)]
#[folder = "assets/"]
#[include = "fonts/*.woff2"]
struct FontAssets;

/// Serves the UI from the baked-in `src/` assets, and `images/...` /
/// `games/...` straight from the content folder beside the exe, so paths in
/// catalog.json and `<img>` tags are just relative to the launcher's folder.
pub fn handleRequest(base_dir: &Path, request: Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let mut path = request.uri().path().trim_start_matches('/');
    if path.is_empty() {
        path = "index.html";
    }

    // The typeface, baked in. Answered before the disk is consulted and from a
    // URL prefix nothing on the cartridge uses, so there is never a question of
    // which source owns a given path.
    if let Some(file) = FontAssets::get(path) {
        let bytes = file.data.into_owned();
        let mime = mimeTypeFor(Path::new(path), &bytes);
        return okResponse(mime, Cow::Owned(bytes));
    }

    // Cartridge content lives on disk beside the exe.
    //
    // `assets/` is where cover art lives now; `images/` is the same thing under
    // the name it had before, still served so a cartridge written by an older
    // installer keeps working. Which one a given cartridge uses is not decided
    // here at all — the path comes out of its own catalog.json, and this only
    // says which prefixes are allowed to name content rather than UI.
    if path.starts_with("assets/")
        || startsWithDir(path, contract::LEGACY_IMAGES_DIR)
        || startsWithDir(path, contract::GAMES_DIR)
    {
        return match fs::read(base_dir.join(path)) {
            Ok(bytes) => {
                let mime = mimeTypeFor(Path::new(path), &bytes);
                okResponse(mime, Cow::Owned(bytes))
            }
            Err(_) => notFound(),
        };
    }

    // Everything else is a UI asset, and only the web files count as one:
    // `src/ui/` is meant to hold nothing else, and neither the live path below
    // nor rust-embed should hand out anything that turns up there anyway.
    if !isUiAsset(path) {
        return notFound();
    }

    // Prefer the live file from the source `src/ui/` folder when it exists —
    // under `cargo run` that's the repo, so edits show up on the next launch
    // with no rebuild. When it's absent (the deployed cartridge has no source
    // tree), fall back to the copy baked into the exe by rust-embed.
    let source_ui = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("ui")
        .join(path);
    if let Ok(bytes) = fs::read(&source_ui) {
        let mime = mimeTypeFor(Path::new(path), &bytes);
        return okResponse(mime, Cow::Owned(bytes));
    }

    match UiAssets::get(path) {
        Some(file) => {
            let bytes = file.data.into_owned();
            let mime = mimeTypeFor(Path::new(path), &bytes);
            okResponse(mime, Cow::Owned(bytes))
        }
        None => notFound(),
    }
}

/// Whether `path` names something under top-level folder `dir` — `dir` itself
/// followed by `/`, not merely a longer name sharing its prefix (`games/` vs.
/// `games2/`).
fn startsWithDir(path: &str, dir: &str) -> bool {
    path.strip_prefix(dir)
        .is_some_and(|rest| rest.starts_with('/'))
}

fn okResponse(mime: &'static str, body: Cow<'static, [u8]>) -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .header(CONTENT_TYPE, mime)
        // Never let WebView2 cache app:// responses in its data folder:
        // otherwise it serves a stale index.html forever and edits (or
        // swapped-in cover art) silently don't show up.
        .header("Cache-Control", "no-store")
        .body(body)
        .unwrap()
}

fn notFound() -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(404)
        .body(Cow::Borrowed(&[][..]))
        .unwrap()
}

/// Whether an `app://` path names one of the web files that make up the UI.
/// Mirrors the rust-embed include list on `UiAssets`, so the dev (live from
/// `src/`) and deployed (embedded) paths serve exactly the same set.
fn isUiAsset(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| UI_ASSET_EXTENSIONS.contains(&ext))
}

/// Sniffs the actual file content for images instead of trusting the
/// extension: cover art dropped into `images/` isn't always what its name
/// claims (e.g. an animated cover saved as `.png` that's actually WebP).
fn mimeTypeFor(path: &Path, content: &[u8]) -> &'static str {
    if content.starts_with(b"\x89PNG\r\n\x1a\n") {
        return "image/png";
    }
    if content.len() >= 12 && &content[0..4] == b"RIFF" && &content[8..12] == b"WEBP" {
        return "image/webp";
    }
    if content.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return "image/jpeg";
    }

    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html",
        Some("css") => "text/css",
        Some("js") => "text/javascript",
        Some("woff2") => "font/woff2",
        Some("json") => "application/json",
        _ => "application/octet-stream",
    }
}
