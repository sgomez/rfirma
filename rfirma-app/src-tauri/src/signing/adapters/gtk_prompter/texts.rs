//! Los textos del diálogo del secreto en los cinco idiomas: título, botones, fallo y aviso del PIN (ADR-0009, ADR-0047).

use crate::identity::domain::secret::{PinWarning, SecretName};
use crate::identity::ports::SecretPromptRequest;
use crate::signing::domain::Language;

/// Estructura interna con los textos localizados para el diálogo modal del secreto.
#[derive(Debug, PartialEq, Eq)]
pub struct DialogI18n {
    pub title: String,
    pub holder_name: Option<String>,
    pub id_number: Option<String>,
    pub incorrect_secret: String,
    pub pin_warning: Option<String>,
    pub accept: &'static str,
    pub cancel: &'static str,
}

fn title_text(lang: Language, secret: &SecretName) -> String {
    match (lang, secret) {
        (Language::Spanish, SecretName::Pin) => "Introduce el PIN".to_string(),
        (Language::Spanish, SecretName::Password) => "Introduce la contraseña".to_string(),

        (Language::Catalan, SecretName::Pin) => "Introdueix el PIN".to_string(),
        (Language::Catalan, SecretName::Password) => "Introdueix la contrasenya".to_string(),

        (Language::Basque, SecretName::Pin) => "Sartu PINa".to_string(),
        (Language::Basque, SecretName::Password) => "Sartu pasahitza".to_string(),

        (Language::Galician, SecretName::Pin) => "Introduce o PIN".to_string(),
        (Language::Galician, SecretName::Password) => "Introduce o contrasinal".to_string(),

        (Language::English, SecretName::Pin) => "Enter PIN".to_string(),
        (Language::English, SecretName::Password) => "Enter password".to_string(),

        (Language::Spanish, SecretName::DocumentPassword) => {
            "Introduce la contraseña del PDF".to_string()
        }
        (Language::Catalan, SecretName::DocumentPassword) => {
            "Introdueix la contrasenya del PDF".to_string()
        }
        (Language::Basque, SecretName::DocumentPassword) => "Sartu PDFaren pasahitza".to_string(),
        (Language::Galician, SecretName::DocumentPassword) => {
            "Introduce o contrasinal do PDF".to_string()
        }
        (Language::English, SecretName::DocumentPassword) => "Enter the PDF password".to_string(),

        (Language::Spanish, SecretName::Pkcs12Password(file)) => {
            format!("Introduce la contraseña de {file}")
        }
        (Language::Catalan, SecretName::Pkcs12Password(file)) => {
            format!("Introdueix la contrasenya de {file}")
        }
        (Language::Basque, SecretName::Pkcs12Password(file)) => {
            format!("Sartu {file} fitxategiaren pasahitza")
        }
        (Language::Galician, SecretName::Pkcs12Password(file)) => {
            format!("Introduce o contrasinal de {file}")
        }
        (Language::English, SecretName::Pkcs12Password(file)) => {
            format!("Enter the password for {file}")
        }
    }
}

fn button_texts(lang: Language) -> (&'static str, &'static str) {
    match lang {
        Language::Spanish => ("Aceptar", "Cancelar"),
        Language::Catalan => ("Acceptar", "Cancel·lar"),
        Language::Basque => ("Onartu", "Utzi"),
        Language::Galician => ("Aceptar", "Cancelar"),
        Language::English => ("OK", "Cancel"),
    }
}

