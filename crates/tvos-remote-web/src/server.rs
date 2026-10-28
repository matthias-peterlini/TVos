//! Tiny HTTP server for the phone: remote control, and a panel to install / remove apps.
//!
//! Anything that changes what runs on the TV (install, remove) needs the PIN shown on the TV screen
//! in the `X-Pin` header; five wrong guesses lock the panel for a minute. Key presses stay open.

use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tvos_core::input::RemoteKey;
use tvos_store::{AppKind, MAX_PACKAGE_BYTES};

const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_FAILED_PINS: u32 = 5;
const LOCK_SECS: u64 = 60;

struct State {
    tx: Sender<RemoteKey>,
    pin: String,
    apps_dir: PathBuf,
    failed: AtomicU32,
    locked_until: Mutex<Option<Instant>>,
}

pub struct RemoteWebServer {
    port: u16,
    state: Arc<State>,
}

/// 4 random digits (from the OS if possible, otherwise from the clock).
fn make_pin() -> String {
    let mut b = [0u8; 4];
    let ok = std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut b)).is_ok();
    let n = if ok {
        u32::from_le_bytes(b)
    } else {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.subsec_nanos() ^ std::process::id())
    };
    format!("{:04}", n % 10_000)
}

impl RemoteWebServer {
    pub fn new(port: u16, event_tx: Sender<RemoteKey>) -> Self {
        let state = State {
            tx: event_tx,
            pin: make_pin(),
            apps_dir: tvos_store::apps_dir(),
            failed: AtomicU32::new(0),
            locked_until: Mutex::new(None),
        };
        Self { port, state: Arc::new(state) }
    }

    /// The code the phone must enter to install or remove apps (shown on the TV).
    pub fn pin(&self) -> &str {
        &self.state.pin
    }

    pub fn start(&self) -> std::io::Result<()> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port))?;
        let state = self.state.clone();
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let st = state.clone();
                thread::spawn(move || handle_client(stream, &st));
            }
        });
        Ok(())
    }
}

struct Request {
    method: String,
    path: String,
    query: String,
    pin: String,
    body: Vec<u8>,
}

fn read_request(stream: &mut TcpStream) -> Option<Request> {
    stream.set_read_timeout(Some(Duration::from_secs(20))).ok()?;
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8192];
    let header_end = loop {
        if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break p;
        }
        if buf.len() > MAX_HEADER_BYTES {
            return None;
        }
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
    };
    let head = String::from_utf8_lossy(&buf[..header_end]).into_owned();
    let mut lines = head.lines();
    let mut first = lines.next()?.split_whitespace();
    let method = first.next()?.to_string();
    let target = first.next()?.to_string();
    let (path, query) = target.split_once('?').map_or((target.clone(), String::new()), |(p, q)| (p.to_string(), q.to_string()));

    let (mut content_length, mut pin) = (0usize, String::new());
    for l in lines {
        if let Some((k, v)) = l.split_once(':') {
            match k.trim().to_ascii_lowercase().as_str() {
                "content-length" => content_length = v.trim().parse().ok()?,
                "x-pin" => pin = v.trim().to_string(),
                _ => {}
            }
        }
    }
    if content_length > MAX_PACKAGE_BYTES + 1024 * 1024 {
        return None;
    }
    let mut body = buf[header_end + 4..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    body.truncate(content_length);
    Some(Request { method, path, query, pin, body })
}

fn respond(stream: &mut TcpStream, status: &str, content_type: &str, body: &[u8], cors: bool) {
    let cors = if cors { "Access-Control-Allow-Origin: *\r\n" } else { "" };
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n{cors}Cache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
}

fn respond_json(stream: &mut TcpStream, status: &str, v: Value) {
    respond(stream, status, "application/json", v.to_string().as_bytes(), false);
}

fn query_param(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|kv| kv.strip_prefix(key)?.strip_prefix('=')).map(str::to_string)
}

fn key_from_name(name: &str) -> Option<RemoteKey> {
    Some(match name.to_lowercase().as_str() {
        "up" => RemoteKey::Up,
        "down" => RemoteKey::Down,
        "left" => RemoteKey::Left,
        "right" => RemoteKey::Right,
        "select" | "ok" => RemoteKey::Select,
        "back" => RemoteKey::Back,
        "home" => RemoteKey::Home,
        "playpause" | "play" => RemoteKey::PlayPause,
        "volup" => RemoteKey::VolumeUp,
        "voldown" => RemoteKey::VolumeDown,
        _ => return None,
    })
}

/// Check the PIN, with a lockout after too many failures.
fn check_pin(st: &State, given: &str) -> Result<(), &'static str> {
    let mut lock = st.locked_until.lock().unwrap();
    if let Some(t) = *lock {
        if Instant::now() < t {
            return Err("Troppi tentativi: riprova tra un minuto");
        }
        *lock = None;
        st.failed.store(0, Ordering::SeqCst);
    }
    if !given.is_empty() && given == st.pin {
        st.failed.store(0, Ordering::SeqCst);
        return Ok(());
    }
    if st.failed.fetch_add(1, Ordering::SeqCst) + 1 >= MAX_FAILED_PINS {
        *lock = Some(Instant::now() + Duration::from_secs(LOCK_SECS));
    }
    Err("PIN errato: lo trovi sullo schermo della TV")
}

fn app_json(a: &tvos_store::InstalledApp) -> Value {
    let kind = if a.kind == AppKind::Web { "web" } else { "native" };
    json!({ "id": a.id, "name": a.name, "type": kind, "url": a.url })
}

