//! Lo que rFirma recuerda entre sesiones: **seis** memorias, partidas en dos
//! (ADR-0010 y su enmienda, #53) — más [`OpenedDocuments`], que no sobrevive al
//! proceso y por eso no cuenta entre ellas.
//!
//! **El tamaño de la ventana no es una de ellas**: rFirma abre siempre al
//! tamaño de `tauri.conf.json` y quien quiera otra cosa maximiza o arrastra un
//! borde, que es lo que hace todo el mundo (ADR-0010, enmienda).
//!
//! Dentro del estado viaja además un apunte que **no es una memoria del
//! usuario** y por eso no cuenta entre las seis: [`VersionCheck`], cuándo se
//! preguntó por última vez a GitHub si hay una versión nueva y qué contestó
//! (ID-180, `app::version`). Es una caché para no salir a la red en cada
//! arranque, no dice nada de quien firma, y por eso es lo único exento de los
//! dos interruptores.
//!
//! **Configuración** —lo que el usuario elige y la aplicación obedece— son el
//! idioma, el tema, la carpeta de destino, los dos interruptores y la rúbrica.
//! **Estado** —lo que la aplicación acumula sola— son los documentos
//! recientes, la última configuración de firma visible y el certificado usado.
//! La que se olvida siempre al contarlas es la quinta, el certificado.
//!
//! La partición **no es estética**: en Windows el estado no debe viajar en un
//! perfil móvil y la configuración sí, y en Linux `XDG_STATE_HOME` existe justo
//! para eso. Borrar el estado no reconfigura la aplicación; borrar la
//! configuración no pierde el trabajo. Dónde cae cada fichero lo resuelve
//! [`crate::paths`], que es el único sitio del código que sabe qué sistema
//! operativo hay debajo.
//!
//! La rúbrica no es un campo de ninguna de las dos estructuras: es una imagen y
//! se guarda **como copia** en [`crate::rubric::RubricStore`], sobre la ruta que
//! da [`crate::paths::Paths::rubric_path`]. Guardar la ruta del original —lo
//! que hace AutoFirma— la pierde en silencio en cuanto el usuario mueve el PNG.
//!
//! [`Memory`] es la única puerta a los dos ficheros, y existe por una razón
//! concreta: **los dos interruptores no se pueden olvidar**. Guardar el estado
//! con «Recordar mi actividad» apagado no escribe nada, y guardar la
//! configuración con ese interruptor recién apagado **borra** el estado que
//! hubiera. Si eso viviera en quien llama, el día que alguien añada una llamada
//! nueva el rótulo pasaría a mentir.

pub mod configuration;
pub mod error;
pub mod handles;
pub mod listed;
pub mod opened;
pub mod recents;
pub mod state;
pub mod store;

pub use configuration::{Configuration, Theme};
pub use error::{MemoryError, Situation};
pub use listed::ListedCertificates;
pub use opened::{OpenedDocuments, Remembrance};
pub use recents::{Badge, Placement, RecentDocument, Recents, ShownBadge, CAPACITY};
pub use state::{BoxSize, RememberedFields, State, VersionCheck, VisibleSignatureMemory};
pub use store::{Damage, JsonFile, Loaded, Recovery, FORMAT_VERSION};

use crate::paths::Paths;

/// Las dos memorias y sus dos soportes.
#[derive(Clone, Debug, PartialEq)]
pub struct Memory {
    configuration: JsonFile<Configuration>,
    state: JsonFile<State>,
}

impl Memory {
    /// La memoria que vive en las rutas dadas.
    pub fn at(paths: &Paths) -> Self {
        Self {
            configuration: JsonFile::at(paths.config_file()),
            state: JsonFile::at(paths.state_file()),
        }
    }

    /// El soporte de la configuración.
    pub fn configuration_file(&self) -> &JsonFile<Configuration> {
        &self.configuration
    }

    /// El soporte del estado.
    pub fn state_file(&self) -> &JsonFile<State> {
        &self.state
    }

    /// La configuración guardada, o la de por omisión.
    pub fn configuration(&self) -> Result<Loaded<Configuration>, MemoryError> {
        self.configuration.load()
    }

    /// El estado guardado, o el vacío.
    ///
    /// No hace falta filtrar por el interruptor al leer: con «Recordar mi
    /// actividad» apagado el fichero sobrevive, pero solo con lo exento dentro
    /// —el tamaño de la ventana (ID-73) y la caché de la comprobación de
    /// versión (ID-180)—, así que no hay nada más que ocultar.
    pub fn state(&self) -> Result<Loaded<State>, MemoryError> {
        self.state.load()
    }

