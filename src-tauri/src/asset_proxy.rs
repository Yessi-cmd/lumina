//! `lcu-asset://` serves LCU game-data images (champion, item, spell and profile icons)
//! to the webview without exposing LCU credentials, and caches them on disk.
//! On Windows the webview reaches it at `http://lcu-asset.localhost/<LCU path>`.

use std::path::{Path, PathBuf};

use tauri::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use tauri::http::{Request, Response, StatusCode};
use tauri::{AppHandle, Manager, UriSchemeContext, UriSchemeResponder, Wry};

use crate::error::{AppError, Result};
use crate::state::AppState;

pub const SCHEME: &str = "lcu-asset";
/// Only static game data is proxied; the rest of the LCU API stays private.
const ALLOWED_PREFIX: &str = "/lol-game-data/assets/";

pub fn handle(
    ctx: UriSchemeContext<'_, Wry>,
    request: Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let app = ctx.app_handle().clone();
    let path = request.uri().path().to_owned();
    tauri::async_runtime::spawn(async move {
        responder.respond(respond(&app, &path).await);
    });
}

async fn respond(app: &AppHandle, path: &str) -> Response<Vec<u8>> {
    let (status, content_type, body) = match load(app, path).await {
        Ok((body, content_type)) => (StatusCode::OK, content_type, body),
        Err(err) => {
            log::debug!("asset {path} unavailable: {err}");
            let body = err.to_string().into_bytes();
            (StatusCode::NOT_FOUND, "text/plain".to_owned(), body)
        }
    };
    let response = Response::builder()
        .status(status)
        .header(CONTENT_TYPE, content_type)
        .header(CACHE_CONTROL, "max-age=86400")
        .body(body);
    response.unwrap_or_default()
}

async fn load(app: &AppHandle, path: &str) -> Result<(Vec<u8>, String)> {
    if !path.starts_with(ALLOWED_PREFIX) || path.contains("..") {
        return Err(AppError::Message(format!("不允许的资源路径: {path}")));
    }

    let file = cache_path(app, path);
    if let Ok(body) = tokio::fs::read(&file).await {
        return Ok((body, content_type_for(path).to_owned()));
    }

    let session = app.state::<AppState>().session()?;
    let (body, content_type) = session.http.get_bytes(path).await?;
    if let Err(err) = write_cache(&file, &body).await {
        log::debug!("failed to cache {path}: {err}");
    }
    let fallback = content_type_for(path);
    let content_type = content_type.unwrap_or_else(|| fallback.to_owned());
    Ok((body, content_type))
}

fn cache_path(app: &AppHandle, path: &str) -> PathBuf {
    let root = app.path().app_cache_dir();
    let root = root.unwrap_or_else(|_| std::env::temp_dir());
    let relative = &path[ALLOWED_PREFIX.len()..];
    let name: String = relative.chars().map(sanitize).collect();
    root.join("lcu-assets").join(name.to_ascii_lowercase())
}

fn sanitize(c: char) -> char {
    if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
        c
    } else {
        '_'
    }
}

async fn write_cache(file: &Path, body: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = file.parent() {
        tokio::fs::create_dir_all(dir).await?;
    }
    tokio::fs::write(file, body).await
}

fn content_type_for(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or_default();
    match ext.to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_cache_names() {
        let name: String = "v1/champion-icons/1.png".chars().map(sanitize).collect();
        assert_eq!(name, "v1_champion-icons_1.png");
    }

    #[test]
    fn guesses_content_type() {
        assert_eq!(content_type_for("/a/B.PNG"), "image/png");
        assert_eq!(content_type_for("/a/b.jpg"), "image/jpeg");
    }
}
