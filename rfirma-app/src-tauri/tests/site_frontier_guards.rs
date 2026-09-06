//! Guardas de lo que sale hacia la sede por el canal local (ADR-0011).

use serde_json::Value;

use rfirma_lib::app::codec::V4Codec;
use rfirma_lib::app::errand::{LiveErrand, ProtocolCodec};
use rfirma_lib::app::frontier;
use rfirma_lib::app::site::{attend_launch, Attendance};
use rfirma_lib::channel::Situation as ChannelSituation;
use rfirma_lib::channel::{answer, Answer, ChannelDuty, ChannelError, OpenChannel, Shutdown};
use rfirma_lib::commands::failure::Failure;
use rfirma_lib::commands::{NoCertificateView, NoChannelView, SiteErrandView};
use rfirma_lib::destination::{DestinationError, Situation as DestinationSituation};
use rfirma_lib::ffi::BridgeError;
use rfirma_lib::pkcs11::{Situation as TokenSituation, TokenError};
use rfirma_lib::protocol::{Refusal, SafCode, WireAnswer};
use rfirma_lib::rubric::{RubricError, Situation as RubricSituation};
use rfirma_lib::signing::Refusal as Inadmissible;

/// Enlace del portal que no puede salir.
const A_PORTAL_HANDLE: &str = "/run/user/1000/doc/1e8b83b9/contrato.pdf";

/// Nombre del documento que no puede salir.
const A_DOCUMENT_NAME: &str = "contrato.pdf";

/// Certificado inventado de pruebas.
const A_CERTIFICATE: &str = "CN=CERTIFICADO DE PRUEBAS RFIRMA, SERIALNUMBER=IDCES-00000000T";

/// Los intentos de PIN que quedan, que son de la ventana y de nadie más.
const ATTEMPTS_LEFT: u32 = 2;

/// La credencial de canal de una invocación bien formada.
const CREDENTIAL: &str = "8jAkPZfRw2mQxN4TbYuL";

/// Transporte de prueba que registra el cometido con el que se le llamó.
fn a_transport(
    duties: &std::cell::RefCell<Vec<ChannelDuty>>,
) -> impl Fn(&[u16], ChannelDuty) -> Result<OpenChannel, ChannelError> + '_ {
    move |ports: &[u16], duty: ChannelDuty| {
        duties.borrow_mut().push(duty.clone());
        Ok(OpenChannel::new(ports[0], Shutdown::of(|| {})))
    }
}

/// Todo lo que se queda dentro, ya serializado como cruza a la ventana.
fn what_stays_inside() -> Vec<(&'static str, Value)> {
    let inside_the_detail =
        format!("no se ha podido firmar {A_PORTAL_HANDLE} ({A_DOCUMENT_NAME}) con {A_CERTIFICATE}");

    let mut token: Failure =
        TokenError::new(TokenSituation::IncorrectPin, inside_the_detail.clone()).into();
    token.attempts_left = Some(ATTEMPTS_LEFT);

    vec![
        (
            "TokenError",
            serde_json::to_value(&token).expect("serializa"),
        ),
        (
            "DestinationError",
            serde_json::to_value(Failure::from(DestinationError::new(
                DestinationSituation::FolderMissing,
                inside_the_detail.clone(),
            )))
            .expect("serializa"),
        ),
        (
            "RubricError",
            serde_json::to_value(Failure::from(RubricError::new(
                RubricSituation::DamagedImage,
                inside_the_detail.clone(),
            )))
            .expect("serializa"),
        ),
        (
            "BridgeError",
            serde_json::to_value(Failure::from(BridgeError::Failed(inside_the_detail)))
                .expect("serializa"),
        ),
        (
            "SiteErrandView (sin canal)",
            serde_json::to_value(SiteErrandView::no_channel(NoChannelView::ChannelNotOpened))
                .expect("serializa"),
        ),
        (
            "SiteErrandView (rechazo sin canal)",
            serde_json::to_value(SiteErrandView::refused(&Refusal::about(
                rfirma_lib::protocol::Parameter::Data,
                format!("no se puede leer '{A_PORTAL_HANDLE}' ({A_DOCUMENT_NAME})"),
            )))
            .expect("serializa"),
        ),
        (
            "SiteErrandView (sin certificado)",
            serde_json::to_value(SiteErrandView::without_certificates(
                NoCertificateView::None,
                0,
            ))
            .expect("serializa"),
        ),
        (
            "los valores contaminados de la URL",
            Value::Array(vec![
                Value::String(A_PORTAL_HANDLE.to_owned()),
                Value::String(A_DOCUMENT_NAME.to_owned()),
                Value::String(A_CERTIFICATE.to_owned()),
            ]),
        ),
    ]
}

