//! El servidor local de la consola web: escucha en `127.0.0.1`, exige el token de la URL y un
//! `Host`/`Origin` propios, y traduce cada ruta a la sesión; no decide nada.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::RecvTimeoutError;
use std::thread::spawn;
use std::time::Duration;

use serde::Deserialize;
use serde_json::json;
use ts_rs::TS;

use crate::client::ClientKind;
use crate::console::{Console, NewReport, Request};

/// Los puertos en que la consola intenta escuchar: por debajo de 49152, donde el cliente publicado
/// sortea los del canal, y lejos del 63117 del protocolo de la versión 3.
pub(crate) const THE_CONSOLE_PORTS: std::ops::RangeInclusive<u16> = 47_117..=47_126;

const THE_CONSOLE_BUILD: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/console/dist");
const THE_PAGE_POLICY: &str = "default-src 'self'; img-src 'self' data:; base-uri 'none'; \
                               form-action 'none'; frame-ancestors 'none'";
const THE_LARGEST_BODY: usize = 64 * 1024;
const THE_HEARTBEAT: Duration = Duration::from_secs(15);

/// Lo que una petición tiene que traer para que se la atienda.
pub(crate) struct Gate {
    token: String,
    port: u16,
}

/// La consola compilada por `just conformance-console`, que se lee del disco en cada petición.
pub(crate) struct Build {
    dir: PathBuf,
}

#[derive(Debug, Default)]
pub(crate) struct HttpRequest {
    method: String,
    path: String,
    query: Vec<(String, String)>,
    host: Option<String>,
    origin: Option<String>,
    body: Vec<u8>,
}

pub struct Server {
    listener: TcpListener,
    gate: std::sync::Arc<Gate>,
    build: std::sync::Arc<Build>,
}

impl Build {
    /// La consola compilada en `dir`, o qué receta la compila si no está.
    pub(crate) fn at(dir: &Path) -> Result<Self, String> {
        if dir.join("index.html").is_file() {
            Ok(Self {
                dir: dir.to_owned(),
            })
        } else {
            Err(format!(
                "falta la consola compilada en {}: ejecuta `just conformance-console`",
                dir.display()
            ))
        }
    }

    fn page(&self, token: &str) -> Result<String, String> {
        std::fs::read_to_string(self.dir.join("index.html"))
            .map(|page| the_page_with_its_token(&page, token))
            .map_err(|error| format!("la página de la consola no se pudo leer: {error}"))
    }

    fn asset(&self, name: &str) -> Option<Vec<u8>> {
        is_an_asset_name(name)
            .then(|| std::fs::read(self.dir.join("assets").join(name)).ok())
            .flatten()
    }
}

/// La página con el token en cada recurso suyo, que sin él la guarda no serviría.
fn the_page_with_its_token(page: &str, token: &str) -> String {
    let mut marked = String::with_capacity(page.len());
    let mut rest = page;
    while let Some(at) = rest.find("\"/assets/") {
        let (before, asset) = rest.split_at(at + 1);
        let end = asset.find('"').unwrap_or(asset.len());
        marked.push_str(before);
        marked.push_str(&asset[..end]);
        marked.push_str("?token=");
        marked.push_str(token);
        rest = &asset[end..];
    }
    marked.push_str(rest);
    marked
}

/// Un fichero de `assets/`, y nunca una ruta que salga de él.
fn is_an_asset_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

