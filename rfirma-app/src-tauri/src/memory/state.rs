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
//! etiqueta del token, etiqueta del objeto y su `CKA_ID`—, **nunca titular ni
//! DNI** (ID-32). El `CKA_ID` es lo que lo identifica de verdad: en un almacén
//! real la etiqueta se repite, y sin él lo persistido no basta para reencontrar
//! el certificado **exacto**.
//! Esto no es una precaución: es un fichero en el disco del usuario. El tipo
//! que se persiste es [`CertificateRef`], que no tiene sitio donde meter un
//! titular aunque alguien quisiera; el titular se relee del DER del token cada
//! vez que hace falta pintarlo.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::pkcs11::CertificateRef;

use super::recents::Recents;

/// Lo que la aplicación recuerda sola.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    /// La bandeja de documentos.
    pub recents: Recents,
    /// Lo **global** de la firma visible: el interruptor, las cinco casillas,
    /// el motivo y el tamaño del recuadro (ID-74). `None` mientras nadie haya
    /// tocado el panel, y también cuando «Recordar la última configuración de
    /// firma visible» está apagado: apagado significa **no guardarla**.
    ///
    /// **La página y la posición no están aquí**: son de cada documento y
    /// viven en su fila de recientes
    /// ([`Placement`](super::recents::Placement)).
    pub visible_signature: Option<VisibleSignatureMemory>,
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
    /// El tamaño de la ventana y si estaba maximizada (ID-72).
    ///
    /// **Es la única memoria exenta de «Recordar mi actividad»** (ID-73): el
    /// tamaño de una ventana no dice qué hizo quien firmó antes, así que
    /// apagar el interruptor —o vaciar la actividad— no se la lleva. Por eso
    /// [`State::forget_everything`] la conserva en vez de vaciarla con el
    /// resto.
    ///
    /// **La posición no se guarda**, y no en ningún campo de aquí: en Wayland
    /// el cliente no puede pedirla, así que unas coordenadas guardadas serían
    /// una promesa que el compositor incumple (ADR-0010, enmienda).
    pub window: Option<WindowMemory>,
}

/// El tamaño de la ventana entre sesiones, sin la posición (ID-72).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WindowMemory {
    /// El ancho, en píxeles lógicos.
    pub width: f64,
    /// El alto, en píxeles lógicos.
    pub height: f64,
    /// Si estaba maximizada al cerrar.
    pub maximized: bool,
}

/// Lo **global** de la firma visible, lo mismo para todos los documentos
/// (ID-74).
///
/// El reparto no es un renombrado: lo que se persistía antes era **solo la
/// geometría del recuadro** —página y cuatro esquinas— y lo que maneja el panel
/// son interruptor, casillas, rúbrica y motivo. No se solapan, así que hay dos
/// tipos y no uno.
///
/// El **tamaño** sí es global y la **posición** no: el tamaño no depende de la
/// página, y reponer sobre un documento nuevo una posición elegida para otro es
/// lo que rechaza el ID-22.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VisibleSignatureMemory {
    /// El interruptor: si se estampa recuadro.
    pub enabled: bool,
    /// Si la rúbrica va dentro del recuadro. Es la quinta casilla.
    pub rubric: bool,
    /// Las cuatro casillas de texto.
    pub fields: RememberedFields,
    /// El motivo escrito. Vacío es «sin motivo».
    pub reason: String,
    /// El tamaño del recuadro, en espacio de usuario PDF.
    pub size: BoxSize,
}

/// Las cuatro casillas de texto del recuadro. La rúbrica va aparte: es una
/// imagen, no un dato del titular.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RememberedFields {
    pub signer_name: bool,
    pub id_number: bool,
    pub signed_at: bool,
    pub reason: bool,
}