/// Todo lo que sale hacia la sede, construido desde su caso de uso.
fn everything_that_goes_out_to_the_site() -> Vec<String> {
    let mut lines = Vec::new();

    let duties = std::cell::RefCell::new(Vec::new());
    let transport = a_transport(&duties);
    let url = format!(
        "afirma://websocket?ports=54001&v=4&idsession=no-vale-{A_CERTIFICATE}\
         &dat=file://{A_PORTAL_HANDLE}&fileid={A_DOCUMENT_NAME}"
    );

    let live = LiveErrand::default();
    match attend_launch(&url, &transport, &live) {
        Attendance::RefusingOverTheChannel { answer, .. } => lines.push(answer.on_the_wire()),
        other => panic!("con puertos el rechazo sale por el socket: {other:?}"),
    }

    let good = format!(
        "afirma://websocket?ports=54002&v=4&idsession={CREDENTIAL}\
         &dat={A_PORTAL_HANDLE}&fileid={A_DOCUMENT_NAME}"
    );
    assert!(
        matches!(
            attend_launch(&good, &transport, &live),
            Attendance::Serving { .. }
        ),
        "la primera invocacion abre el canal"
    );
    match attend_launch(&good, &transport, &live) {
        Attendance::RefusingOverTheChannel { answer, .. } => lines.push(answer.on_the_wire()),
        other => panic!("con un tramite vivo la segunda se rechaza: {other:?}"),
    }

    for duty in duties.borrow().iter() {
        let messages = [
            format!("echo=-idsession={CREDENTIAL}@EOF"),
            format!("afirma://sign?op=sign&dat={A_PORTAL_HANDLE}&idsession={CREDENTIAL}"),
            A_PORTAL_HANDLE.to_owned(),
        ];
        for message in messages {
            for from_loopback in [true, false] {
                match answer(duty, from_loopback, &message) {
                    Answer::Reply(text) | Answer::ReplyAndClose(text) => lines.push(text),
                    Answer::Pending(_) => {}
                }
            }
        }
    }

    for code in [
        frontier::code_of_token(TokenSituation::IncorrectPin),
        frontier::code_of_destination(DestinationSituation::FolderMissing),
        frontier::code_of_rubric(RubricSituation::DamagedImage),
        frontier::code_of_channel(ChannelSituation::NoDrawnPortIsFree),
        frontier::code_of_inadmissible(Inadmissible::NotAPdf),
        frontier::code_of_bridge(&BridgeError::Failed("lo que dijera Java".to_owned())),
        frontier::code_of_broken_seal(),
    ] {
        lines.push(WireAnswer::refused(code).on_the_wire());
    }
    lines.push(frontier::cancelled().on_the_wire());
    let codec = V4Codec;
    lines.push(codec.encode(&rfirma_lib::app::errand::declined(&LiveErrand::default())));
    lines.push(
        codec.encode(&rfirma_lib::app::errand::the_signature_did_not_come_out(
            &LiveErrand::default(),
            rfirma_lib::app::signing::SiteRefusal::new(
                frontier::code_of_bridge(&BridgeError::Failed(String::new())),
                Failure::from(BridgeError::Failed(format!(
                    "no se ha podido firmar {A_PORTAL_HANDLE} ({A_DOCUMENT_NAME}) con \
                     {A_CERTIFICATE}"
                ))),
            ),
        )),
    );

    lines
}

