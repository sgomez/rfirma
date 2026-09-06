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
//! Hay **tres desenlaces** y no dos: contestar, contestar y cerrar, y aceptar
//! la operación y **dejarla pendiente** (ID-320). El tercero es el de las
//! operaciones de sede, que no tienen respuesta en el momento en que llegan:
//! entre el mensaje y su respuesta hay una persona que tiene que consentir. Aun
//! así esta función sigue siendo pura y síncrona —no recibe el escritorio del
//! trámite ni nada que haga entrada y salida—: lo único que decide es que el
//! mensaje es una operación legítima que va al trámite.
//!
//! Y antes de las tres, la que el original no tiene: **para qué se abrió el
//! canal** ([`ChannelDuty`]). Un canal abierto para contestar un rechazo
//! (ID-248) no expone ninguna capacidad: conteste quien conteste y diga lo que
//! diga, recibe el `SAF_` y se cierra.

use crate::protocol::{
    AfirmaUrl, ChannelCredential, ChannelMessage, Parameter, SafCode, WireAnswer,
};

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
    Refuse(WireAnswer),
}

/// Lo que el servidor hace con un mensaje.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    /// Escribe esto y sigue escuchando.
    Reply(String),
    /// Escribe esto y cierra el canal.
    ReplyAndClose(String),
    /// **La operación se acepta y queda pendiente** (ID-320): no se escribe
    /// nada y la conexión no se cierra. Quien contesta es el trámite, cuando la
    /// persona haya consentido o dicho que no, y lo hace por el asa de
    /// respuesta ([`crate::channel::ReplyHandle`], ID-321).
    ///
    /// Lleva dentro la operación ya partida: leerla es lo único que decide esta
    /// función, y volver a leerla en el servidor sería partir la misma URL dos
    /// veces.
    Pending(AfirmaUrl),
}

