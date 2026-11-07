//! Unswitched native HTTP boundary. No product routes, store writes or ACKs.
//! One request per connection; strict framing and bounded concurrent workers.
#![allow(dead_code)]
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;

pub const BODY_LIMIT: usize = 32768;
pub const CALENDAR_LIMIT: usize = 2 * 1048576;
const HEADER_LIMIT: usize = 16384;
const MAX_WORKERS: usize = 8;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Failure {
    pub status: u16,
    pub code: &'static str,
}
fn fail(status: u16, code: &'static str) -> Failure {
    Failure { status, code }
}
type Result<T> = std::result::Result<T, Failure>;
#[derive(Debug)]
pub struct Request {
    pub method: String,
    pub target: String,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}
impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(String::as_str)
    }
}
pub struct Response {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}
impl Response {
    pub fn bytes(status: u16, content_type: &str, body: Vec<u8>) -> Self {
        let mut headers = BTreeMap::new();
        headers.insert("Content-Type".into(), content_type.into());
        headers.insert("Cache-Control".into(), "no-store".into());
        headers.insert("X-Content-Type-Options".into(), "nosniff".into());
        headers.insert("Referrer-Policy".into(), "no-referrer".into());
        Self {
            status,
            headers,
            body,
        }
    }
    pub fn json(status: u16, value: serde_json::Value) -> Self {
        Self::bytes(
            status,
            "application/json; charset=utf-8",
            serde_json::to_vec(&value).unwrap(),
        )
    }
    pub fn error(e: Failure) -> Self {
        Self::json(
            e.status,
            serde_json::json!({"ok":false,"error":{"code":e.code}}),
        )
    }
}
#[derive(Clone, Copy)]
pub struct Limits {
    pub headers: Duration,
    pub body: Duration,
    pub write: Duration,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            headers: Duration::from_secs(10),
            body: Duration::from_secs(15),
            write: Duration::from_secs(5),
        }
    }
}
struct Input<'a> {
    stream: &'a mut TcpStream,
    buffer: Vec<u8>,
    at: usize,
    deadline: Instant,
}
impl Input<'_> {
    fn fill(&mut self) -> Result<()> {
        let left = self
            .deadline
            .checked_duration_since(Instant::now())
            .ok_or(fail(408, "REQUEST_TIMEOUT"))?;
        self.stream
            .set_read_timeout(Some(left))
            .map_err(|_| fail(500, "LOCAL_OPERATION_FAILED"))?;
        let mut b = [0u8; 4096];
        let n = self.stream.read(&mut b).map_err(|e| {
            if matches!(
                e.kind(),
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
            ) {
                fail(408, "REQUEST_TIMEOUT")
            } else {
                fail(400, "BAD_REQUEST")
            }
        })?;
        if n == 0 {
            return Err(fail(400, "BAD_REQUEST"));
        }
        if self.at > 0 {
            self.buffer.drain(..self.at);
            self.at = 0
        }
        self.buffer.extend_from_slice(&b[..n]);
        Ok(())
    }
    fn line(&mut self, max: usize) -> Result<Vec<u8>> {
        loop {
            let rest = &self.buffer[self.at..];
            if let Some(end) = rest.windows(2).position(|w| w == b"\r\n") {
                if end > max {
                    return Err(fail(400, "BAD_REQUEST"));
                }
                let v = rest[..end].to_vec();
                self.at += end + 2;
                return Ok(v);
            }
            if rest.len() > max {
                return Err(fail(400, "BAD_REQUEST"));
            }
            self.fill()?
        }
    }
    fn take(&mut self, n: usize, out: &mut Vec<u8>) -> Result<()> {
        let mut left = n;
        while left > 0 {
            if self.at == self.buffer.len() {
                self.fill()?
            }
            let count = left.min(self.buffer.len() - self.at);
            out.extend_from_slice(&self.buffer[self.at..self.at + count]);
            self.at += count;
            left -= count;
        }
        Ok(())
    }
}
fn read_request(stream: &mut TcpStream, limits: Limits) -> Result<Request> {
    let mut input = Input {
        stream,
        buffer: Vec::new(),
        at: 0,
        deadline: Instant::now() + limits.headers,
    };
    let end = loop {
        if let Some(n) = input.buffer.windows(4).position(|w| w == b"\r\n\r\n") {
            if n + 4 > HEADER_LIMIT {
                return Err(fail(431, "HEADERS_TOO_LARGE"));
            }
            break n + 4;
        }
        if input.buffer.len() >= HEADER_LIMIT {
            return Err(fail(431, "HEADERS_TOO_LARGE"));
        }
        input.fill()?
    };
    if input.buffer[..end].iter().enumerate().any(|(i, b)| {
        (*b == b'\n' && (i == 0 || input.buffer[i - 1] != b'\r'))
            || (*b == b'\r' && input.buffer.get(i + 1) != Some(&b'\n'))
    }) {
        return Err(fail(400, "BAD_REQUEST"));
    }
    let mut slots = [httparse::EMPTY_HEADER; 64];
    let mut parsed = httparse::Request::new(&mut slots);
    let status = parsed
        .parse(&input.buffer[..end])
        .map_err(|_| fail(400, "BAD_REQUEST"))?;
    if !status.is_complete() || parsed.version != Some(1) {
        return Err(fail(400, "BAD_REQUEST"));
    }
    let method = parsed.method.ok_or(fail(400, "BAD_REQUEST"))?.to_owned();
    let target = parsed.path.ok_or(fail(400, "BAD_REQUEST"))?.to_owned();
    validate_target(&target)?;
    let mut headers = BTreeMap::new();
    for h in parsed.headers.iter() {
        let key = h.name.to_ascii_lowercase();
        let v = std::str::from_utf8(h.value)
            .map_err(|_| fail(400, "BAD_REQUEST"))?
            .trim_matches([' ', '\t']);
        if v.bytes().any(|b| b < 32 && b != b'\t' || b == 127)
            || headers.insert(key, v.to_owned()).is_some()
        {
            return Err(fail(400, "AMBIGUOUS_HEADERS"));
        }
    }
    if headers.contains_key("expect") {
        return Err(fail(417, "EXPECTATION_FAILED"));
    }
    if headers.contains_key("upgrade") {
        return Err(fail(400, "BAD_REQUEST"));
    }
    let transfer = headers.get("transfer-encoding");
    let length = headers.get("content-length");
    if transfer.is_some() && length.is_some() {
        return Err(fail(400, "AMBIGUOUS_FRAMING"));
    }
    let cap = if target == "/api/calendar/import" {
        CALENDAR_LIMIT
    } else {
        BODY_LIMIT
    };
    input.at = end;
    input.deadline = Instant::now() + limits.body;
    let mut body = Vec::new();
    if let Some(te) = transfer {
        if !te.eq_ignore_ascii_case("chunked") {
            return Err(fail(400, "AMBIGUOUS_FRAMING"));
        }
        let mut chunks = 0;
        loop {
            chunks += 1;
            if chunks > 4096 {
                return Err(fail(413, "BODY_TOO_LARGE"));
            }
            let line = input.line(1024)?;
            let hex = line.split(|b| *b == b';').next().unwrap();
            if hex.is_empty()
                || hex.len() > 16
                || !hex.iter().all(u8::is_ascii_hexdigit)
                || line.iter().any(|b| !b.is_ascii_graphic() && *b != b' ')
            {
                return Err(fail(400, "BAD_REQUEST"));
            }
            let n = usize::from_str_radix(std::str::from_utf8(hex).unwrap(), 16)
                .map_err(|_| fail(413, "BODY_TOO_LARGE"))?;
            if n > cap - body.len() {
                return Err(fail(413, "BODY_TOO_LARGE"));
            }
            if n == 0 {
                if !input.line(0)?.is_empty() {
                    return Err(fail(400, "TRAILERS_DENIED"));
                }
                break;
            }
            input.take(n, &mut body)?;
            if !input.line(0)?.is_empty() {
                return Err(fail(400, "BAD_REQUEST"));
            }
        }
    } else if let Some(cl) = length {
        if cl.is_empty() || !cl.bytes().all(|b| b.is_ascii_digit()) {
            return Err(fail(400, "AMBIGUOUS_FRAMING"));
        }
        let n = cl
            .parse::<usize>()
            .map_err(|_| fail(413, "BODY_TOO_LARGE"))?;
        if n > cap {
            return Err(fail(413, "BODY_TOO_LARGE"));
        }
        input.take(n, &mut body)?;
    }
    if ["GET", "HEAD"].contains(&method.as_str()) && !body.is_empty() {
        return Err(fail(400, "GET_BODY_DENIED"));
    }
    Ok(Request {
        method,
        target,
        headers,
        body,
    })
}
pub fn validate_target(target: &str) -> Result<()> {
    if !target.starts_with('/')
        || target.starts_with("//")
        || target.contains('#')
        || target.bytes().any(|b| b <= 32 || b >= 127)
    {
        return Err(fail(400, "PATH_DENIED"));
    }
    let p = target.split('?').next().unwrap();
    let b = p.as_bytes();
    let mut decoded = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            if i + 2 >= b.len() {
                return Err(fail(400, "PATH_DENIED"));
            }
            let n = u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap(), 16)
                .map_err(|_| fail(400, "PATH_DENIED"))?;
            if n == b'/' || n == b'\\' || n == b'%' {
                return Err(fail(400, "PATH_DENIED"));
            }
            decoded.push(n);
            i += 3
        } else {
            decoded.push(b[i]);
            i += 1
        }
    }
    let decoded = std::str::from_utf8(&decoded).map_err(|_| fail(400, "PATH_DENIED"))?;
    if decoded.contains('\\')
        || decoded.chars().any(|c| c.is_control())
        || decoded.split('/').any(|s| s == ".." || s == ".")
    {
        return Err(fail(400, "PATH_DENIED"));
    }
    if target.contains('?') {
        return Err(fail(400, "QUERY_NOT_ALLOWED"));
    }
    Ok(())
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Manager,
    Wallpaper,
}
pub(crate) fn random_hex(bytes: usize) -> Result<String> {
    let mut b = vec![0u8; bytes];
    getrandom::fill(&mut b).map_err(|_| fail(500, "ENTROPY_UNAVAILABLE"))?;
    Ok(b.iter().map(|b| format!("{b:02x}")).collect())
}
fn equal_secret(a: &str, b: &str) -> bool {
    bool::from(a.as_bytes().ct_eq(b.as_bytes()))
}
pub struct Security {
    pub origin: String,
    host: String,
    manager: String,
    wallpaper: String,
}
impl Security {
    pub fn new(port: u16) -> Result<Self> {
        if port == 0 {
            return Err(fail(500, "INVALID_LISTENER"));
        }
        let manager = random_hex(32)?;
        let wallpaper = random_hex(32)?;
        if equal_secret(&manager, &wallpaper) {
            return Err(fail(500, "ENTROPY_UNAVAILABLE"));
        }
        Ok(Self {
            origin: format!("http://127.0.0.1:{port}"),
            host: format!("127.0.0.1:{port}"),
            manager,
            wallpaper,
        })
    }
    pub fn common(&self, req: &Request) -> Result<()> {
        validate_target(&req.target)?;
        if req.header("host") != Some(&self.host) {
            return Err(fail(403, "HOST_DENIED"));
        }
        if req.header("origin").is_some_and(|v| v != self.origin) {
            return Err(fail(403, "ORIGIN_DENIED"));
        }
        if req.header("sec-fetch-site") == Some("cross-site")
            && !(req.header("sec-fetch-mode") == Some("navigate")
                && req.header("sec-fetch-dest") == Some("document"))
        {
            return Err(fail(403, "FETCH_METADATA_DENIED"));
        }
        Ok(())
    }
    pub fn authenticate_api(&self, req: &Request) -> Result<Role> {
        self.common(req)?;
        if req.header("sec-fetch-site") != Some("same-origin") {
            return Err(fail(403, "FETCH_METADATA_DENIED"));
        }
        let token = req.header("x-atmos-session").unwrap_or("");
        let role = if equal_secret(token, &self.manager) {
            Role::Manager
        } else if equal_secret(token, &self.wallpaper) {
            Role::Wallpaper
        } else {
            return Err(fail(401, "SESSION_DENIED"));
        };
        Ok(role)
    }
    pub fn api(&self, req: &Request, policy: ApiPolicy) -> Result<Role> {
        let role = self.authenticate_api(req)?;
        if policy.manager_only && role != Role::Manager {
            return Err(fail(403, "ROLE_DENIED"));
        }
        if req.method != policy.method {
            return Err(fail(405, "METHOD_DENIED"));
        }
        if policy.json {
            if req.header("origin") != Some(self.origin.as_str()) {
                return Err(fail(403, "ORIGIN_REQUIRED"));
            }
            if req
                .header("content-type")
                .and_then(|v| v.split(';').next())
                .map(str::trim)
                != Some("application/json")
            {
                return Err(fail(415, "CONTENT_TYPE_DENIED"));
            }
        }
        if req.body.len() > policy.body_limit {
            return Err(fail(413, "BODY_TOO_LARGE"));
        }
        Ok(role)
    }
    pub fn static_response(&self, req: &Request, assets: &Assets) -> Result<Response> {
        self.scoped_static_response(req, assets, &serde_json::Value::Null)
    }
    pub(crate) fn scoped_static_response(
        &self,
        req: &Request,
        assets: &Assets,
        routing: &serde_json::Value,
    ) -> Result<Response> {
        self.common(req)?;
        if req.method != "GET" {
            return Err(fail(405, "METHOD_DENIED"));
        }
        let wallpaper = req.target == "/wallpaper";
        let page = ["/", "/manager", "/wallpaper", "/index.html"].contains(&req.target.as_str());
        if page && req.header("sec-fetch-mode") != Some("navigate") {
            return if req.header("sec-fetch-mode").is_none() {
                Ok(Response::bytes(
                    200,
                    "text/plain; charset=utf-8",
                    b"Atmos runtime ready. Open this address in a browser navigation.".to_vec(),
                ))
            } else {
                Err(fail(403, "NAVIGATION_REQUIRED"))
            };
        }
        let route = if page { "/index.html" } else { &req.target };
        let asset = assets.files.get(route).ok_or(fail(404, "NOT_FOUND"))?;
        let mut out = Response::bytes(200, &asset.mime, asset.bytes.clone());
        if asset.mime.starts_with("text/html") {
            if !page {
                return Err(fail(404, "NOT_FOUND"));
            }
            let nonce = random_hex(18)?;
            let role = if wallpaper { "wallpaper" } else { "manager" };
            let token = if wallpaper {
                &self.wallpaper
            } else {
                &self.manager
            };
            let mut html = String::from_utf8(out.body).map_err(|_| fail(500, "STATIC_INVALID"))?;
            if html.matches("<!--BRIDGE_SESSION-->").count() != 1 {
                return Err(fail(500, "STATIC_INVALID"));
            }
            let routing = routing.to_string().replace('<', "\\u003c");
            let script=format!("<script nonce=\"{nonce}\">window.atmosBridgeSession=\"{token}\";window.atmosBridgeMode=\"{role}\";window.atmosBridgeRouting={routing};</script>");
            html = rewrite_links(&html).replace("<!--BRIDGE_SESSION-->", &script);
            out.body = html.into_bytes();
            let youtube = if wallpaper {
                ""
            } else {
                " https://www.youtube.com/iframe_api https://www.youtube.com/s/player/"
            };
            let frame = if wallpaper {
                "'none'"
            } else {
                "https://www.youtube-nocookie.com/embed/"
            };
            out.headers.insert("Content-Security-Policy".into(),format!("default-src 'self'; script-src 'self' 'nonce-{nonce}'{youtube}; style-src 'self' 'unsafe-inline'; img-src 'self' blob: data:; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-src {frame}; frame-ancestors 'self'"));
        }
        Ok(out)
    }
}
#[derive(Clone, Copy)]
pub struct ApiPolicy {
    pub method: &'static str,
    pub manager_only: bool,
    pub json: bool,
    pub body_limit: usize,
}
impl ApiPolicy {
    pub fn read() -> Self {
        Self {
            method: "GET",
            manager_only: false,
            json: false,
            body_limit: 0,
        }
    }
    pub fn manager_json(limit: usize) -> Self {
        Self {
            method: "POST",
            manager_only: true,
            json: true,
            body_limit: limit,
        }
    }
}
fn rewrite_links(html: &str) -> String {
    let re = regex::Regex::new(r#"(?i)((?:src|href)=["'])([^"']+)(["'])"#).unwrap();
    re.replace_all(html, |c: &regex::Captures<'_>| {
        let value = &c[2];
        let scheme = value
            .split_once(':')
            .is_some_and(|(s, _)| !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphabetic()));
        if value.starts_with(['/', '#']) || scheme {
            c[0].to_string()
        } else {
            format!("{}/{value}{}", &c[1], &c[3])
        }
    })
    .into_owned()
}
struct Asset {
    bytes: Vec<u8>,
    mime: String,
}
pub struct Assets {
    files: BTreeMap<String, Asset>,
}
fn no_links(p: &Path) -> Result<()> {
    for ancestor in p.ancestors() {
        let m = fs::symlink_metadata(ancestor).map_err(|_| fail(500, "STATIC_INVALID"))?;
        if m.file_type().is_symlink() {
            return Err(fail(500, "STATIC_LINK_DENIED"));
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if m.file_attributes() & 0x400 != 0 {
                return Err(fail(500, "STATIC_LINK_DENIED"));
            }
        }
    }
    Ok(())
}
fn safe_read(p: &Path, root: &Path) -> Result<Vec<u8>> {
    no_links(p)?;
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000);
    }
    let mut file = options.open(p).map_err(|_| fail(500, "STATIC_INVALID"))?;
    let meta = file.metadata().map_err(|_| fail(500, "STATIC_INVALID"))?;
    if !meta.is_file() || meta.len() > 16 * 1048576 {
        return Err(fail(500, "STATIC_LIMIT"));
    }
    #[cfg(windows)]
    {
        use std::os::windows::{fs::MetadataExt, io::AsRawHandle};
        use windows_sys::Win32::Storage::FileSystem::GetFinalPathNameByHandleW;
        if meta.file_attributes() & 0x400 != 0 {
            return Err(fail(500, "STATIC_LINK_DENIED"));
        }
        let mut buf = vec![0u16; 32768];
        let n = unsafe {
            GetFinalPathNameByHandleW(
                file.as_raw_handle() as _,
                buf.as_mut_ptr(),
                buf.len() as u32,
                0,
            )
        };
        if n == 0 || n as usize >= buf.len() {
            return Err(fail(500, "STATIC_INVALID"));
        }
        let final_path = PathBuf::from(
            String::from_utf16(&buf[..n as usize]).map_err(|_| fail(500, "STATIC_INVALID"))?,
        );
        if !final_path.starts_with(root) {
            return Err(fail(500, "STATIC_LINK_DENIED"));
        }
    }
    let mut bytes = Vec::new();
    std::io::Read::by_ref(&mut file)
        .take(16 * 1048576 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| fail(500, "STATIC_INVALID"))?;
    if bytes.len() > 16 * 1048576 {
        return Err(fail(500, "STATIC_LIMIT"));
    }
    no_links(p)?;
    Ok(bytes)
}
impl Assets {
    pub(crate) fn bytes(&self, path: &str) -> Option<&[u8]> {
        self.files.get(path).map(|asset| asset.bytes.as_slice())
    }
    pub fn load(root: &Path) -> Result<Self> {
        if !root.is_absolute() {
            return Err(fail(500, "STATIC_INVALID"));
        }
        no_links(root)?;
        let canonical = fs::canonicalize(root).map_err(|_| fail(500, "STATIC_INVALID"))?;
        let mut files = BTreeMap::new();
        let mut total = 0usize;
        let mut entries = 0usize;
        fn visit(
            root: &Path,
            dir: &Path,
            canonical: &Path,
            files: &mut BTreeMap<String, Asset>,
            total: &mut usize,
            entries: &mut usize,
            depth: usize,
        ) -> Result<()> {
            if depth > 32 {
                return Err(fail(500, "STATIC_LIMIT"));
            }
            for entry in fs::read_dir(dir).map_err(|_| fail(500, "STATIC_INVALID"))? {
                let p = entry.map_err(|_| fail(500, "STATIC_INVALID"))?.path();
                *entries += 1;
                if *entries > 4096 {
                    return Err(fail(500, "STATIC_LIMIT"));
                }
                no_links(&p)?;
                if p.is_dir() {
                    visit(root, &p, canonical, files, total, entries, depth + 1)?
                } else {
                    if files.len() >= 4096 {
                        return Err(fail(500, "STATIC_LIMIT"));
                    }
                    let bytes = safe_read(&p, canonical)?;
                    *total += bytes.len();
                    if *total > 256 * 1048576 {
                        return Err(fail(500, "STATIC_LIMIT"));
                    }
                    let relative = p
                        .strip_prefix(root)
                        .map_err(|_| fail(500, "STATIC_INVALID"))?
                        .to_str()
                        .ok_or(fail(500, "STATIC_INVALID"))?
                        .replace('\\', "/");
                    let route = format!("/{relative}");
                    validate_target(&route)?;
                    let mime = match p.extension().and_then(|e| e.to_str()).unwrap_or("") {
                        "html" => "text/html; charset=utf-8",
                        "js" => "text/javascript; charset=utf-8",
                        "css" => "text/css; charset=utf-8",
                        "png" => "image/png",
                        "svg" => "image/svg+xml",
                        "woff" => "font/woff",
                        _ => "application/octet-stream",
                    };
                    files.insert(
                        route,
                        Asset {
                            bytes,
                            mime: mime.into(),
                        },
                    );
                }
            }
            Ok(())
        }
        visit(
            root,
            root,
            &canonical,
            &mut files,
            &mut total,
            &mut entries,
            0,
        )?;
        if !files.contains_key("/index.html") {
            return Err(fail(500, "STATIC_INVALID"));
        }
        Ok(Self { files })
    }
}

