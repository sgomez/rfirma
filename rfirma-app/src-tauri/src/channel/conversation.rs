//! Evaluación y respuesta a los mensajes del canal local (ADR-0005).

use crate::protocol::{
    AfirmaUrl, ChannelCredential, ChannelMessage, Parameter, SafCode, WireAnswer,
};

/// Respuesta exacta al mensaje de eco del protocolo.
pub const ECHO_OK: &str = "OK";

/// Cometido con el que se abrió el canal local.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChannelDuty {
    /// Servir la conversación con la credencial acordada.
    Serve(ChannelCredential),
    /// Contestar un rechazo al primer mensaje y cerrar.
    Refuse(WireAnswer),
}

/// Respuesta del servidor ante un mensaje recibido por el canal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    /// Escribe la respuesta y continúa escuchando.
    Reply(String),
    /// Escribe la respuesta y cierra la conexión.
    ReplyAndClose(String),
    /// La operación se acepta y queda pendiente de resolución.
    Pending(AfirmaUrl),
}

impl Answer {
    /// Texto de la respuesta, si corresponde enviar alguno.
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Reply(text) | Self::ReplyAndClose(text) => Some(text),
            Self::Pending(_) => None,
        }
    }
}

/// Determina la respuesta a un mensaje recibido en el canal.
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
        ChannelMessage::Operation { url } => Answer::Pending(url),
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

    #[test]
    fn a_legitimate_operation_is_left_pending_and_nothing_is_written() {
        let message = format!("afirma://selectcert?op=selectcert&idsession={CREDENTIAL}");

        let answer = answer(&serving(), true, &message);

        let Answer::Pending(url) = answer else {
            panic!("la operacion va al tramite: {answer:?}");
        };
        assert_eq!(url.verb(), "selectcert");
    }

    #[test]
    fn none_of_the_three_guards_lets_an_operation_through() {
        let operation = format!("afirma://selectcert?op=selectcert&idsession={CREDENTIAL}");
        let refusing = ChannelDuty::Refuse(WireAnswer::refused(SafCode::UnsupportedProcedure));

        let guarded = [
            answer(&serving(), false, &operation),
            answer(
                &serving(),
                true,
                "afirma://selectcert?op=selectcert&idsession=otraPaginaDelEquipo0",
            ),
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