    /// Guarda la configuración y, si «Recordar mi actividad» queda apagado,
    /// **borra el estado**.
    ///
    /// Conservar el fichero mientras la preferencia dice que no se recuerda
    /// nada incumple lo que promete el rótulo (ADR-0010). Por eso el borrado
    /// está aquí y no en quien llama.
    pub fn remember_configuration(&self, configuration: &Configuration) -> Result<(), MemoryError> {
        self.configuration.save(configuration)?;
        if !configuration.remember_activity {
            self.erase_activity_but_keep_the_exempt()?;
        }
        Ok(())
    }

    /// Guarda el estado **según lo que permitan los dos interruptores**.
    ///
    /// Con «Recordar mi actividad» apagado no se escribe nada y se borra lo que
    /// hubiera. Con «Recordar la última configuración de firma visible»
    /// apagado se escribe el resto, pero el recuadro no: apagado significa no
    /// guardarlo, y no guardarlo y no aplicarlo.
    pub fn remember_state(
        &self,
        configuration: &Configuration,
        state: &State,
    ) -> Result<(), MemoryError> {
        if !configuration.remember_activity {
            return self.erase_activity_but_keep_the_exempt();
        }
        if configuration.remember_visible_signature {
            return self.state.save(state);
        }
        // Apagado se lleva **las dos mitades** de la firma visible (ID-74): lo
        // global y también dónde cayó el recuadro en cada fila, que vive en los
        // recientes. Guardar solo una dejaría el recuadro reponiéndose con el
        // interruptor apagado.
        let mut without_the_box = state.clone();
        without_the_box.visible_signature = None;
        without_the_box.recents.forget_placements();
        self.state.save(&without_the_box)
    }

    /// Olvida lo acumulado sin tocar ningún interruptor.
    ///
    /// No es «Vaciar la lista» —eso son solo los recientes,
    /// [`Recents::clear`]—: esto se lleva también el certificado.
    pub fn forget_activity(&self) -> Result<(), MemoryError> {
        self.erase_activity_but_keep_the_exempt()
    }

    /// Cuándo se preguntó por una versión nueva y qué se contestó, sin mirar
    /// ningún interruptor (ID-180).
    ///
    /// Escribe siempre porque no es actividad de quien firma, es un apunte de
    /// rFirma sobre sí misma. Con «Recordar mi actividad» apagado el
    /// `state.json` sobrevive con esto dentro, y nada más.
    pub fn remember_version_check(&self, check: VersionCheck) -> Result<(), MemoryError> {
        let mut state = self.state.load()?.into_value();
        state.version_check = Some(check);
        self.state.save(&state)
    }

    /// Borra lo acumulado —recientes, certificado, firma visible, última
    /// carpeta— conservando lo que ningún interruptor cubre: la caché de la
    /// comprobación de versión (ID-180).
    ///
    /// Qué se conserva lo dice [`State::forget_everything`] y no esta función:
    /// así el día que haya una exención más no hay dos sitios que ponerse de
    /// acuerdo. Si no quedaba nada exento esto es exactamente el borrado de
    /// antes: el fichero desaparece del disco.
    ///
    /// La lectura previa no puede vetar el borrado: en el camino de borrado
    /// por privacidad, un `state.json` que `load` no consiga leer se trata
    /// como si no llevara nada exento dentro —se pierde ese apunte, no el
    /// borrado— en vez de dejar la actividad en el disco con un `Err`.
    fn erase_activity_but_keep_the_exempt(&self) -> Result<(), MemoryError> {
        let mut kept = self
            .state
            .load()
            .map(Loaded::into_value)
            .unwrap_or_default();
        kept.forget_everything();
        self.state.erase()?;
        if kept.is_empty() {
            return Ok(());
        }
        self.state.save(&kept)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pkcs11::CertificateRef;
    use crate::signing::Language;
    use std::fs;
    use std::path::Path;
    use std::time::SystemTime;

    /// **Grada A**: dos ficheros en un directorio temporal. Sin token, sin
    /// librería nativa y sin red.
    fn a_memory(root: &Path) -> (Memory, Paths) {
        let paths = Paths::under(root);
        (Memory::at(&paths), paths)
    }

    fn a_state(directory: &Path) -> State {
        let document = directory.join("contrato.pdf");
        fs::write(&document, b"%PDF-1.7 de prueba").expect("deberia escribirse");
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
        state.recents.record(
            RecentDocument::seen(&document, Badge::Unsigned, SystemTime::now())
                .expect("deberia anotarse"),
        );
        state.recents.place(
            &fs::canonicalize(&document).expect("deberia canonicalizarse"),
            Some(Placement {
                lower_left_x: 48.0,
                lower_left_y: 179.0,
                pages: crate::signing::PageSet::only_page(1),
            }),
        );
        state
    }

    #[test]
    fn the_two_memories_live_in_different_supports() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let (memory, paths) = a_memory(directory.path());

        memory
            .remember_configuration(&Configuration::default())
            .expect("deberia guardarse la configuracion");
        memory
            .remember_state(&Configuration::default(), &a_state(directory.path()))
            .expect("deberia guardarse el estado");

        assert!(paths.config_file().exists());
        assert!(paths.state_file().exists());
        assert_ne!(
            paths.config_file().parent(),
            paths.state_file().parent(),
            "borrar el estado no puede reconfigurar la aplicacion"
        );
    }

