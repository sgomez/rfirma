//! Framing artesanal del transporte `service`: lector y escritor puros, sin socket
//! (`CommandProcessorThread.java`, ADR-0017).

use base64::engine::general_purpose::URL_SAFE;
use base64::Engine as _;

use super::launch::NegotiatedCredential;

const CMD: &str = "cmd=";
const ECHO: &str = "echo=";
const FRAGMENT: &str = "fragment=";
const FIRM: &str = "firm=";
const SEND: &str = "send=";
const EOF: &str = "@EOF";
const IDSESSION: &str = "idsession";
const RESET: &str = "-";

/// Tamaño máximo de cada parte de una respuesta fragmentada (`RESPONSE_MAX_SIZE`, línea 57).
pub const RESPONSE_MAX_SIZE: usize = 1_000_000;

/// Una petición del framing `service`, ya leída y lista para el trámite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FramedRequest {
    /// `echo=`: mensaje ya reconstruido para `conversation::answer`, y si pedía reiniciar el
    /// reensamblado en curso (`doEchoPetition`, línea 254).
    Echo { message: String, resets: bool },
    /// `cmd=`: una operación sin fragmentar, ya reconstruida para `conversation::answer`
    /// (`doCmdPetition`, línea 330).
    Command { message: String },
    /// `fragment=`: un trozo de una operación repartida en varias peticiones, con la
    /// credencial de esta petición (`doFragmentPetition`, línea 367).
    Fragment {
        part: usize,
        total: usize,
        chunk: String,
        credential: Option<String>,
    },
    /// `firm=`: combina los fragmentos ya recibidos y lanza la operación (`doFragmentedProcess`,
    /// línea 264), con la credencial de esta petición.
    Firm { credential: Option<String> },
    /// `send=`: pide una parte ya calculada de una respuesta fragmentada (`doSendPetition`,
    /// línea 397), con la credencial de esta petición.
    Send {
        part: usize,
        total: usize,
        credential: Option<String>,
    },
}

/// La petición cruda no trae ninguno de los cinco comandos del framing
/// (`getUriTypeFromRequest`, línea 229).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotOfTheFraming;

/// Lee la petición cruda tal como llega del socket: separa la credencial de canal de la cola
/// (`idsession=…@EOF`) y reconoce el comando, en el mismo orden que `getUriTypeFromRequest`
/// (línea 229): `cmd=`, `echo=`, `fragment=`, `firm=`, `send=`.
pub fn read_request(raw: &str) -> Result<FramedRequest, NotOfTheFraming> {
    let (data, credential) = split_credential(raw);

    if let Some(value) = after(data, CMD) {
        let decoded = decode_base64(value).ok_or(NotOfTheFraming)?;
        return Ok(FramedRequest::Command {
            message: with_credential(&decoded, credential.as_deref()),
        });
    }
    if let Some(value) = after(data, ECHO) {
        return Ok(FramedRequest::Echo {
            message: echo_message(credential.as_deref()),
            resets: value.contains(RESET),
        });
    }
    if let Some(value) = after(data, FRAGMENT) {
        let (part, total, chunk) = parse_fragment(value).ok_or(NotOfTheFraming)?;
        return Ok(FramedRequest::Fragment {
            part,
            total,
            chunk,
            credential,
        });
    }
    if after(data, FIRM).is_some() {
        return Ok(FramedRequest::Firm { credential });
    }
    if let Some(value) = after(data, SEND) {
        let (part, total) = parse_send(value).ok_or(NotOfTheFraming)?;
        return Ok(FramedRequest::Send {
            part,
            total,
            credential,
        });
    }

    Err(NotOfTheFraming)
}

/// Si la credencial negociada exige una y la petición trae otra, o no trae ninguna
/// (`checkIdSession`, línea 618).
pub fn credential_matches(negotiated: &NegotiatedCredential, candidate: Option<&str>) -> bool {
    match negotiated {
        NegotiatedCredential::Absent => true,
        NegotiatedCredential::Required(expected) => candidate == Some(expected.as_str()),
    }
}

/// Reensamblado en orden de una operación repartida en varias peticiones `fragment=`
/// (los campos estáticos `request`, `toSend` y `parts`, líneas 65-67). Sin socket.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FragmentBuffer {
    parts: Vec<String>,
}