fn content_type_of(name: &str) -> &'static str {
    match name.rsplit('.').next() {
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

impl Server {
    /// Escucha en el primer puerto libre, con un token nuevo, si la consola está compilada.
    pub fn bind() -> Result<Self, String> {
        let build = Build::at(Path::new(THE_CONSOLE_BUILD))?;
        let listener = THE_CONSOLE_PORTS
            .clone()
            .find_map(|port| TcpListener::bind(("127.0.0.1", port)).ok())
            .ok_or_else(|| {
                format!(
                    "no queda libre ningún puerto de la consola ({}–{})",
                    THE_CONSOLE_PORTS.start(),
                    THE_CONSOLE_PORTS.end()
                )
            })?;
        let port = listener
            .local_addr()
            .map_err(|error| format!("el puerto de la consola no se pudo leer: {error}"))?
            .port();
        Ok(Self {
            listener,
            gate: std::sync::Arc::new(Gate {
                token: a_fresh_token()?,
                port,
            }),
            build: std::sync::Arc::new(build),
        })
    }

    pub fn url(&self) -> String {
        format!(
            "http://127.0.0.1:{}/?token={}",
            self.gate.port, self.gate.token
        )
    }

    pub fn serve(self, console: &Console) {
        for stream in self.listener.incoming().filter_map(Result::ok) {
            let console = console.clone();
            let gate = std::sync::Arc::clone(&self.gate);
            let build = std::sync::Arc::clone(&self.build);
            spawn(move || attend(stream, &gate, &build, &console));
        }
    }
}

fn a_fresh_token() -> Result<String, String> {
    let mut bytes = [0u8; 24];
    getrandom::fill(&mut bytes).map_err(|error| format!("no hay azar para el token: {error}"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

impl Gate {
    /// Si la petición trae el token y viene de la propia página; si no, el motivo.
    fn admits(&self, request: &HttpRequest) -> Result<(), &'static str> {
        let ours = [
            format!("127.0.0.1:{}", self.port),
            format!("localhost:{}", self.port),
        ];
        if !request
            .host
            .as_ref()
            .is_some_and(|host| ours.contains(host))
        {
            return Err("el Host no es el de la consola");
        }
        if let Some(origin) = &request.origin {
            let from_ours = ours.iter().any(|host| origin == &format!("http://{host}"));
            if !from_ours {
                return Err("el Origin no es el de la consola");
            }
        }
        if request.parameter("token") != Some(self.token.as_str()) {
            return Err("falta el token de la consola, o no es el suyo");
        }
        Ok(())
    }
}

impl HttpRequest {
    fn parameter(&self, name: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    fn read(stream: &mut impl Read) -> Result<Self, String> {
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|error| format!("la petición no se pudo leer: {error}"))?;
        let mut parts = line.split_whitespace();
        let method = parts.next().unwrap_or_default().to_owned();
        let target = parts.next().unwrap_or_default();
        let (path, query) = target.split_once('?').unwrap_or((target, ""));
        let mut request = Self {
            method,
            path: path.to_owned(),
            query: query
                .split('&')
                .filter(|pair| !pair.is_empty())
                .map(|pair| {
                    let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
                    (percent_decoded(key), percent_decoded(value))
                })
                .collect(),
            ..Self::default()
        };
        let mut length = 0;
        loop {
            line.clear();
            reader
                .read_line(&mut line)
                .map_err(|error| format!("la cabecera no se pudo leer: {error}"))?;
            let header = line.trim_end();
            if header.is_empty() {
                break;
            }
            let Some((name, value)) = header.split_once(':') else {
                continue;
            };
            let value = value.trim().to_owned();
            match name.to_ascii_lowercase().as_str() {
                "host" => request.host = Some(value),
                "origin" => request.origin = Some(value),
                "content-length" => length = value.parse().unwrap_or(0),
                _ => {}
            }
        }
        if length > THE_LARGEST_BODY {
            return Err("el cuerpo de la petición es demasiado grande".to_owned());
        }
        request.body = vec![0; length];
        reader
            .read_exact(&mut request.body)
            .map_err(|error| format!("el cuerpo no se pudo leer: {error}"))?;
        Ok(request)
    }
}

fn percent_decoded(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let escaped = bytes
            .get(i + 1..i + 3)
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match (bytes[i], escaped) {
            (b'%', Some(byte)) => {
                decoded.push(byte);
                i += 3;
            }
            (b'+', _) => {
                decoded.push(b' ');
                i += 1;
            }
            (byte, _) => {
                decoded.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn attend(mut stream: TcpStream, gate: &Gate, build: &Build, console: &Console) {
    let request = match HttpRequest::read(&mut stream) {
        Ok(request) => request,
        Err(complaint) => return respond(&mut stream, 400, &json!({ "error": complaint })),
    };
    if let Err(complaint) = gate.admits(&request) {
        return respond(&mut stream, 403, &json!({ "error": complaint }));
    }
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", path) if is_a_page(path) => match build.page(&gate.token) {
            Ok(page) => respond_with_page(&mut stream, &page),
            Err(complaint) => respond(&mut stream, 500, &json!({ "error": complaint })),
        },
        ("GET", path) if path.starts_with("/assets/") => {
            let name = &path["/assets/".len()..];
            match build.asset(name) {
                Some(bytes) => respond_with_bytes(&mut stream, 200, content_type_of(name), &bytes),
                None => respond(&mut stream, 404, &json!({ "error": "no hay tal recurso" })),
            }
        }
        ("GET", "/api/events") => stream_events(stream, console),
        ("GET", "/api/defaults") => answer(&mut stream, console.the_deduced_coordinates()),
        ("GET", "/api/log") => answer_text(
            &mut stream,
            console.log_of(
                request.parameter("report"),
                request.parameter("check").unwrap_or_default(),
            ),
        ),
        ("GET", "/api/transcript") => answer_text(
            &mut stream,
            console.transcript_of(
                request.parameter("report"),
                request.parameter("check").unwrap_or_default(),
            ),
        ),
        ("GET", "/api/report-view") => answer(
            &mut stream,
            console.report_view(request.parameter("report").unwrap_or_default()),
        ),
        ("GET", "/api/references") => answer(&mut stream, console.references()),
        ("GET", "/api/compare") => answer(
            &mut stream,
            console.compare(
                request.parameter("a").unwrap_or_default(),
                request.parameter("b").unwrap_or_default(),
            ),
        ),
        ("GET", "/api/validate") => answer(
            &mut stream,
            console.validate(
                request.parameter("report").unwrap_or_default(),
                request.parameter("reference").unwrap_or_default(),
            ),
        ),
        ("POST", "/api/client") => answer(
            &mut stream,
            body_of::<ClientChoice>(&request).and_then(|choice| {
                let kind = ClientKind::named(&choice.kind)
                    .ok_or_else(|| format!("no conozco el cliente «{}»", choice.kind))?;
                console.choose_client(
                    kind,
                    choice
                        .binary
                        .filter(|path| !path.is_empty())
                        .map(PathBuf::from),
                    choice
                        .trust_root
                        .filter(|path| !path.is_empty())
                        .map(PathBuf::from),
                )
            }),
        ),
        ("POST", "/api/reports") => answer(
            &mut stream,
            body_of::<NewReport>(&request).and_then(|new| console.create_report(new)),
        ),
        ("POST", "/api/report") => answer(
            &mut stream,
            body_of::<ReportChoice>(&request).and_then(|choice| console.open_report(choice.name)),
        ),
        ("POST", "/api/run") => answer(
            &mut stream,
            body_of::<Request>(&request).and_then(|wanted| console.enqueue(wanted)),
        ),
        ("POST", "/api/stop") => {
            console.stop();
            answer(&mut stream, Ok::<(), String>(()));
        }
        ("POST", "/api/skip") => {
            console.skip();
            answer(&mut stream, Ok::<(), String>(()));
        }
        ("POST", "/api/answer") => answer(
            &mut stream,
            body_of::<Answer>(&request).and_then(|given| console.answer(given.answer)),
        ),
        _ => respond(&mut stream, 404, &json!({ "error": "no hay tal ruta" })),
    }
}

/// Las direcciones que sirven la página: la sesión activa, un informe en solo lectura y la comparación.
fn is_a_page(path: &str) -> bool {
    matches!(path, "/" | "/comparar")
        || path
            .strip_prefix("/informe/")
            .is_some_and(|name| !name.is_empty() && !name.contains('/'))
}

#[derive(Deserialize, TS)]
#[ts(export)]
struct ClientChoice {
    kind: String,
    binary: Option<String>,
    trust_root: Option<String>,
}

#[derive(Deserialize, TS)]
#[ts(export)]
struct ReportChoice {
    name: String,
}

#[derive(Deserialize, TS)]
#[ts(export)]
struct Answer {
    answer: Option<String>,
}

fn body_of<'a, T: Deserialize<'a>>(request: &'a HttpRequest) -> Result<T, String> {
    serde_json::from_slice(&request.body)
        .map_err(|error| format!("la petición no trae lo que se esperaba: {error}"))
}

fn answer<T: serde::Serialize>(stream: &mut TcpStream, result: Result<T, String>) {
    match result {
        Ok(value) => respond(stream, 200, &json!({ "ok": value })),
        Err(complaint) => respond(stream, 409, &json!({ "error": complaint })),
    }
}

fn answer_text(stream: &mut TcpStream, result: Result<String, String>) {
    match result {
        Ok(text) => respond_with(stream, 200, "text/plain; charset=utf-8", &text),
        Err(complaint) => respond(stream, 409, &json!({ "error": complaint })),
    }
}

fn respond(stream: &mut TcpStream, status: u16, body: &serde_json::Value) {
    respond_with(stream, status, "application/json", &body.to_string());
}

fn respond_with(stream: &mut TcpStream, status: u16, content_type: &str, body: &str) {
    respond_with_bytes(stream, status, content_type, body.as_bytes());
}

fn respond_with_page(stream: &mut TcpStream, page: &str) {
    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\
         Content-Security-Policy: {THE_PAGE_POLICY}\r\nX-Content-Type-Options: nosniff\r\n\
         Referrer-Policy: no-referrer\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{page}",
        page.len()
    );
}

fn respond_with_bytes(stream: &mut TcpStream, status: u16, content_type: &str, body: &[u8]) {
    let head = format!(
        "HTTP/1.1 {status} {}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\
         X-Content-Type-Options: nosniff\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        reason_of(status),
        body.len()
    );
    let _ = stream
        .write_all(head.as_bytes())
        .and_then(|()| stream.write_all(body));
}

fn reason_of(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Conflict",
    }
}

fn stream_events(mut stream: TcpStream, console: &Console) {
    let events = console.subscribe();
    if write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\n\
         Connection: keep-alive\r\n\r\n"
    )
    .is_err()
    {
        return;
    }
    loop {
        let frame = match events.recv_timeout(THE_HEARTBEAT) {
            Ok(frame) => frame,
            Err(RecvTimeoutError::Timeout) => ": latido\n\n".to_owned(),
            Err(RecvTimeoutError::Disconnected) => return,
        };
        if stream
            .write_all(frame.as_bytes())
            .and_then(|()| stream.flush())
            .is_err()
        {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_gate() -> Gate {
        Gate {
            token: "secreto".to_owned(),
            port: 47_117,
        }
    }

    fn a_request(host: Option<&str>, origin: Option<&str>, token: Option<&str>) -> HttpRequest {
        HttpRequest {
            method: "POST".to_owned(),
            path: "/api/run".to_owned(),
            query: token
                .map(|token| vec![("token".to_owned(), token.to_owned())])
                .unwrap_or_default(),
            host: host.map(str::to_owned),
            origin: origin.map(str::to_owned),
            body: Vec::new(),
        }
    }

    #[test]
    fn admits_the_page_with_its_token_host_and_origin() {
        let request = a_request(
            Some("127.0.0.1:47117"),
            Some("http://127.0.0.1:47117"),
            Some("secreto"),
        );

        assert_eq!(a_gate().admits(&request), Ok(()));
    }

    #[test]
    fn admits_a_request_without_origin_like_the_first_page_load() {
        let request = a_request(Some("localhost:47117"), None, Some("secreto"));

        assert_eq!(a_gate().admits(&request), Ok(()));
    }

    #[test]
    fn rejects_a_request_without_the_token_or_with_another() {
        let without = a_request(Some("127.0.0.1:47117"), None, None);
        let another = a_request(Some("127.0.0.1:47117"), None, Some("otro"));

        assert!(a_gate().admits(&without).is_err());
        assert!(a_gate().admits(&another).is_err());
    }

    #[test]
    fn rejects_a_foreign_host_even_with_the_token() {
        let request = a_request(Some("evil.example:47117"), None, Some("secreto"));

        assert_eq!(
            a_gate().admits(&request),
            Err("el Host no es el de la consola")
        );
    }

    #[test]
    fn rejects_a_foreign_origin_even_with_the_token() {
        let request = a_request(
            Some("127.0.0.1:47117"),
            Some("https://evil.example"),
            Some("secreto"),
        );

        assert_eq!(
            a_gate().admits(&request),
            Err("el Origin no es el de la consola")
        );
    }

    #[test]
    fn the_console_ports_stay_clear_of_the_protocol() {
        for port in THE_CONSOLE_PORTS {
            assert!(
                port < 49_152,
                "{port} cae donde el cliente publicado sortea"
            );
            assert_ne!(port, 63_117);
        }
    }

    #[test]
    fn reads_the_method_the_path_the_query_and_the_body() {
        let raw = b"POST /api/run?token=a%20b HTTP/1.1\r\nHost: 127.0.0.1:47117\r\n\
                    Origin: http://127.0.0.1:47117\r\nContent-Length: 9\r\n\r\n\"pending\"";

        let request = HttpRequest::read(&mut &raw[..]).unwrap();

        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/api/run");
        assert_eq!(request.parameter("token"), Some("a b"));
        assert_eq!(request.host.as_deref(), Some("127.0.0.1:47117"));
        assert_eq!(request.origin.as_deref(), Some("http://127.0.0.1:47117"));
        assert_eq!(request.body, b"\"pending\"");
    }

    #[test]
    fn the_session_any_report_and_the_comparison_serve_the_page() {
        for path in [
            "/",
            "/informe/autofirma-1.9.2",
            "/informe/rfirma",
            "/comparar",
        ] {
            assert!(is_a_page(path), "{path}");
        }
    }

    #[test]
    fn a_report_page_needs_one_name_and_nothing_more() {
        for path in ["/informe/", "/informe/a/b", "/informe", "/api/report-view"] {
            assert!(!is_a_page(path), "{path}");
        }
    }

    #[test]
    fn the_page_hands_its_token_to_each_of_its_assets() {
        let page = r#"<script type="module" src="/assets/index-a1.js"></script><link href="/assets/index-b2.css">"#;

        assert_eq!(
            the_page_with_its_token(page, "t0k"),
            r#"<script type="module" src="/assets/index-a1.js?token=t0k"></script><link href="/assets/index-b2.css?token=t0k">"#
        );
    }

    #[test]
    fn an_asset_is_one_file_of_the_assets_dir_and_never_a_way_out() {
        assert!(is_an_asset_name("index-a1.js"));
        for name in ["", "../index.html", ".hidden", "a/b.js", "%2e%2e"] {
            assert!(!is_an_asset_name(name), "{name}");
        }
    }

    #[test]
    fn a_console_that_was_never_built_names_the_recipe_that_builds_it() {
        let empty = tempfile::tempdir().unwrap();

        let complaint = Build::at(empty.path()).err().unwrap();

        assert!(
            complaint.contains("just conformance-console"),
            "{complaint}"
        );
    }

    #[test]
    fn percent_decoding_leaves_a_stray_percent_alone() {
        assert_eq!(percent_decoded("%C3%B1u+%zz%"), "ñu %zz%");
    }

    #[test]
    fn a_fresh_token_is_long_and_never_the_same() {
        let first = a_fresh_token().unwrap();

        assert_eq!(first.len(), 48);
        assert_ne!(first, a_fresh_token().unwrap());
    }
}