    #[test]
    fn what_the_user_chose_comes_back_in_the_next_session() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let (memory, _) = a_memory(directory.path());
        let configuration = Configuration {
            language: Language::Catalan,
            destination: Some(crate::destination::DestinationFolder::at(
                "/home/quien/Documentos/Firmados",
            )),
            ..Configuration::default()
        };

        memory
            .remember_configuration(&configuration)
            .expect("deberia guardarse");

        assert_eq!(
            memory.configuration().expect("deberia leerse").into_value(),
            configuration
        );
    }

    #[test]
    fn what_the_application_accumulated_comes_back_too() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let (memory, _) = a_memory(directory.path());
        let state = a_state(directory.path());

        memory
            .remember_state(&Configuration::default(), &state)
            .expect("deberia guardarse");

        assert_eq!(memory.state().expect("deberia leerse").into_value(), state);
    }

    #[test]
    fn turning_remember_activity_off_erases_what_was_already_remembered() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let (memory, paths) = a_memory(directory.path());
        memory
            .remember_state(&Configuration::default(), &a_state(directory.path()))
            .expect("deberia guardarse");
        assert!(paths.state_file().exists());

        memory
            .remember_configuration(&Configuration {
                remember_activity: false,
                ..Configuration::default()
            })
            .expect("deberia guardarse la configuracion");

        // Se comprueba leyendo el soporte, no la estructura en memoria: lo que
        // promete el rótulo es que en el disco no queda nada.
        assert!(
            !paths.state_file().exists(),
            "apagar el interruptor tiene que borrar el fichero de estado"
        );
        assert!(memory
            .state()
            .expect("deberia leerse")
            .into_value()
            .is_empty());
        assert!(
            paths.config_file().exists(),
            "la configuracion no se va con el estado"
        );
    }

    /// La invariante que promete el `///` de `remember_version_check`
    /// —«escribe siempre, sin mirar interruptores»— comprobada desde donde se
    /// cumple.
    #[test]
    fn the_version_check_is_remembered_even_with_the_switch_off() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let (memory, _) = a_memory(directory.path());
        memory
            .remember_configuration(&Configuration {
                remember_activity: false,
                ..Configuration::default()
            })
            .expect("deberia guardarse la configuracion");

        let check = VersionCheck {
            checked_at: 1_757_000_000,
            announced: "v0.5.0".to_owned(),
        };
        memory
            .remember_version_check(check.clone())
            .expect("deberia guardarse");

        let state = memory.state().expect("deberia leerse").into_value();
        assert_eq!(state.version_check, Some(check));
    }

    /// Apagar «Recordar mi actividad» no borra el apunte de la consulta: no
    /// dice qué firmó nadie, dice cuándo habló rFirma con GitHub, y borrarlo
    /// sólo conseguiría una conexión de más.
    #[test]
    fn turning_remember_activity_off_does_not_erase_the_version_check() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let (memory, paths) = a_memory(directory.path());
        let check = VersionCheck {
            checked_at: 1_757_000_000,
            announced: "v0.5.0".to_owned(),
        };
        memory
            .remember_state(&Configuration::default(), &a_state(directory.path()))
            .expect("deberia guardarse");
        memory
            .remember_version_check(check.clone())
            .expect("deberia guardarse");

        memory
            .remember_configuration(&Configuration {
                remember_activity: false,
                ..Configuration::default()
            })
            .expect("deberia guardarse la configuracion");

        assert!(
            paths.state_file().exists(),
            "el apunte de la consulta no se ha ido, así que el fichero sigue"
        );
        let state = memory.state().expect("deberia leerse").into_value();
        assert_eq!(state.version_check, Some(check));
        assert!(
            state.certificate.is_none(),
            "el resto de la actividad sí se ha ido"
        );
    }

    #[test]
    fn with_remember_activity_off_nothing_is_written_even_if_someone_asks() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let (memory, paths) = a_memory(directory.path());
        let configuration = Configuration {
            remember_activity: false,
            ..Configuration::default()
        };

        memory
            .remember_state(&configuration, &a_state(directory.path()))
            .expect("no guardar no es un fallo");

        assert!(!paths.state_file().exists());
    }

    #[test]
    fn with_the_visible_signature_switch_off_the_box_is_not_saved_but_the_rest_is() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let (memory, _) = a_memory(directory.path());
        let configuration = Configuration {
            remember_visible_signature: false,
            ..Configuration::default()
        };

        memory
            .remember_state(&configuration, &a_state(directory.path()))
            .expect("deberia guardarse");

        let stored = memory.state().expect("deberia leerse").into_value();
        assert!(stored.visible_signature.is_none());
        assert_eq!(stored.recents.len(), 1);
        assert!(stored.certificate.is_some());
        // La otra mitad del ID-74: la fila sigue —es actividad— pero sin dónde
        // cayó su recuadro, que es firma visible.
        assert!(
            stored.recents.entries()[0].placement().is_none(),
            "apagado no guarda tampoco la posicion de cada documento"
        );
    }

    #[test]
    fn emptying_the_list_is_not_the_same_as_turning_the_switch_off() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let (memory, paths) = a_memory(directory.path());
        let mut state = a_state(directory.path());
        state.recents.clear();

        memory
            .remember_state(&Configuration::default(), &state)
            .expect("deberia guardarse");

        assert!(
            paths.state_file().exists(),
            "el soporte sigue: hoy no, manana si"
        );
        let stored = memory.state().expect("deberia leerse").into_value();
        assert!(stored.recents.is_empty());
        assert!(stored.certificate.is_some());
    }

    #[test]
    fn forgetting_the_activity_leaves_the_configuration_alone() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let (memory, paths) = a_memory(directory.path());
        memory
            .remember_configuration(&Configuration::default())
            .expect("deberia guardarse");
        memory
            .remember_state(&Configuration::default(), &a_state(directory.path()))
            .expect("deberia guardarse");

        memory.forget_activity().expect("deberia olvidarse");

        assert!(!paths.state_file().exists());
        assert!(paths.config_file().exists());
    }

    #[test]
    fn a_corrupt_configuration_does_not_stop_the_application_from_starting() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let (memory, paths) = a_memory(directory.path());
        fs::create_dir_all(paths.config_file().parent().expect("deberia tener padre"))
            .expect("deberia crearse");
        fs::write(paths.config_file(), b"{ roto").expect("deberia escribirse");

        let loaded = memory.configuration().expect("no puede impedir firmar");

        assert_eq!(loaded.value(), &Configuration::default());
        assert!(loaded.recovery().is_some(), "se avisa una vez");
        assert!(directory
            .path()
            .join("config/rfirma/config.json.bak")
            .exists());
    }

    /// La quinta memoria del grupo de configuración es la rúbrica, y no es un
    /// campo: es la **copia** que guarda el almacén sobre
    /// [`Paths::rubric_path`]. Lo que se comprueba es justo lo que AutoFirma
    /// hace mal: que en la configuración no queda ninguna ruta al original.
    #[test]
    fn the_rubric_is_persisted_as_a_copy_and_never_as_a_path_to_the_original() {
        use crate::rubric::RubricStore;
        use image::{ImageFormat, Rgba, RgbaImage};
        use std::io::Cursor;

        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let (memory, paths) = a_memory(directory.path());
        let original = directory.path().join("firma-escaneada.png");
        let mut png = RgbaImage::new(10, 10);
        for pixel in png.pixels_mut() {
            *pixel = Rgba([30, 60, 90, 255]);
        }
        let mut bytes = Vec::new();
        png.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .expect("el PNG de prueba deberia codificarse");
        fs::write(&original, bytes).expect("deberia escribirse");

        RubricStore::at(paths.rubric_path())
            .adopt(&original)
            .expect("deberia adoptarse");
        memory
            .remember_configuration(&Configuration::default())
            .expect("deberia guardarse");
        fs::remove_file(&original).expect("el original deberia poder borrarse");

        assert!(
            paths.rubric_path().exists(),
            "la rubrica sobrevive al original"
        );
        let written = fs::read_to_string(paths.config_file()).expect("deberia leerse");
        assert!(
            !written.contains("firma-escaneada"),
            "la configuracion no puede guardar la ruta del original"
        );
    }

    #[test]
    fn a_first_run_remembers_nothing_and_complains_about_nothing() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let (memory, _) = a_memory(directory.path());

        let configuration = memory.configuration().expect("deberia leerse");
        let state = memory.state().expect("deberia leerse");

        assert!(configuration.recovery().is_none());
        assert!(state.recovery().is_none());
        assert!(state.into_value().is_empty());
    }
}