/// El tamaño del recuadro, en puntos de espacio de usuario PDF.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BoxSize {
    pub width: f64,
    pub height: f64,
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
    ///
    /// El tamaño de la ventana **no** se va con el resto (ID-73): es la única
    /// memoria exenta.
    pub fn forget_everything(&mut self) {
        let window = self.window.take();
        *self = Self::default();
        self.window = window;
    }

    /// Si no hay nada que recordar.
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::recents::{Badge, Placement, RecentDocument};
    use crate::pkcs11::TokenCertificate;
    use crate::signing::PageSet;
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
                vec![0x01],
            )),
            visible_signature: Some(VisibleSignatureMemory {
                enabled: true,
                size: BoxSize {
                    width: 100.0,
                    height: 50.0,
                },
                ..VisibleSignatureMemory::default()
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

    /// El ID-73 dicho en `State`: la ventana es la única memoria que
    /// `forget_everything` no toca.
    #[test]
    fn forgetting_everything_leaves_the_window_size_alone() {
        let mut state = State {
            certificate: Some(CertificateRef::new(
                "/usr/lib/softhsm/libsofthsm2.so",
                "rfirma-test",
                "Certificado de pruebas",
                vec![0x01],
            )),
            window: Some(WindowMemory {
                width: 1024.0,
                height: 768.0,
                maximized: false,
            }),
            ..State::default()
        };

        state.forget_everything();

        assert!(state.certificate.is_none());
        assert_eq!(
            state.window,
            Some(WindowMemory {
                width: 1024.0,
                height: 768.0,
                maximized: false,
            })
        );
    }

    /// El ID-74 dicho como lo dice el issue: **lo global por un lado y lo de
    /// cada documento por otro**. Lo que se persistía antes era una sola cosa
    /// —la geometría del recuadro— y ahora son dos tipos que no comparten
    /// ningún campo.
    #[test]
    fn what_is_global_and_what_is_of_each_document_are_two_different_places() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let mut state = State {
            visible_signature: Some(VisibleSignatureMemory {
                enabled: true,
                rubric: true,
                fields: RememberedFields {
                    signer_name: true,
                    id_number: true,
                    signed_at: true,
                    reason: false,
                },
                reason: "Conforme".to_owned(),
                size: BoxSize {
                    width: 200.0,
                    height: 80.0,
                },
            }),
            ..State::default()
        };
        let document = a_document(directory.path());
        let path = document.path().to_path_buf();
        state.recents.record(document);
        state.recents.place(
            &path,
            Some(Placement {
                lower_left_x: 48.0,
                lower_left_y: 179.0,
                pages: PageSet::only_page(3),
            }),
        );

        let written = serde_json::to_value(&state).expect("deberia serializarse");

        let global = &written["visible_signature"];
        assert!(
            global["page"].is_null() && global["lower_left_x"].is_null(),
            "la pagina y la posicion no son globales: son de cada documento"
        );
        assert_eq!(global["size"]["width"], 200.0);
        assert_eq!(global["reason"], "Conforme");
        let row = &written["recents"][0]["placement"];
        assert_eq!(row["pages"], serde_json::json!({ "only": [3] }));
        assert_eq!(row["lower_left_x"], 48.0);
        assert!(
            row["width"].is_null(),
            "el tamano no se repite en cada fila: dos sitios donde divergir"
        );
    }

    /// Reabrir el mismo contrato acuña un identificador opaco nuevo (ID-62),
    /// pero la fila es la misma ruta canónica y su recuadro sigue donde estaba.
    #[test]
    fn recording_a_document_again_keeps_where_its_box_had_fallen() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let mut state = State::default();
        let document = a_document(directory.path());
        let path = document.path().to_path_buf();
        state.recents.record(document);
        state.recents.place(
            &path,
            Some(Placement {
                lower_left_x: 10.0,
                lower_left_y: 20.0,
                pages: PageSet::only_page(2),
            }),
        );

        state.recents.record(a_document(directory.path()));

        assert_eq!(
            state.recents.entries()[0]
                .placement()
                .map(|box_| box_.pages.clone()),
            Some(PageSet::only_page(2))
        );
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
                vec![0x01],
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
        // `init_args` entra con el #99: es lo que distingue un perfil de
        // Firefox de otro, y sin ello un certificado de NSS recordado no sabria
        // a que almacen volver. Sigue siendo *donde* esta el certificado, no
        // quien es.
        assert_eq!(
            fields,
            vec!["cka_id", "init_args", "label", "module", "token_label"]
        );
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
                vec![0x01],
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
