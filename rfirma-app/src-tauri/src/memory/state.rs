//! El **estado**: lo que la aplicación acumula por su cuenta (ID-31,
//! ADR-0010).
//!
//! Las otras tres memorias: los documentos recientes, la última configuración
//! de firma visible y el certificado usado. Y, colgada de ellas, la última
//! carpeta de la que se abrió algo, que fuera del arenero sí se puede saber.
//! Vive en un fichero aparte del de la configuración porque en Windows **no
//! debe viajar en un perfil móvil**, y porque borrarlo no reconfigura la
//! aplicación.
//!
//! Del certificado se guarda **cómo volver a encontrarlo** —módulo PKCS#11,
//! etiqueta del token y etiqueta del objeto—, **nunca titular ni DNI** (ID-32).
//! Esto no es una precaución: es un fichero en el disco del usuario. El tipo
//! que se persiste es [`CertificateRef`], que no tiene sitio donde meter un
//! titular aunque alguien quisiera; el titular se relee del DER del token cada
//! vez que hace falta pintarlo.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::pkcs11::CertificateRef;
use crate::signing::SignatureBox;

use super::recents::Recents;

/// Lo que la aplicación recuerda sola.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    /// La bandeja de documentos.
    pub recents: Recents,
    /// Dónde cayó el recuadro la última vez. `None` mientras no se haya
    /// firmado nada, y también cuando «Recordar la última configuración de
    /// firma visible» está apagado: apagado significa **no guardarla**.
    pub visible_signature: Option<SignatureBox>,
    /// Cómo volver a encontrar el certificado que se usó. Se relee del token al
    /// arrancar; si no está, el panel vuelve a «Sin certificado» sin ruido.
    pub certificate: Option<CertificateRef>,
    /// La última carpeta de la que se abrió un documento, para volver a abrir
    /// el diálogo ahí (enmienda del ADR-0011).
    ///
    /// Es `None` **siempre bajo el arenero**, y no por precaución: allí el
    /// diálogo devuelve un enlace del portal y la carpeta real no se puede
    /// saber. En los canales sin arenero —deb, rpm, Windows, macOS— sí se
    /// sabe, y entonces se recuerda.
    ///
    /// Es la única ruta del anfitrión que se guarda además de las de los
    /// recientes, y es **estado y no configuración**: la acumula la aplicación
    /// sola, nadie la elige, y por eso «Recordar mi actividad» se la lleva
    /// como se lleva todo lo demás.
    pub last_open_folder: Option<PathBuf>,
}

impl State {
    /// Olvida todo lo acumulado, dejando el estado como en el primer arranque.
    ///
    /// Es lo que cubre «Recordar mi actividad»: recientes **y** certificado, la
    /// misma promesa a quien firma en un ordenador compartido. La última
    /// configuración de firma visible se va con ellos, y la última carpeta
    /// abierta también: es estado, y el interruptor promete no recordar
    /// actividad. Una carpeta del anfitrión que sobreviviera a «Vaciar la
    /// lista» diría por dónde anduvo quien firmó antes.
    pub fn forget_everything(&mut self) {
        *self = Self::default();
    }

    /// Si no hay nada que recordar.
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::recents::{Badge, RecentDocument};
    use crate::pkcs11::TokenCertificate;
    use std::fs;
    use std::path::Path;
    use std::time::SystemTime;

    /// **Grada A**: ni token, ni librería nativa, ni red. El certificado se
    /// simula: lo que se prueba es qué se escribe, no cómo se lee del token.
    fn a_document(directory: &Path) -> RecentDocument {
        let path = directory.join("contrato.pdf");
        fs::write(&path, b"%PDF-1.7 de prueba").expect("deberia escribirse");
        RecentDocument::seen(&path, Badge::Unsigned, SystemTime::now()).expect("deberia anotarse")
    }

    #[test]
    fn a_fresh_state_remembers_nothing() {
        assert!(State::default().is_empty());
    }

    #[test]
    fn forgetting_everything_covers_the_recents_and_the_certificate_alike() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let mut state = State {
            certificate: Some(CertificateRef::new(
                "/usr/lib/softhsm/libsofthsm2.so",
                "rfirma-test",
                "Certificado de pruebas",
            )),
            visible_signature: Some(SignatureBox {
                page: 1,
                lower_left_x: 10,
                lower_left_y: 20,
                upper_right_x: 110,
                upper_right_y: 70,
            }),
            ..State::default()
        };
        state.recents.record(a_document(directory.path()));

        state.forget_everything();

        assert!(state.is_empty());
        assert!(state.recents.is_empty());
        assert!(state.certificate.is_none());
        assert!(state.visible_signature.is_none());
    }

    /// La carpeta de la que se abrió algo es actividad como el resto: contar
    /// por dónde anduvo quien firmó antes es justo lo que el interruptor
    /// promete no hacer.
    #[test]
    fn forgetting_everything_also_covers_the_last_folder_opened() {
        let mut state = State {
            last_open_folder: Some(PathBuf::from("/home/quien/Contratos")),
            ..State::default()
        };

        state.forget_everything();

        assert!(state.last_open_folder.is_none());
    }

    #[test]
    fn the_persisted_certificate_is_how_to_find_it_again_and_nothing_else() {
        let state = State {
            certificate: Some(CertificateRef::new(
                "/usr/lib/softhsm/libsofthsm2.so",
                "rfirma-test",
                "Certificado de pruebas",
            )),
            ..State::default()
        };

        let written = serde_json::to_value(&state).expect("deberia serializarse");

        let fields: Vec<&str> = written["certificate"]
            .as_object()
            .expect("deberia ser un objeto")
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(fields, vec!["label", "module", "token_label"]);
    }

    /// El requisito del ID-32 dicho como lo dice el issue: **ni el titular ni
    /// el DNI** pueden aparecer en el fichero. El certificado del token trae
    /// los dos dentro del DER; lo que se persiste sale de él y no los lleva.
    #[test]
    fn neither_the_holder_nor_the_id_number_reach_the_persisted_file() {
        let holder = "APELLIDO APELLIDO NOMBRE - 12345678Z";
        let mut der = b"\x30\x82 certificado de mentira ".to_vec();
        der.extend_from_slice(holder.as_bytes());
        let certificate = TokenCertificate::new(
            CertificateRef::new(
                "/usr/lib/softhsm/libsofthsm2.so",
                "rfirma-test",
                "Certificado de pruebas",
            ),
            der,
        );

        let state = State {
            certificate: Some(certificate.reference().clone()),
            ..State::default()
        };
        let written = serde_json::to_string(&state).expect("deberia serializarse");

        assert!(
            written.contains("libsofthsm2.so") && written.contains("Certificado de pruebas"),
            "se guarda como volver a encontrarlo"
        );
        assert!(
            !written.contains(holder),
            "el titular no puede estar en el fichero"
        );
        assert!(
            !written.contains("12345678Z"),
            "el DNI no puede estar en el fichero"
        );
    }
}