fn handle_client(mut stream: TcpStream, st: &State) {
    let Some(req) = read_request(&mut stream) else {
        return respond(&mut stream, "400 Bad Request", "text/plain", b"bad request", false);
    };
    let page = |s: &str| s.as_bytes().to_vec();
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/") | ("GET", "/index.html") => {
            respond(&mut stream, "200 OK", "text/html; charset=utf-8", &page(include_str!("remote.html")), false)
        }
        ("GET", "/apps") => respond(&mut stream, "200 OK", "text/html; charset=utf-8", &page(include_str!("apps.html")), false),
        ("GET" | "POST", "/api/key") => {
            if let Some(k) = query_param(&req.query, "key").as_deref().and_then(key_from_name) {
                let _ = st.tx.send(k);
            }
            respond(&mut stream, "200 OK", "application/json", br#"{"status":"ok"}"#, true);
        }
        ("GET", "/api/apps") => {
            let apps: Vec<Value> = tvos_store::list(&st.apps_dir).iter().map(app_json).collect();
            respond_json(&mut stream, "200 OK", json!({ "apps": apps }));
        }
        ("POST", "/api/install-web") | ("POST", "/api/install-package") | ("POST", "/api/remove") => {
            if let Err(msg) = check_pin(st, &req.pin) {
                return respond_json(&mut stream, "403 Forbidden", json!({ "error": msg }));
            }
            let result: Result<Value, String> = match req.path.as_str() {
                "/api/install-web" => serde_json::from_slice::<Value>(&req.body)
                    .map_err(|_| "Richiesta non valida".to_string())
                    .and_then(|v| {
                        let color = v["color"].as_str().and_then(tvos_store::parse_color);
                        tvos_store::install_web(&st.apps_dir, v["name"].as_str().unwrap_or(""), v["url"].as_str().unwrap_or(""), color)
                    })
                    .map(|a| app_json(&a)),
                "/api/install-package" => tvos_store::install_package(&st.apps_dir, &req.body).map(|a| app_json(&a)),
                _ => serde_json::from_slice::<Value>(&req.body)
                    .map_err(|_| "Richiesta non valida".to_string())
                    .and_then(|v| tvos_store::remove(&st.apps_dir, v["id"].as_str().unwrap_or("")))
                    .map(|_| json!({ "removed": true })),
            };
            match result {
                Ok(v) => respond_json(&mut stream, "200 OK", json!({ "ok": true, "app": v })),
                Err(e) => respond_json(&mut stream, "400 Bad Request", json!({ "error": e })),
            }
        }
        _ => respond(&mut stream, "404 Not Found", "text/plain", b"not found", false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    fn server(dir: &str) -> (u16, String, PathBuf) {
        let d = std::env::temp_dir().join(format!("tvos-web-{dir}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let (tx, _rx) = channel();
        let mut s = RemoteWebServer::new(0, tx);
        // bind to an ephemeral port by hand so the test knows it
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        Arc::get_mut(&mut s.state).unwrap().apps_dir = d.clone();
        let st = s.state.clone();
        let pin = st.pin.clone();
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let st = st.clone();
                thread::spawn(move || handle_client(stream, &st));
            }
        });
        std::mem::forget(s);
        (port, pin, d)
    }

    fn http(port: u16, method: &str, path: &str, pin: &str, body: &[u8]) -> (u16, String) {
        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let head = format!("{method} {path} HTTP/1.1\r\nHost: x\r\nX-Pin: {pin}\r\nContent-Length: {}\r\n\r\n", body.len());
        s.write_all(head.as_bytes()).unwrap();
        s.write_all(body).unwrap();
        let mut out = Vec::new();
        s.read_to_end(&mut out).unwrap();
        let text = String::from_utf8_lossy(&out).into_owned();
        let code = text.split_whitespace().nth(1).unwrap().parse().unwrap();
        (code, text.split("\r\n\r\n").nth(1).unwrap_or("").to_string())
    }

    #[test]
    fn install_requires_the_pin_and_works_with_it() {
        let (port, pin, dir) = server("pin");
        let body = br##"{"name":"Netflix","url":"https://www.netflix.com","color":"#E50914"}"##;
        assert_eq!(http(port, "POST", "/api/install-web", "0000x", body).0, 403);
        assert!(tvos_store::list(&dir).is_empty());
        let (code, _) = http(port, "POST", "/api/install-web", &pin, body);
        assert_eq!(code, 200);
        let (_, list) = http(port, "GET", "/api/apps", "", b"");
        assert!(list.contains("Netflix"));
        let (code, _) = http(port, "POST", "/api/remove", &pin, br#"{"id":"netflix"}"#);
        assert_eq!(code, 200);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn five_wrong_pins_lock_the_panel() {
        let (port, pin, dir) = server("lock");
        for _ in 0..5 {
            assert_eq!(http(port, "POST", "/api/remove", "nope", b"{}").0, 403);
        }
        // even the right PIN is refused while locked
        let (code, body) = http(port, "POST", "/api/remove", &pin, br#"{"id":"x"}"#);
        assert_eq!(code, 403);
        assert!(body.contains("Troppi tentativi"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn large_uploads_are_read_completely() {
        let (port, pin, dir) = server("big");
        // 3 MB of garbage: must reach the installer (and be rejected as "not a zip"), not be cut at 2 KB
        let (code, body) = http(port, "POST", "/api/install-package", &pin, &vec![7u8; 3 * 1024 * 1024]);
        assert_eq!(code, 400);
        assert!(body.contains("zip"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
