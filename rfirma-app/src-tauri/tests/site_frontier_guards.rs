//! **Las guardas de lo que sale hacia la sede**, hermanas de las de
//! [`rfirma_lib::commands::guards`] (ID-291, TD-58).
//!
//! La de allí comprueba que la ruta del portal no cruza a la ventana; ésta
//! comprueba que **nada nuestro cruza el socket**: ni ruta, ni nombre de
//! documento, ni certificado, ni intentos de PIN restantes. El método es el
//! mismo, y por eso son hermanas: no se inventa ningún valor a mano, se
//! construye lo que sale **desde su caso de uso** con un enlace del portal
//! dentro, y se recorre campo a campo lo que se queda para comprobar que no
//! aparece en lo que sale.
//!
//! **Grada A**: no toca token, ni librería nativa, ni red.
//!
//! Vive en `tests/` y no dentro de `app/` a propósito: para mirar a la vez lo
//! que se queda (los tipos de [`rfirma_lib::commands`]) y lo que sale (los de
//! [`rfirma_lib::protocol`]) hay que nombrar las dos capas, y un módulo de
//! `app/` no puede nombrar al adaptador sin ir contra la flecha del ID-81 —la
//! guarda `module_directions` lo denuncia, y con razón, porque lee la mitad de
//! producción de cada fichero de `src/` y un módulo declarado `#[cfg(test)]`
//! entero no tiene `mod tests` por el que cortar—. Como prueba de integración
//! entra por la puerta pública del crate y no hay flecha que torcer.

use serde_json::Value;

use rfirma_lib::app::frontier;
use rfirma_lib::app::site::{attend_launch, Attendance};
use rfirma_lib::channel::Situation as ChannelSituation;
use rfirma_lib::channel::{answer, Answer, ChannelDuty, ChannelError, OpenChannel, Shutdown};
use rfirma_lib::commands::failure::Failure;
use rfirma_lib::destination::{DestinationError, Situation as DestinationSituation};
use rfirma_lib::ffi::BridgeError;
use rfirma_lib::pkcs11::{Situation as TokenSituation, TokenError};
use rfirma_lib::protocol::{SafCode, WireAnswer};
use rfirma_lib::rubric::{RubricError, Situation as RubricSituation};
use rfirma_lib::signing::Refusal as Inadmissible;

/// El enlace que el portal concede, que es lo que **no** puede salir. El mismo
/// literal que usa la guarda hermana de `commands/guards.rs`.
const A_PORTAL_HANDLE: &str = "/run/user/1000/doc/1e8b83b9/contrato.pdf";

/// El nombre del documento, que tampoco sale: la sede sabe qué mandó, no cómo
/// se llama aquí.
const A_DOCUMENT_NAME: &str = "contrato.pdf";

/// El certificado con el que se firmaría. Es inventado a propósito: ningún dato
/// de una persona real entra en una prueba.
const A_CERTIFICATE: &str = "CN=CERTIFICADO DE PRUEBAS RFIRMA, SERIALNUMBER=IDCES-00000000T";

/// Los intentos de PIN que quedan, que son de la ventana y de nadie más.
const ATTEMPTS_LEFT: u32 = 2;

/// La credencial de canal de una invocación bien formada.
const CREDENTIAL: &str = "8jAkPZfRw2mQxN4TbYuL";

/// Un transporte que abre siempre y apunta el cometido con el que se le llamó:
/// el cometido es la otra mitad de lo que acaba en el cable.
fn a_transport(
    duties: &std::cell::RefCell<Vec<ChannelDuty>>,
) -> impl Fn(&[u16], ChannelDuty) -> Result<OpenChannel, ChannelError> + '_ {
    move |ports: &[u16], duty: ChannelDuty| {
        duties.borrow_mut().push(duty.clone());
        Ok(OpenChannel::new(ports[0], Shutdown::of(|| {})))
    }
}

/// **Todo lo que se queda dentro**, ya serializado como cruza a la ventana.
///
/// Cada fallo sale de su error de dominio con el enlace del portal, el nombre
/// del documento y el certificado dentro, que es donde de verdad viajan: en el
/// detalle crudo del ID-29.
///
/// Y los tres valores **por separado**, además de dentro del detalle: `Failure`
/// solo tiene tres campos, así que el detalle es una sola hoja que los
/// concatena, y buscar únicamente esa frase entera dejaría pasar una línea que
/// llevara sólo el nombre del documento o sólo el certificado. La comprobación
/// del criterio ("ni ruta, ni nombre de documento, ni certificado") es la de
/// cada uno suelto.
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
            "los valores contaminados de la URL",
            Value::Array(vec![
                Value::String(A_PORTAL_HANDLE.to_owned()),
                Value::String(A_DOCUMENT_NAME.to_owned()),
                Value::String(A_CERTIFICATE.to_owned()),
            ]),
        ),
    ]
}