fn incorrect_secret_text(lang: Language, secret: &SecretName) -> String {
    match (lang, secret) {
        (Language::Spanish, SecretName::Pin) => "PIN incorrecto. Vuelve a intentarlo.".to_string(),
        (Language::Spanish, SecretName::Password | SecretName::Pkcs12Password(_)) => {
            "Contraseña incorrecta. Vuelve a intentarlo.".to_string()
        }

        (Language::Catalan, SecretName::Pin) => "PIN incorrecte. Torna-ho a provar.".to_string(),
        (Language::Catalan, SecretName::Password | SecretName::Pkcs12Password(_)) => {
            "Contrasenya incorrecta. Torna-ho a provar.".to_string()
        }

        (Language::Basque, SecretName::Pin) => "PIN okerra. Saiatu berriro.".to_string(),
        (Language::Basque, SecretName::Password | SecretName::Pkcs12Password(_)) => {
            "Pasahitza okerra. Saiatu berriro.".to_string()
        }

        (Language::Galician, SecretName::Pin) => "PIN incorrecto. Tenta de novo.".to_string(),
        (Language::Galician, SecretName::Password | SecretName::Pkcs12Password(_)) => {
            "Contrasinal incorrecto. Tenta de novo.".to_string()
        }

        (Language::English, SecretName::Pin) => "Incorrect PIN. Try again.".to_string(),
        (Language::English, SecretName::Password | SecretName::Pkcs12Password(_)) => {
            "Incorrect password. Try again.".to_string()
        }

        (Language::Spanish, SecretName::DocumentPassword) => {
            "Contraseña del PDF incorrecta. Vuelve a intentarlo.".to_string()
        }
        (Language::Catalan, SecretName::DocumentPassword) => {
            "Contrasenya del PDF incorrecta. Torna-ho a provar.".to_string()
        }
        (Language::Basque, SecretName::DocumentPassword) => {
            "PDFaren pasahitza okerra. Saiatu berriro.".to_string()
        }
        (Language::Galician, SecretName::DocumentPassword) => {
            "Contrasinal do PDF incorrecto. Tenta de novo.".to_string()
        }
        (Language::English, SecretName::DocumentPassword) => {
            "Incorrect PDF password. Try again.".to_string()
        }
    }
}

fn pin_warning_text(lang: Language, warning: PinWarning) -> Option<&'static str> {
    let text = match (lang, warning) {
        (_, PinWarning::Quiet) => return None,
        (Language::Spanish, PinWarning::FinalTry) => {
            "Último intento: si el PIN no es correcto, la tarjeta se bloqueará."
        }
        (Language::Spanish, PinWarning::CountLow) => {
            "Ya ha habido algún intento fallido con esta tarjeta."
        }
        (Language::Catalan, PinWarning::FinalTry) => {
            "Darrer intent: si el PIN no és correcte, la targeta es bloquejarà."
        }
        (Language::Catalan, PinWarning::CountLow) => {
            "Ja hi ha hagut algun intent fallit amb aquesta targeta."
        }
        (Language::Basque, PinWarning::FinalTry) => {
            "Azken saiakera: PINa zuzena ez bada, txartela blokeatu egingo da."
        }
        (Language::Basque, PinWarning::CountLow) => {
            "Txartel honekin dagoeneko saiakera okerren bat egon da."
        }
        (Language::Galician, PinWarning::FinalTry) => {
            "Último intento: se o PIN non é correcto, a tarxeta bloquearase."
        }
        (Language::Galician, PinWarning::CountLow) => {
            "Xa houbo algún intento fallido con esta tarxeta."
        }
        (Language::English, PinWarning::FinalTry) => {
            "Last try: if the PIN is wrong, the card will be locked."
        }
        (Language::English, PinWarning::CountLow) => {
            "There has already been a failed try with this card."
        }
    };
    Some(text)
}

/// Traduce los textos del diálogo a uno de los 5 idiomas oficiales de rFirma (ADR-0009).
pub fn localize(request: &SecretPromptRequest) -> DialogI18n {
    let (accept, cancel) = button_texts(request.language);

    DialogI18n {
        title: title_text(request.language, &request.secret),
        holder_name: request.holder.as_ref().map(|holder| holder.name.clone()),
        id_number: request
            .holder
            .as_ref()
            .map(|holder| holder.id_number.clone())
            .filter(|id_number| !id_number.is_empty()),
        incorrect_secret: incorrect_secret_text(request.language, &request.secret),
        pin_warning: (request.secret == SecretName::Pin)
            .then(|| pin_warning_text(request.language, request.pin_warning))
            .flatten()
            .map(str::to_owned),
        accept,
        cancel,
    }
}