#[test]
fn the_dead_ends_write_nothing_on_the_wire() {
    let duties = std::cell::RefCell::new(Vec::new());
    let live = LiveErrand::default();

    let without_ports = format!("afirma://websocket?v=4&idsession={CREDENTIAL}");
    match attend_launch(&without_ports, &a_transport(&duties), &live) {
        Attendance::RefusingInTheWindow(_) => {}
        other => panic!("sin puertos el rechazo es de la ventana: {other:?}"),
    }
    assert!(
        duties.borrow().is_empty(),
        "sin puertos no se le pide nada al transporte, ni para contestar"
    );

    let with_every_port_taken =
        format!("afirma://websocket?ports=54001&v=4&idsession={CREDENTIAL}");
    let refuses_everything = |_: &[u16], _: ChannelDuty| {
        Err(ChannelError::new(
            ChannelSituation::NoDrawnPortIsFree,
            "el puerto sorteado esta ocupado",
        ))
    };
    match attend_launch(&with_every_port_taken, &refuses_everything, &live) {
        Attendance::ChannelNotOpened(_) => {}
        other => panic!("con el puerto ocupado no hay canal: {other:?}"),
    }
}

/// Hojas de un valor ya serializado.
fn leaves(value: &Value, into: &mut Vec<String>) {
    match value {
        Value::String(text) => into.push(text.clone()),
        Value::Number(number) => into.push(number.to_string()),
        Value::Array(items) => items.iter().for_each(|item| leaves(item, into)),
        Value::Object(fields) => fields.values().for_each(|field| leaves(field, into)),
        _ => {}
    }
}

/// Comprueba si la línea menciona la hoja.
fn mentions(line: &str, leaf: &str) -> bool {
    if leaf.chars().all(|letter| letter.is_ascii_digit()) {
        return line
            .split(|letter: char| !letter.is_ascii_alphanumeric())
            .any(|word| word == leaf);
    }
    line.contains(leaf)
}

#[test]
fn nothing_of_ours_crosses_the_socket() {
    let outgoing = everything_that_goes_out_to_the_site();
    assert!(!outgoing.is_empty(), "no se ha construido nada que salga");

    for (name, inside) in what_stays_inside() {
        let mut hojas = Vec::new();
        leaves(&inside, &mut hojas);
        assert!(!hojas.is_empty(), "{name} no tiene ningun campo que mirar");

        for leaf in hojas {
            for line in &outgoing {
                assert!(
                    !mentions(line, &leaf),
                    "«{line}» lleva a la sede el campo «{leaf}» de {name}"
                );
            }
        }
    }
}

#[test]
fn every_line_that_goes_out_is_one_the_closed_catalogue_can_produce() {
    let mut possible: Vec<String> = Vec::new();
    for code in SafCode::ALL {
        possible.push(WireAnswer::refused(code).on_the_wire());
        for parameter in rfirma_lib::protocol::Parameter::ALL {
            possible.push(WireAnswer::refused_because_of(code, parameter).on_the_wire());
        }
    }
    possible.push(WireAnswer::Cancelled.on_the_wire());
    possible.push(WireAnswer::OutOfMemory.on_the_wire());
    possible.push(WireAnswer::Nothing.on_the_wire());
    possible.push(rfirma_lib::channel::ECHO_OK.to_owned());

    for line in everything_that_goes_out_to_the_site() {
        assert!(
            possible.contains(&line),
            "«{line}» no la produce el catalogo cerrado: es un codigo acunado"
        );
    }
}

#[test]
fn the_shadow_attack_code_is_named_only_where_the_catalogue_is_declared() {
    use std::fs;
    use std::path::{Path, PathBuf};

    fn rust_files_under(directory: &Path, into: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(directory).expect("el directorio de fuentes deberia leerse") {
            let path = entry.expect("cada entrada deberia leerse").path();
            if path.is_dir() {
                rust_files_under(&path, into);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                into.push(path);
            }
        }
    }

    let sources = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files_under(&sources, &mut files);

    let allowed = ["codes.rs"];
    for file in files {
        let name = file
            .file_name()
            .and_then(|name| name.to_str())
            .expect("todo fichero tiene nombre");
        if allowed.contains(&name) {
            continue;
        }
        let source = fs::read_to_string(&file).expect("la fuente deberia leerse");
        let production = source
            .split_once("\nmod tests {")
            .map_or(source.as_str(), |(before, _)| before);
        assert!(
            !production.contains("PdfShadowAttack"),
            "{} nombra el codigo del shadow attack, y ese no se emite nunca",
            file.display()
        );
    }
}
