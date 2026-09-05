//! **Qué se contesta a cada mensaje del canal**, sin socket delante.
//!
//! El servidor lee un mensaje, llama aquí y escribe lo que salga. Todo lo que
//! decide el canal cabe en esta función, y por eso se prueba entera como
//! función pura: las pruebas de las tres guardias no abren ningún puerto.
//!
//! Las guardias van **en este orden**, que es el del original
//! (`AfirmaWebSocketServerV4.onMessage`, `:57`-`93`):
//!
//! 1. La petición viene del *loopback*, o `SAF_47`.
//! 2. El mensaje repite la credencial del canal, o `SAF_46` —y no se ejecuta—.
//! 3. Sólo entonces se mira si es el eco o una operación.
//!
//! Y antes de las tres, la que el original no tiene: **para qué se abrió el
//! canal** ([`ChannelDuty`]). Un canal abierto para contestar un rechazo
//! (ID-248) no expone ninguna capacidad: conteste quien conteste y diga lo que
//! diga, recibe el `SAF_` y se cierra.

use crate::protocol::{ChannelCredential, ChannelMessage, SafCode};

/// La respuesta exacta al eco (`ECHO_OK_RESPONSE`,
/// `AfirmaWebSocketServerV4.java:35`).
pub const ECHO_OK: &str = "OK";

/// **Para qué se abrió el canal.**
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChannelDuty {
    /// Servir la conversación, cerrada con la credencial que trajo la
    /// invocación de arranque. Hoy la conversación es el eco y nada más.
    Serve(ChannelCredential),
    /// Contestar un rechazo al primer mensaje y cerrar (ID-248).
    ///
    /// Es lo que rfirma hace **mejor que el original**, que se mata y deja a la
    /// sede reintentando quince veces hasta un `ApplicationNotFoundException`
    /// falso.
    Refuse(SafCode),
}

/// Lo que el servidor hace con un mensaje.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    /// Escribe esto y sigue escuchando.
    Reply(String),
    /// Escribe esto y cierra el canal.
    ReplyAndClose(String),
}

impl Answer {
    /// El texto que se escribe, sea cual sea el desenlace.
    pub fn text(&self) -> &str {
        match self {
            Self::Reply(text) | Self::ReplyAndClose(text) => text,
        }
    }
}