pub struct Server {
    listener: TcpListener,
    pub security: Security,
    limits: Limits,
}
impl Server {
    pub fn bind(limits: Limits) -> Result<Self> {
        let listener =
            TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(|_| fail(500, "LISTEN_FAILED"))?;
        listener
            .set_nonblocking(true)
            .map_err(|_| fail(500, "LISTEN_FAILED"))?;
        let security = Security::new(
            listener
                .local_addr()
                .map_err(|_| fail(500, "LISTEN_FAILED"))?
                .port(),
        )?;
        Ok(Self {
            listener,
            security,
            limits,
        })
    }
    pub fn run(
        self,
        stop: Arc<AtomicBool>,
        handler: impl Fn(&Security, Request) -> Response + Send + Sync + 'static,
    ) -> Result<()> {
        let security = Arc::new(self.security);
        let handler = Arc::new(handler);
        let active = Arc::new(AtomicUsize::new(0));
        let mut workers = Vec::new();
        let mut accept_error = None;
        while !stop.load(Ordering::Acquire) {
            match self.listener.accept() {
                Ok((mut stream, peer)) => {
                    if stream.set_nonblocking(false).is_err() {
                        continue;
                    }
                    #[cfg(windows)]
                    {
                        use std::os::windows::io::AsRawSocket;
                        use windows_sys::Win32::Networking::WinSock::{
                            setsockopt, SOL_SOCKET, SO_SNDBUF,
                        };
                        let bytes = 32768i32;
                        if unsafe {
                            setsockopt(
                                stream.as_raw_socket() as _,
                                SOL_SOCKET,
                                SO_SNDBUF,
                                (&bytes as *const i32).cast(),
                                std::mem::size_of::<i32>() as i32,
                            )
                        } != 0
                        {
                            continue;
                        }
                    }
                    if peer.ip() != std::net::IpAddr::V4(Ipv4Addr::LOCALHOST) {
                        continue;
                    }
                    if active.load(Ordering::Acquire) >= MAX_WORKERS {
                        let _ = stream.shutdown(std::net::Shutdown::Both);
                        continue;
                    }
                    active.fetch_add(1, Ordering::AcqRel);
                    let worker_active = active.clone();
                    let handler = handler.clone();
                    let security = security.clone();
                    let limits = self.limits;
                    let worker =
                        thread::Builder::new()
                            .name("atmos-http".into())
                            .spawn(move || {
                                let mut head = false;
                                let response = match read_request(&mut stream, limits) {
                                    Ok(req) => {
                                        head = req.method == "HEAD";
                                        match security.common(&req) {
                                            Ok(()) => std::panic::catch_unwind(
                                                std::panic::AssertUnwindSafe(|| {
                                                    handler(&security, req)
                                                }),
                                            )
                                            .unwrap_or_else(|_| {
                                                Response::error(fail(500, "LOCAL_OPERATION_FAILED"))
                                            }),
                                            Err(e) => Response::error(e),
                                        }
                                    }
                                    Err(e) => Response::error(e),
                                };
                                let _ = write_response(&mut stream, response, limits.write, head);
                                let _ = stream.set_nonblocking(false);
                                let _ = stream.shutdown(std::net::Shutdown::Write);
                                if stream
                                    .set_read_timeout(Some(Duration::from_millis(50)))
                                    .is_ok()
                                {
                                    let mut drain = [0u8; 4096];
                                    let until = Instant::now() + Duration::from_millis(100);
                                    let mut drained = 0;
                                    while Instant::now() < until && drained < 65536 {
                                        match stream.read(&mut drain) {
                                            Ok(0) | Err(_) => break,
                                            Ok(n) => drained += n,
                                        }
                                    }
                                }
                                let _ = stream.shutdown(std::net::Shutdown::Both);
                                worker_active.fetch_sub(1, Ordering::AcqRel);
                            });
                    match worker {
                        Ok(worker) => workers.push(worker),
                        Err(_) => {
                            active.fetch_sub(1, Ordering::AcqRel);
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5))
                }
                Err(_) => {
                    accept_error = Some(fail(500, "LISTEN_FAILED"));
                    break;
                }
            }
            let mut i = 0;
            while i < workers.len() {
                if workers[i].is_finished() {
                    let _ = workers.swap_remove(i).join();
                } else {
                    i += 1
                }
            }
        }
        drop(self.listener);
        for worker in workers {
            let _ = worker.join();
        }
        accept_error.map_or(Ok(()), Err)
    }
}
fn write_response(
    stream: &mut TcpStream,
    mut r: Response,
    timeout: Duration,
    head: bool,
) -> std::io::Result<()> {
    let deadline = Instant::now() + timeout;
    stream.set_nonblocking(true)?;
    r.headers.retain(|k, _| {
        !["content-length", "transfer-encoding", "connection"]
            .iter()
            .any(|name| k.eq_ignore_ascii_case(name))
    });
    let reason = match r.status {
        200 => "OK",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        408 => "Request Timeout",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        417 => "Expectation Failed",
        431 => "Request Header Fields Too Large",
        429 => "Too Many Requests",
        503 => "Service Unavailable",
        _ => "Error",
    };
    let mut headers = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        r.status,
        reason,
        r.body.len()
    );
    for (k, v) in r.headers {
        if !k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || v.bytes().any(|b| b < 32 || b == 127)
        {
            return Err(std::io::Error::other("Invalid response header"));
        }
        headers.push_str(&format!("{k}: {v}\r\n"))
    }
    headers.push_str("\r\n");
    let pieces = if head {
        vec![headers.as_bytes()]
    } else {
        vec![headers.as_bytes(), r.body.as_slice()]
    };
    write_bounded(stream, &pieces, deadline)
}
fn write_bounded(
    writer: &mut impl Write,
    pieces: &[&[u8]],
    deadline: Instant,
) -> std::io::Result<()> {
    for &mut_piece in pieces {
        let mut piece = mut_piece;
        while !piece.is_empty() {
            let _left = deadline
                .checked_duration_since(Instant::now())
                .ok_or(std::io::ErrorKind::TimedOut)?;
            let n = match writer.write(piece) {
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(2));
                    continue;
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            };
            if n == 0 {
                return Err(std::io::ErrorKind::WriteZero.into());
            }
            piece = &piece[n..];
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blocked_or_partial_writes_respect_one_total_deadline() {
        struct Blocked;
        impl Write for Blocked {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::ErrorKind::WouldBlock.into())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let start = Instant::now();
        let err = write_bounded(
            &mut Blocked,
            &[b"headers", b"body"],
            start + Duration::from_millis(10),
        )
        .unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);
        assert!(start.elapsed() < Duration::from_millis(500));
        struct Partial(Vec<u8>);
        impl Write for Partial {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                let n = b.len().min(3);
                self.0.extend_from_slice(&b[..n]);
                Ok(n)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut writer = Partial(vec![]);
        write_bounded(
            &mut writer,
            &[b"headers", b"body"],
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();
        assert_eq!(writer.0, b"headersbody");
    }
    #[test]
    fn paths_reject_ambiguous_and_traversal_inputs() {
        for p in [
            "/../secret",
            "/%2e%2e/secret",
            "/a\\b",
            "//evil/path",
            "http://evil/",
            "/%2fsecret",
            "/%252e%252e/",
            "/%00",
            "/%ff",
            "/%",
            "/a#b",
            "/./x",
        ] {
            assert!(validate_target(p).is_err(), "{p}")
        }
        for p in [
            "/manager",
            "/index.html",
            "/weather-icons/sun.svg",
            "/api/calendar/import",
        ] {
            assert!(validate_target(p).is_ok())
        }
        assert_eq!(
            validate_target("/api/snapshot?x=1").unwrap_err().code,
            "QUERY_NOT_ALLOWED"
        )
    }
    #[test]
    fn independent_sessions_cannot_authorize_each_other() {
        let a = Security::new(1234).unwrap();
        let b = Security::new(1234).unwrap();
        let mut req = Request {
            method: "GET".into(),
            target: "/api/snapshot".into(),
            headers: BTreeMap::from([
                ("host".into(), "127.0.0.1:1234".into()),
                ("sec-fetch-site".into(), "same-origin".into()),
                ("x-atmos-session".into(), a.wallpaper),
            ]),
            body: vec![],
        };
        assert_eq!(
            b.api(&req, ApiPolicy::read()).unwrap_err().code,
            "SESSION_DENIED"
        );
        req.headers
            .insert("x-atmos-session".into(), b.manager.clone());
        assert_eq!(b.api(&req, ApiPolicy::read()).unwrap(), Role::Manager)
    }
}
