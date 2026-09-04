//! El **estado**: lo que la aplicación acumula por su cuenta (ID-31,
//! ADR-0010).
//!
//! Las otras tres memorias: los documentos recientes, la última configuración
//! de firma visible y el certificado usado. Y, colgada de ellas, la última
//! carpeta de la que se abrió algo, que fuera del sandbox sí se puede saber.
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
    /// Es `None` **siempre bajo el sandbox**, y no por precaución: allí el
    /// diálogo devuelve un enlace del portal y la carpeta real no se puede
    /// saber. En los canales sin sandbox —deb, rpm, Windows, macOS— sí se
    /// sabe, y entonces se recuerda.
    ///
    /// Es la única ruta del anfitrión que se guarda además de las de los
    /// recientes, y es **estado y no configuración**: la acumula la aplicación
    /// sola, nadie la elige, y por eso «Recordar mi actividad» se la lleva
    /// como se lleva todo lo demás.
    pub last_open_folder: Option<PathBuf>,
    /// Cuándo se preguntó por última vez si hay una versión nueva, y qué se
    /// contestó (ID-180).
    ///
    /// **No es una memoria del usuario**: no dice qué firmó ni por dónde
    /// anduvo, dice cuándo habló rFirma con GitHub. Por eso es la **única
    /// exenta de los dos interruptores**, y [`State::forget_everything`] la
    /// conserva: borrarla no borraría nada de
    /// nadie y solo conseguiría una conexión de más en el siguiente arranque.
    pub version_check: Option<VersionCheck>,
}

/// La última vez que se preguntó por una versión nueva, y lo que se contestó.
///
/// Existe para **no preguntar más de una vez cada 24 h** (ID-180): sin este
/// apunte, cada arranque sería una conexión saliente. Guarda lo anunciado y no
/// si había versión nueva, porque «nueva» se decide contra la versión que se
/// está ejecutando y esa cambia con cada actualización.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionCheck {
    /// Cuándo se preguntó, en segundos desde el epoch.
    pub checked_at: u64,
    /// La versión que anunció GitHub, tal y como se leyó.
    pub announced: String,
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
///
/// `id_number` estuvo aquí hasta la v0.3.1 y se fue con la casilla «DNI». El
/// estado viejo **se ignora en silencio**, sin migración: es un booleano de
/// adorno del recuadro, y `serde(default)` sin `deny_unknown_fields` es lo que
/// hace que un fichero guardado por la versión anterior siga leyéndose entero
/// en vez de fallar por un campo de sobra.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RememberedFields {
    pub signer_name: bool,
    pub issuer: bool,
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
    /// La caché de la comprobación de versión **no** se va con el resto
    /// (ID-180): no es actividad de nadie, es un apunte de rFirma sobre sí
    /// misma.
    pub fn forget_everything(&mut self) {
        let version_check = self.version_check.take();
        *self = Self::default();
        self.version_check = version_check;
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

    /// La configuración guardada por la v0.3.0 trae la casilla «DNI», que ya
    /// no existe: se ignora **en silencio**, y el resto de lo guardado
    /// sobrevive. Sin esto, un campo de sobra tumbaba la deserialización
    /// entera y el usuario perdía la configuración del recuadro al actualizar.
    #[test]
    fn a_visible_signature_saved_by_an_older_version_survives_the_field_that_is_gone() {
        let saved = r#"{
            "enabled": true,
            "rubric": true,
            "fields": {
                "signer_name": true,
                "id_number": true,
                "signed_at": true,
                "reason": true
            },
            "reason": "Conforme",
            "size": { "width": 200.0, "height": 80.0 }
        }"#;

        let remembered: VisibleSignatureMemory =
            serde_json::from_str(saved).expect("el campo de sobra se ignora");

        assert!(remembered.enabled);
        assert!(remembered.rubric);
        assert_eq!(remembered.reason, "Conforme");
        assert_eq!(remembered.size.width, 200.0);
        assert_eq!(remembered.size.height, 80.0);
        assert_eq!(
            remembered.fields,
            RememberedFields {
                signer_name: true,
                // La casilla nueva no estaba en lo guardado: vale «sin marcar».
                issuer: false,
                signed_at: true,
                reason: true,
            }
        );
    }

    /// La caché de la comprobación de versión tampoco se va con la actividad:
    /// no dice qué hizo quien firmó antes, dice cuándo se habló con GitHub
    /// (ID-180).
    #[test]
    fn forgetting_everything_leaves_the_version_check_alone() {
        let mut state = State {
            last_open_folder: Some(Path::new("/home/quien/Documentos").to_path_buf()),
            version_check: Some(VersionCheck {
                checked_at: 1_756_000_000,
                announced: "0.4.0".to_string(),
            }),
            ..State::default()
        };

        state.forget_everything();

        assert!(state.last_open_folder.is_none());
        assert_eq!(
            state.version_check,
            Some(VersionCheck {
                checked_at: 1_756_000_000,
                announced: "0.4.0".to_string(),
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
                    issuer: true,
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
