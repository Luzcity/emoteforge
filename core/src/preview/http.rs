//! 依存なしの最小 HTTP/1.1 POST クライアント（localhost 用）。
//! Preview Bridge への JSON 送信専用。TLS は扱わない（http のみ）。

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    #[error("invalid url: {0}")]
    InvalidUrl(String),
    #[error("connection failed: {0}")]
    Connect(#[source] std::io::Error),
    #[error("io error: {0}")]
    Io(#[source] std::io::Error),
}

/// パース済み URL（http のみ）。
struct Url {
    host: String,
    port: u16,
    path: String,
}

fn parse_url(url: &str) -> Result<Url, HttpError> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| HttpError::InvalidUrl(url.to_string()))?;
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => (
            h.to_string(),
            p.parse::<u16>().map_err(|_| HttpError::InvalidUrl(url.to_string()))?,
        ),
        None => (authority.to_string(), 80),
    };
    if host.is_empty() {
        return Err(HttpError::InvalidUrl(url.to_string()));
    }
    Ok(Url {
        host,
        port,
        path: path.to_string(),
    })
}

/// JSON ボディを POST し、(ステータスコード, レスポンスボディ) を返す。
pub fn post_json(url: &str, body: &str, timeout: Duration) -> Result<(u16, String), HttpError> {
    let u = parse_url(url)?;
    let addr = format!("{}:{}", u.host, u.port);
    let mut stream = TcpStream::connect(&addr).map_err(HttpError::Connect)?;
    stream.set_read_timeout(Some(timeout)).map_err(HttpError::Io)?;
    stream.set_write_timeout(Some(timeout)).map_err(HttpError::Io)?;

    let req = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        u.path,
        u.host,
        body.as_bytes().len(),
        body
    );
    stream.write_all(req.as_bytes()).map_err(HttpError::Io)?;
    stream.flush().map_err(HttpError::Io)?;

    let mut raw = String::new();
    stream.read_to_string(&mut raw).map_err(HttpError::Io)?;

    let status = parse_status(&raw).unwrap_or(0);
    let body = raw.split_once("\r\n\r\n").map(|(_, b)| b.to_string()).unwrap_or_default();
    Ok((status, body))
}

fn parse_status(raw: &str) -> Option<u16> {
    // "HTTP/1.1 200 OK" の 2 トークン目。
    raw.lines().next()?.split_whitespace().nth(1)?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_url_with_port_and_path() {
        let u = parse_url("http://localhost:30120/emoteforge_bridge/preview").unwrap();
        assert_eq!(u.host, "localhost");
        assert_eq!(u.port, 30120);
        assert_eq!(u.path, "/emoteforge_bridge/preview");
    }

    #[test]
    fn defaults_port_and_path() {
        let u = parse_url("http://example.com").unwrap();
        assert_eq!(u.port, 80);
        assert_eq!(u.path, "/");
    }

    #[test]
    fn rejects_non_http() {
        assert!(parse_url("https://x").is_err());
        assert!(parse_url("ftp://x").is_err());
    }
}
