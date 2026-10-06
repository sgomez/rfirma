use super::*;

fn a_flatpak_report() -> DebugReport {
    DebugReport {
        version: "1.2.3".to_owned(),
        channel: "flatpak".to_owned(),
        operating_system: "linux".to_owned(),
        architecture: "x86_64".to_owned(),
        linux: Some(LinuxEnvironment {
            distribution: "Ubuntu 24.04".to_owned(),
            desktop: "GNOME".to_owned(),
            session: "wayland".to_owned(),
        }),
        locale: "es_ES.UTF-8".to_owned(),
        protocol_handler: ProtocolHandlerStatus::NotQueryableFromTheSandbox,
        native_library: NativeLibraryStatus::Loaded,
    }
}

#[test]
fn the_report_is_one_key_and_value_per_line() {
    assert_eq!(
        debug_report_text(&a_flatpak_report()),
        "Versión de rFirma: 1.2.3\n\
         Compatible con AutoFirma: 1.9.2\n\
         Canal de instalación: flatpak\n\
         Sistema operativo: linux\n\
         Arquitectura: x86_64\n\
         Distribución: Ubuntu 24.04\n\
         Entorno de escritorio: GNOME\n\
         Tipo de sesión: wayland\n\
         Idioma del sistema: es_ES.UTF-8\n\
         Manejador de afirma://: no se puede consultar desde el sandbox de flatpak\n\
         Biblioteca nativa: cargada"
    );
}

#[test]
fn outside_linux_the_report_has_no_distribution_desktop_or_session() {
    let report = DebugReport {
        linux: None,
        channel: "windows".to_owned(),
        operating_system: "windows".to_owned(),
        protocol_handler: ProtocolHandlerStatus::Registered("rfirma.desktop".to_owned()),
        ..a_flatpak_report()
    };

    let text = debug_report_text(&report);

    for absent in ["Distribución", "escritorio", "sesión"] {
        assert!(!text.contains(absent), "sobra {absent}");
    }
    assert!(text.contains("Manejador de afirma://: rfirma.desktop"));
}

#[test]
fn a_missing_or_incompatible_library_is_said_without_any_path() {
    for (status, sentence) in [
        (NativeLibraryStatus::NotFound, "no encontrada"),
        (
            NativeLibraryStatus::IncompatibleSymbols,
            "símbolos incompatibles",
        ),
    ] {
        let text = debug_report_text(&DebugReport {
            native_library: status,
            ..a_flatpak_report()
        });

        assert!(text.ends_with(&format!("Biblioteca nativa: {sentence}")));
        assert!(!text.contains("/ "), "{text}");
        assert!(!text.contains("/usr") && !text.contains("/home"), "{text}");
    }
}

#[test]
fn no_handler_registered_is_said_plainly() {
    let text = debug_report_text(&DebugReport {
        protocol_handler: ProtocolHandlerStatus::NoneRegistered,
        ..a_flatpak_report()
    });

    assert!(text.contains("Manejador de afirma://: ninguno registrado"));
}
