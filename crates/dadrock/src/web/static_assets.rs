#[cfg(dadrock_no_js_build)]
use std::borrow::Cow;

#[cfg(not(dadrock_no_js_build))]
use rust_embed::RustEmbed;

use super::http::{static_response, HttpResponse};

#[cfg(not(dadrock_no_js_build))]
#[derive(RustEmbed)]
#[folder = "web/build/"]
#[prefix = ""]
struct WebAssets;

#[cfg(not(dadrock_no_js_build))]
pub(super) fn web_response(path: &str) -> Option<HttpResponse> {
    let trimmed = path.trim_start_matches('/');
    let asset_path = if trimmed.is_empty() || !trimmed.contains('.') {
        "index.html"
    } else {
        trimmed
    };
    let file = WebAssets::get(asset_path)?;
    let mime = mime_guess::from_path(asset_path).first_or_octet_stream();
    Some(static_response(
        200,
        web_content_type(mime.as_ref()),
        file.data,
    ))
}

#[cfg(dadrock_no_js_build)]
pub(super) fn web_response(path: &str) -> Option<HttpResponse> {
    let trimmed = path.trim_start_matches('/');
    if !trimmed.is_empty() && trimmed.contains('.') {
        return None;
    }

    Some(static_response(
        200,
        "text/html; charset=utf-8",
        Cow::Borrowed(NO_JS_BUILD_PREVIEW_HTML.as_bytes()),
    ))
}

#[cfg(dadrock_no_js_build)]
const NO_JS_BUILD_PREVIEW_HTML: &str = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Dadrock Preview</title>
    <style>
      body {
        margin: 0;
        font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
        color: #1f2933;
        background: #f6f7f9;
      }
      main {
        max-width: 42rem;
        margin: 12vh auto;
        padding: 2rem;
        line-height: 1.55;
      }
      code {
        background: #e9edf2;
        border-radius: 0.25rem;
        padding: 0.1rem 0.25rem;
      }
    </style>
  </head>
  <body>
    <main>
      <h1>Dadrock was installed without the web UI assets</h1>
      <p>This binary can run CLI and API commands, but the preview UI was not embedded because the JavaScript build was skipped.</p>
      <p>Install Bun, then run <code>make install</code> again to build and embed the preview UI.</p>
    </main>
  </body>
</html>
"#;

#[cfg(not(dadrock_no_js_build))]
fn web_content_type(mime: &str) -> &str {
    match mime {
        "text/html" => "text/html; charset=utf-8",
        "text/css" => "text/css; charset=utf-8",
        "text/javascript" | "application/javascript" => "text/javascript; charset=utf-8",
        other => other,
    }
}