impl FragmentBuffer {
    /// Un reensamblado vacío.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserta o sustituye la parte recibida, por su posición de uno (`doFragmentPetition`,
    /// líneas 385-391).
    pub fn insert(&mut self, part: usize, chunk: String) {
        let index = part.saturating_sub(1);
        if index < self.parts.len() {
            self.parts[index] = chunk;
        } else {
            self.parts.push(chunk);
        }
    }

    /// La operación entera, con las partes unidas en orden, cuando ya están todas
    /// (`doFragmentedProcess`, líneas 271-274).
    pub fn combined(&self) -> Option<String> {
        if self.parts.is_empty() {
            return None;
        }
        Some(self.parts.concat())
    }

    /// Descarta lo reensamblado hasta ahora (`reset()`, línea 431).
    pub fn reset(&mut self) {
        self.parts.clear();
    }
}

/// Reparte una respuesta larga en partes de como mucho `RESPONSE_MAX_SIZE` caracteres,
/// en el mismo orden en que se piden luego con `send=` (`calculateNumberPartsResponse`,
/// línea 419).
pub fn split_response(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }

    text.as_bytes()
        .chunks(RESPONSE_MAX_SIZE)
        .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
        .collect()
}

/// La respuesta HTTP artesanal que espera el cliente publicado: cinco cabeceras terminadas en
/// `\n`, una línea en blanco, y el cuerpo en Base64 URL-safe (`createHttpResponse`, línea 493).
pub fn http_response(body: &str) -> Vec<u8> {
    let mut response = String::from("HTTP/1.1 200 OK\n");
    response.push_str("Connection: close\n");
    response.push_str("Pragma: no-cache\n");
    response.push_str("Server: Cliente @firma\n");
    response.push_str("Content-Type: text/html; charset=utf-8\n");
    response.push_str("Access-Control-Allow-Origin: *\n");
    response.push('\n');
    response.push_str(&URL_SAFE.encode(body.as_bytes()));

    response.into_bytes()
}

/// Separa la credencial de canal (`idsession=…`) y la cola `@EOF` del resto de la petición,
/// como hace `read()` con el buffer completo ya reensamblado (líneas 552-596).
fn split_credential(raw: &str) -> (&str, Option<String>) {
    let Some(eof) = raw.find(EOF) else {
        return (raw, None);
    };

    match raw[..eof].find(IDSESSION) {
        Some(id_position) => {
            let value_start = id_position + IDSESSION.len() + 1;
            let credential = raw.get(value_start..eof).unwrap_or_default().to_owned();
            (&raw[..id_position], Some(credential))
        }
        None => (&raw[..eof], None),
    }
}

fn after<'a>(data: &'a str, marker: &str) -> Option<&'a str> {
    data.find(marker)
        .map(|position| &data[position + marker.len()..])
}

fn decode_base64(value: &str) -> Option<String> {
    let bytes = URL_SAFE.decode(value.trim()).ok()?;
    String::from_utf8(bytes).ok()
}

fn echo_message(credential: Option<&str>) -> String {
    match credential {
        Some(value) => format!("{ECHO}{RESET}{IDSESSION}={value}{EOF}"),
        None => format!("{ECHO}{EOF}"),
    }
}

fn with_credential(message: &str, credential: Option<&str>) -> String {
    match credential {
        Some(value) => {
            let separator = if message.contains('?') { '&' } else { '?' };
            format!("{message}{separator}{IDSESSION}={value}{EOF}")
        }
        None => message.to_owned(),
    }
}

fn parse_fragment(value: &str) -> Option<(usize, usize, String)> {
    let mut fields = value.split('@');
    fields.next()?;
    let part = fields.next()?.parse().ok()?;
    let total = fields.next()?.parse().ok()?;
    let chunk = decode_base64(fields.next()?)?;
    Some((part, total, chunk))
}

fn parse_send(value: &str) -> Option<(usize, usize)> {
    let mut fields = value.split('@');
    fields.next()?;
    let part = fields.next()?.parse().ok()?;
    let total = fields.next()?.parse().ok()?;
    Some((part, total))
}

#[cfg(test)]
mod tests;