/// **Todo lo que sale hacia la sede**, construido desde su caso de uso.
///
/// Dos orígenes, que son los dos únicos que escriben en el socket: el cometido
/// con el que [`attend_launch`] abre el canal, y lo que
/// [`rfirma_lib::channel::conversation`] contesta a cada mensaje. Y encima, la
/// traducción de cada situación del ID-29, que es lo que saldrá en cuanto haya
/// operaciones que fallen.
fn everything_that_goes_out_to_the_site() -> Vec<String> {
    let mut lines = Vec::new();

    let duties = std::cell::RefCell::new(Vec::new());
    let transport = a_transport(&duties);
    // Una invocación con el enlace del portal, el documento y el certificado
    // metidos en sus parámetros, y mal formada para que sea un rechazo: es el
    // peor caso, porque el rechazo es justo lo que sale por el cable.
    //
    // La versión es la **buena** (`v=4`) a propósito. Con una versión vieja el
    // rechazo se decide en `check_protocol_version`, antes de leer `idsession`,
    // `dat` y `fileid`, y entonces la línea que sale no depende de la URL: la
    // guarda pasaría sin que ninguno de los tres valores contaminados llegara a
    // tocarse. Con `v=4` el rechazo lo decide la credencial mal formada, que es
    // el parámetro contaminado, y la respuesta examinada nace de él.
    let url = format!(
        "afirma://websocket?ports=54001&v=4&idsession=no-vale-{A_CERTIFICATE}\
         &dat=file://{A_PORTAL_HANDLE}&fileid={A_DOCUMENT_NAME}"
    );

    match attend_launch(&url, &transport) {
        Attendance::RefusingOverTheChannel { answer, .. } => lines.push(answer.on_the_wire()),
        other => panic!("con puertos el rechazo sale por el socket: {other:?}"),
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

    lines
}

/// Cada hoja de un valor ya serializado: las cadenas y los números, que son lo
/// que puede aparecer copiado en otro sitio.
fn leaves(value: &Value, into: &mut Vec<String>) {
    match value {
        Value::String(text) => into.push(text.clone()),
        Value::Number(number) => into.push(number.to_string()),
        Value::Array(items) => items.iter().for_each(|item| leaves(item, into)),
        Value::Object(fields) => fields.values().for_each(|field| leaves(field, into)),
        _ => {}
    }
}

/// Si esa línea menciona esa hoja.
///
/// Una hoja que es un número se busca como **palabra entera**: el `02` de
/// `SAF_02` no es los dos intentos de PIN que quedan, y confundirlos pondría la
/// guarda roja para siempre por el motivo equivocado.
fn mentions(line: &str, leaf: &str) -> bool {
    if leaf.chars().all(|letter| letter.is_ascii_digit()) {
        return line
            .split(|letter: char| !letter.is_ascii_alphanumeric())
            .any(|word| word == leaf);
    }
    line.contains(leaf)
}

/// **Nada de lo que se queda dentro cruza el socket** (ID-291, TD-58).
///
/// Ni la ruta del portal, ni el nombre del documento, ni el certificado, ni los
/// intentos de PIN que quedan. Se recorre campo a campo lo que la ventana sí
/// recibe y se comprueba que ninguna de esas hojas aparece en ninguna de las
/// líneas que salen al cable.
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

/// Y lo que sale **no puede ser otra cosa que el catálogo** (ID-289, TD-57): la
/// lista de líneas posibles es finita y se calcula del `enum`, así que una
/// línea que no esté en ella es un código acuñado.
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
    // El `OK` del eco no es una respuesta de la frontera, pero sí sale por el
    // mismo socket.
    possible.push(rfirma_lib::channel::ECHO_OK.to_owned());

    for line in everything_that_goes_out_to_the_site() {
        assert!(
            possible.contains(&line),
            "«{line}» no la produce el catalogo cerrado: es un codigo acunado"
        );
    }
}

/// **`SAF_48` no se emite nunca** (ID-295), y esto se lee en el código y no en
/// una ejecución: el código del *shadow attack* sólo puede aparecer donde se
/// declara el catálogo y en esta guarda.
///
/// `PdfShadowAttackException` es de `master`; la 1.9.2 contra la que se firma no
/// la lanza, así que emitirlo sería inventarse una situación.
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

    // Sólo donde se declara el catálogo: esta guarda ya no vive en `src/`, y la
    // hermana de `commands/guards.rs` no nombra el código.
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
        // Sólo la mitad de producción, igual que las guardas hermanas: una
        // prueba que comprueba que el código **no** sale sí puede nombrarlo.
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