/// Qué se contesta a este mensaje.
///
/// `from_loopback` es de dónde vino la conexión. El escuchador ya está atado a
/// `127.0.0.1` ([`crate::channel::bind`]), así que en la práctica siempre es
/// cierto; la guardia se reproduce igual porque es la del original y porque una
/// segunda cerradura no depende de que la primera siga puesta mañana.
pub fn answer(duty: &ChannelDuty, from_loopback: bool, message: &str) -> Answer {
    if !from_loopback {
        return Answer::ReplyAndClose(SafCode::ExternalRequestToSocket.on_the_wire());
    }

    let credential = match duty {
        ChannelDuty::Refuse(code) => return Answer::ReplyAndClose(code.on_the_wire()),
        ChannelDuty::Serve(credential) => credential,
    };

    let message = ChannelMessage::read(message);
    if message.credential() != Some(credential.as_str()) {
        return Answer::ReplyAndClose(SafCode::InvalidSessionId.on_the_wire());
    }

    match message {
        ChannelMessage::Echo { .. } => Answer::Reply(ECHO_OK.to_owned()),
        // Las operaciones —`selectcert`, `sign`, `cosign`— llegan con el
        // trámite de sede; hoy no hay ninguna registrada, y una operación que
        // no se atiende es `SAF_04` en el original.
        //
        // `NotOfTheProtocol` no llega hasta aquí: no repite ninguna credencial,
        // así que la guardia de arriba ya lo ha sacado con `SAF_46` —igual que
        // el original, que mira el `idsession` antes de mirar el mensaje—. Por
        // eso comparte brazo en vez de tener uno propio con un código que nadie
        // produciría.
        ChannelMessage::Operation { .. } | ChannelMessage::NotOfTheProtocol => {
            Answer::ReplyAndClose(SafCode::UnsupportedOperation.on_the_wire())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CREDENTIAL: &str = "8jAkPZfRw2mQxN4TbYuL";

    fn serving() -> ChannelDuty {
        ChannelDuty::Serve(
            ChannelCredential::parse(CREDENTIAL).expect("veinte alfanumericos son credencial"),
        )
    }

    fn echo_with(credential: &str) -> String {
        format!("echo=-idsession={credential}@EOF")
    }

    #[test]
    fn the_echo_of_the_published_client_is_answered_with_ok_and_the_channel_stays_open() {
        let answer = answer(&serving(), true, &echo_with(CREDENTIAL));

        assert_eq!(answer, Answer::Reply("OK".to_owned()));
    }

    /// La credencial es lo único que impide que otra página del equipo use el
    /// canal: sin ella, no se ejecuta nada.
    #[test]
    fn an_echo_with_another_credential_gets_the_invalid_session_code() {
        let answer = answer(&serving(), true, &echo_with("otraPaginaDelEquipo0"));

        assert_eq!(
            answer,
            Answer::ReplyAndClose("SAF_46: Id de sesion invalido".to_owned())
        );
    }

    #[test]
    fn an_echo_without_credential_gets_the_invalid_session_code_too() {
        let answer = answer(&serving(), true, "echo=@EOF");

        assert!(answer.text().starts_with("SAF_46"));
    }

    /// La guardia de origen es la primera de las tres, y se contesta **antes**
    /// de mirar la credencial.
    #[test]
    fn a_request_that_does_not_come_from_the_loopback_is_refused_first() {
        let answer = answer(&serving(), false, &echo_with(CREDENTIAL));

        assert_eq!(
            answer,
            Answer::ReplyAndClose(
                "SAF_47: Peticion al socket desde IP externa o sin identificar".to_owned()
            )
        );
    }

    /// Un canal abierto sólo para contestar un rechazo (ID-248) **no expone
    /// ninguna capacidad**: ni con la credencial buena hace otra cosa.
    #[test]
    fn a_channel_opened_to_refuse_answers_the_code_to_whatever_arrives_and_closes() {
        let duty = ChannelDuty::Refuse(SafCode::UnsupportedProcedure);

        for message in [echo_with(CREDENTIAL), "afirma://sign?op=sign".to_owned()] {
            assert_eq!(
                answer(&duty, true, &message),
                Answer::ReplyAndClose(
                    "SAF_21: La version de Autofirma instalada no es compatible con este tramite"
                        .to_owned()
                )
            );
        }
    }

    #[test]
    fn an_operation_is_not_attended_yet_and_says_so_with_the_code_of_the_original() {
        let message = format!("afirma://sign?op=sign&idsession={CREDENTIAL}");

        let answer = answer(&serving(), true, &message);

        assert_eq!(
            answer,
            Answer::ReplyAndClose("SAF_04: Operacion no soportada".to_owned())
        );
    }

    #[test]
    fn something_that_is_not_of_the_protocol_never_reaches_the_operations() {
        let answer = answer(&serving(), true, "GET / HTTP/1.1");

        assert!(
            answer.text().starts_with("SAF_46"),
            "sin credencial no se llega ni a mirar que es: {}",
            answer.text()
        );
    }

    /// Todo lo que sale al cable o es `OK` o empieza por `SAF_`: cualquier otra
    /// cosa la entrega el cliente publicado como si fuera una firma (§5 del
    /// informe del protocolo).
    #[test]
    fn everything_that_goes_out_is_either_ok_or_a_saf_code() {
        let messages = [
            echo_with(CREDENTIAL),
            echo_with("otra"),
            "GET / HTTP/1.1".to_owned(),
            "afirma://sign?op=sign".to_owned(),
            String::new(),
        ];

        for duty in [serving(), ChannelDuty::Refuse(SafCode::Params)] {
            for from_loopback in [true, false] {
                for message in &messages {
                    let text = answer(&duty, from_loopback, message).text().to_owned();
                    assert!(
                        text == ECHO_OK || text.starts_with("SAF_"),
                        "el canal ha escrito algo que la sede tomaria por una firma: {text}"
                    );
                }
            }
        }
    }
}