impl Answer {
    /// El texto que se escribe, cuando se escribe alguno.
    ///
    /// La operación pendiente no escribe ninguno todavía (ID-320), y por eso
    /// esto es un [`Option`] y no una cadena vacía: nada de lo que sale al
    /// cable puede confundirse con «no ha salido nada».
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Reply(text) | Self::ReplyAndClose(text) => Some(text),
            Self::Pending(_) => None,
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
        return Answer::ReplyAndClose(
            WireAnswer::refused(SafCode::ExternalRequestToSocket).on_the_wire(),
        );
    }

    let credential = match duty {
        ChannelDuty::Refuse(answer) => return Answer::ReplyAndClose(answer.on_the_wire()),
        ChannelDuty::Serve(credential) => credential,
    };

    let message = ChannelMessage::read(message);
    if message.credential() != Some(credential.as_str()) {
        return Answer::ReplyAndClose(
            WireAnswer::refused_because_of(SafCode::InvalidSessionId, Parameter::IdSession)
                .on_the_wire(),
        );
    }

    match message {
        ChannelMessage::Echo { .. } => Answer::Reply(ECHO_OK.to_owned()),
        // Las operaciones —`selectcert`, `sign`, `cosign`— van al trámite, y
        // **no se contestan aquí**: entre el mensaje y su respuesta hay una
        // persona que tiene que consentir (ID-320). Qué operación es, si se
        // atiende y con qué código se rechaza cuando no, lo decide
        // [`crate::app::errand::attend_operation`]; esta función sólo dice que
        // el mensaje es legítimo.
        ChannelMessage::Operation { url } => Answer::Pending(url),
        // `NotOfTheProtocol` no llega hasta aquí: no repite ninguna credencial,
        // así que la guardia de arriba ya lo ha sacado con `SAF_46` —igual que
        // el original, que mira el `idsession` antes de mirar el mensaje—. Se
        // contesta con el `SAF_04` del original por si algún día llegara.
        ChannelMessage::NotOfTheProtocol => {
            Answer::ReplyAndClose(WireAnswer::refused(SafCode::UnsupportedOperation).on_the_wire())
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

    /// Lo que se escribió, cuando la prueba da por hecho que se escribió algo.
    fn written(answer: &Answer) -> &str {
        answer.text().expect("esta respuesta escribe en el cable")
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
            Answer::ReplyAndClose(
                "SAF_46: Id de sesion invalido; el parametro que falla es 'idsession'".to_owned()
            )
        );
    }

    #[test]
    fn an_echo_without_credential_gets_the_invalid_session_code_too() {
        let answer = answer(&serving(), true, "echo=@EOF");

        assert!(written(&answer).starts_with("SAF_46"));
    }

    /// La guardia de origen es la primera de las tres, y se contesta **antes**
    /// de mirar la credencial.
    #[test]
    fn a_request_that_does_not_come_from_the_loopback_is_refused_first() {
        let answer = answer(&serving(), false, &echo_with(CREDENTIAL));

        assert_eq!(
            answer,
            Answer::ReplyAndClose(
                "SAF_47: Peticion al canal desde una direccion externa o sin identificar"
                    .to_owned()
            )
        );
    }

    /// Un canal abierto sólo para contestar un rechazo (ID-248) **no expone
    /// ninguna capacidad**: ni con la credencial buena hace otra cosa.
    #[test]
    fn a_channel_opened_to_refuse_answers_the_code_to_whatever_arrives_and_closes() {
        let duty = ChannelDuty::Refuse(WireAnswer::refused(SafCode::UnsupportedProcedure));

        for message in [echo_with(CREDENTIAL), "afirma://sign?op=sign".to_owned()] {
            assert_eq!(
                answer(&duty, true, &message),
                Answer::ReplyAndClose(
                    "SAF_21: Este tramite no es compatible con la version instalada".to_owned()
                )
            );
        }
    }

    /// **TD-73.** Una operación legítima **queda pendiente**: no se escribe
    /// nada, el canal no se cierra y quien contesta es el trámite.
    #[test]
    fn a_legitimate_operation_is_left_pending_and_nothing_is_written() {
        let message = format!("afirma://selectcert?op=selectcert&idsession={CREDENTIAL}");

        let answer = answer(&serving(), true, &message);

        let Answer::Pending(url) = answer else {
            panic!("la operacion va al tramite: {answer:?}");
        };
        assert_eq!(url.verb(), "selectcert");
    }

    /// **TD-73**, la otra mitad: ninguna de las tres guardias del original deja
    /// pasar la operación, y las tres contestan en el acto.
    #[test]
    fn none_of_the_three_guards_lets_an_operation_through() {
        let operation = format!("afirma://selectcert?op=selectcert&idsession={CREDENTIAL}");
        let refusing = ChannelDuty::Refuse(WireAnswer::refused(SafCode::UnsupportedProcedure));

        let guarded = [
            // El origen: ni siquiera se mira qué pide.
            answer(&serving(), false, &operation),
            // La credencial del canal.
            answer(
                &serving(),
                true,
                "afirma://selectcert?op=selectcert&idsession=otraPaginaDelEquipo0",
            ),
            // Y el cometido: un canal abierto para rechazar no atiende nada.
            answer(&refusing, true, &operation),
        ];

        for answer in guarded {
            assert!(
                matches!(answer, Answer::ReplyAndClose(_)),
                "la guardia contesta en el acto y cierra: {answer:?}"
            );
            assert!(written(&answer).starts_with("SAF_"));
        }
    }

    #[test]
    fn something_that_is_not_of_the_protocol_never_reaches_the_operations() {
        let answer = answer(&serving(), true, "GET / HTTP/1.1");

        assert!(
            written(&answer).starts_with("SAF_46"),
            "sin credencial no se llega ni a mirar que es: {}",
            written(&answer)
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
            format!("afirma://selectcert?op=selectcert&idsession={CREDENTIAL}"),
            String::new(),
        ];

        for duty in [
            serving(),
            ChannelDuty::Refuse(WireAnswer::refused(SafCode::Params)),
        ] {
            for from_loopback in [true, false] {
                for message in &messages {
                    // La operación pendiente no escribe **nada**, que es la
                    // única forma de salir de aquí sin ser `OK` ni `SAF_`
                    // (ID-320): lo que conteste después pasa por
                    // el códec negociado ([`crate::app::codec::V4Codec`]).
                    let Some(text) = answer(&duty, from_loopback, message)
                        .text()
                        .map(str::to_owned)
                    else {
                        continue;
                    };
                    assert!(
                        text == ECHO_OK || text.starts_with("SAF_"),
                        "el canal ha escrito algo que la sede tomaria por una firma: {text}"
                    );
                }
            }
        }
    }
}
