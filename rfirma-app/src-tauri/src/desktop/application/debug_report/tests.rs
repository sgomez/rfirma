use std::path::PathBuf;

use super::*;

fn ana() -> ReportOwner {
    ReportOwner {
        home: PathBuf::from("/home/ana"),
        user_name: "ana".to_owned(),
    }
}

fn a_flatpak_report() -> DebugReport {
    DebugReport {
        version: "1.2.3".to_owned(),
        installation: Installation::Flatpak(FlatpakInstallation {
            remote: "rfirma".to_owned(),
            branch: "stable".to_owned(),
            runtime: "org.gnome.Platform/x86_64/48".to_owned(),
        }),
        glibc: None,
        webview: None,
        operating_system: "linux".to_owned(),
        architecture: "x86_64".to_owned(),
        linux: Some(LinuxEnvironment {
            distribution: "Ubuntu 24.04".to_owned(),
            desktop: "GNOME".to_owned(),
            session: "wayland".to_owned(),
        }),
        locale: "es_ES.UTF-8".to_owned(),
        protocol_handler: ProtocolHandlerStatus::NotQueryableFromTheSandbox,
        native_library: NativeLibrary {
            status: NativeLibraryStatus::Loaded,
            path: Some(PathBuf::from("/app/lib/rfirma/librfirma_crypto.so")),
        },
        library_directory_override: None,
        pkcs11_module_override: None,
        pcsc: Some(PcscStatus::Responding(Vec::new())),
        bundled_pcsc_lite: None,
        pkcs11_modules: None,
    }
}

#[test]
fn the_report_is_a_header_then_system_then_integration() {
    assert_eq!(
        debug_report_text(&a_flatpak_report(), &ana()),
        "rFirma 1.2.3\n\
         \n\
         Sistema\n  \
           Instalación: flatpak\n    \
             Remoto: rfirma\n    \
             Rama: stable\n    \
             Runtime: org.gnome.Platform/x86_64/48\n  \
           Distribución: Ubuntu 24.04\n  \
           Arquitectura: x86_64\n  \
           Escritorio: GNOME\n  \
           Sesión: wayland\n  \
           Idioma: es_ES.UTF-8\n\
         \n\
         Integración\n  \
           afirma://: no consultable · sandbox de flatpak\n  \
           Biblioteca nativa: cargada\n    \
             ruta: /app/lib/rfirma/librfirma_crypto.so\n\
         \n\
         Lectores\n  \
           pcscd: responde"
    );
}

#[test]
fn the_report_no_longer_says_it_is_compatible_with_autofirma() {
    let text = debug_report_text(&a_flatpak_report(), &ana());

    assert!(!text.contains("AutoFirma"), "{text}");
}

#[test]
fn outside_linux_the_system_is_named_instead_of_distribution_desktop_and_session() {
    let report = DebugReport {
        linux: None,
        installation: Installation::WindowsInstaller(InstallerScope::PerUser),
        operating_system: "windows".to_owned(),
        protocol_handler: ProtocolHandlerStatus::Registered("rfirma.desktop".to_owned()),
        ..a_flatpak_report()
    };

    let text = debug_report_text(&report, &ana());

    assert!(
        text.contains("  Instalación: instalador de Windows · por usuario\n  Sistema operativo: windows\n  Arquitectura:"),
        "{text}"
    );
    for absent in ["Distribución", "Escritorio", "Sesión"] {
        assert!(!text.contains(absent), "sobra {absent}");
    }
    assert!(text.contains("  afirma://: rfirma.desktop\n"), "{text}");
}

#[test]
fn no_handler_registered_is_said_plainly() {
    let text = debug_report_text(
        &DebugReport {
            protocol_handler: ProtocolHandlerStatus::NoneRegistered,
            ..a_flatpak_report()
        },
        &ana(),
    );

    assert!(text.contains("  afirma://: ninguno registrado\n"), "{text}");
}

#[test]
fn a_library_that_is_not_found_has_no_path_below_it() {
    let text = debug_report_text(
        &DebugReport {
            native_library: NativeLibrary {
                status: NativeLibraryStatus::NotFound,
                path: None,
            },
            ..a_flatpak_report()
        },
        &ana(),
    );

    assert!(
        text.contains("  Biblioteca nativa: no encontrada\n\nLectores"),
        "{text}"
    );
}

#[test]
fn a_library_that_does_not_load_shows_why_and_its_anonymized_path() {
    for (status, line) in [
        (
            NativeLibraryStatus::IncompatibleSymbols,
            "no carga · símbolos incompatibles",
        ),
        (NativeLibraryStatus::NotLoadable, "no carga"),
    ] {
        let text = debug_report_text(
            &DebugReport {
                native_library: NativeLibrary {
                    status,
                    path: Some(PathBuf::from("/home/ana/rfirma/lib/librfirma_crypto.so")),
                },
                ..a_flatpak_report()
            },
            &ana(),
        );

        assert!(
            text.contains(&format!(
                "  Biblioteca nativa: {line}\n    ruta: ~/rfirma/lib/librfirma_crypto.so\n\nLectores"
            )),
            "{text}"
        );
    }
}

#[test]
fn the_overriding_variables_appear_in_integration_when_defined() {
    let text = debug_report_text(
        &DebugReport {
            library_directory_override: Some(PathBuf::from("/home/ana/rfirma/target/lib")),
            pkcs11_module_override: Some(PathBuf::from("/usr/lib/softhsm/libsofthsm2.so")),
            ..a_flatpak_report()
        },
        &ana(),
    );

    assert!(
        text.contains(
            "  RFIRMA_LIB_DIR: ~/rfirma/target/lib\n  \
             RFIRMA_PKCS11_MODULE: /usr/lib/softhsm/libsofthsm2.so\n\nLectores"
        ),
        "{text}"
    );
}

#[test]
fn no_line_holds_the_home_or_the_user_name() {
    let text = debug_report_text(
        &DebugReport {
            native_library: NativeLibrary {
                status: NativeLibraryStatus::Loaded,
                path: Some(PathBuf::from("/run/user/1000/doc/ana/librfirma_crypto.so")),
            },
            library_directory_override: Some(PathBuf::from("/media/ana/usb/lib")),
            pkcs11_module_override: Some(PathBuf::from("/home/ana/.local/lib/module.so")),
            ..a_flatpak_report()
        },
        &ana(),
    );

    assert!(
        text.contains("    ruta: $XDG_RUNTIME_DIR/doc/<usuario>/librfirma_crypto.so\n"),
        "{text}"
    );
    assert!(
        text.contains("  RFIRMA_LIB_DIR: /media/<usuario>/usb/lib\n"),
        "{text}"
    );
    assert!(
        text.contains("  RFIRMA_PKCS11_MODULE: ~/.local/lib/module.so"),
        "{text}"
    );
    for line in text.lines() {
        assert!(!line.contains("/home/ana"), "{line}");
        assert!(!line.contains("ana"), "{line}");
    }
}

#[test]
fn a_directory_that_only_starts_like_the_home_is_not_taken_for_it() {
    let text = debug_report_text(
        &DebugReport {
            library_directory_override: Some(PathBuf::from("/home/anabel/lib")),
            ..a_flatpak_report()
        },
        &ana(),
    );

    assert!(
        text.contains("  RFIRMA_LIB_DIR: /home/<usuario>bel/lib"),
        "{text}"
    );
}

#[test]
fn system_paths_are_left_as_they_are() {
    let owner = ReportOwner {
        home: PathBuf::from("/home/lib"),
        user_name: "lib".to_owned(),
    };

    let text = debug_report_text(
        &DebugReport {
            pkcs11_module_override: Some(PathBuf::from("/usr/lib/opensc-pkcs11.so")),
            library_directory_override: Some(PathBuf::from("/etc/lib")),
            ..a_flatpak_report()
        },
        &owner,
    );

    assert!(
        text.contains("    ruta: /app/lib/rfirma/librfirma_crypto.so\n"),
        "{text}"
    );
    assert!(text.contains("  RFIRMA_LIB_DIR: /etc/lib\n"), "{text}");
    assert!(
        text.contains("  RFIRMA_PKCS11_MODULE: /usr/lib/opensc-pkcs11.so"),
        "{text}"
    );
}

fn installation_line(installation: Installation) -> String {
    let report = DebugReport {
        installation,
        ..a_flatpak_report()
    };
    let text = debug_report_text(&report, &ana());
    let from = text
        .find("  Instalación")
        .expect("hay línea de instalación");
    let rest = &text[from..];
    let end = rest.find("\n  D").expect("sigue la distribución");
    rest[..end].to_owned()
}

#[test]
fn a_deb_and_an_rpm_say_which_package_they_are() {
    assert_eq!(installation_line(Installation::Deb), "  Instalación: deb");
    assert_eq!(installation_line(Installation::Rpm), "  Instalación: rpm");
}

#[test]
fn a_development_build_says_the_commit_it_was_built_from() {
    assert_eq!(
        installation_line(Installation::Development {
            commit: "8d99e4a9".to_owned()
        }),
        "  Instalación: compilación de desarrollo\n    Commit: 8d99e4a9"
    );
}

#[test]
fn a_windows_installer_says_its_scope() {
    assert_eq!(
        installation_line(Installation::WindowsInstaller(InstallerScope::PerMachine)),
        "  Instalación: instalador de Windows · por equipo"
    );
}

#[test]
fn macos_says_so() {
    assert_eq!(
        installation_line(Installation::MacOs),
        "  Instalación: macOS"
    );
}

#[test]
fn glibc_and_the_webview_appear_only_when_known() {
    let native = DebugReport {
        installation: Installation::Deb,
        glibc: Some("2.42".to_owned()),
        webview: Some(WebView {
            name: "WebKitGTK".to_owned(),
            version: "2.50.1".to_owned(),
        }),
        ..a_flatpak_report()
    };

    let text = debug_report_text(&native, &ana());

    assert!(
        text.contains("  Arquitectura: x86_64\n  glibc: 2.42\n  Escritorio: GNOME\n"),
        "{text}"
    );
    assert!(
        text.contains("  Idioma: es_ES.UTF-8\n  WebKitGTK: 2.50.1\n\nIntegración"),
        "{text}"
    );
    let without = debug_report_text(&a_flatpak_report(), &ana());
    assert!(
        !without.contains("glibc") && !without.contains("WebKit"),
        "{without}"
    );
}

fn a_reader(name: &str, has_a_card: bool) -> Reader {
    Reader {
        name: name.to_owned(),
        has_a_card,
    }
}

fn readers_section(pcsc: Option<PcscStatus>) -> String {
    let text = debug_report_text(
        &DebugReport {
            pcsc,
            ..a_flatpak_report()
        },
        &ana(),
    );
    text.split("\n\n")
        .find(|block| block.starts_with("Lectores"))
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn a_pcscd_that_does_not_answer_lists_no_readers() {
    assert_eq!(
        readers_section(Some(PcscStatus::NotResponding)),
        "Lectores\n  pcscd: no responde"
    );
}

#[test]
fn a_pcscd_with_no_readers_says_only_that_it_answers() {
    assert_eq!(
        readers_section(Some(PcscStatus::Responding(Vec::new()))),
        "Lectores\n  pcscd: responde"
    );
}

#[test]
fn each_reader_says_whether_it_holds_a_card() {
    assert_eq!(
        readers_section(Some(PcscStatus::Responding(vec![
            a_reader("Alcor Micro AU9540 00 00", true),
            a_reader("Generic Smart Card Reader Interface 01 00", false),
        ]))),
        "Lectores\n  pcscd: responde\n  \
         Alcor Micro AU9540 00 00: con tarjeta\n  \
         Generic Smart Card Reader Interface 01 00: sin tarjeta"
    );
}

#[test]
fn a_platform_without_pcsc_has_no_readers_section() {
    assert_eq!(readers_section(None), "");
}

#[test]
fn the_bundled_pcsc_lite_version_closes_the_system_section() {
    let text = debug_report_text(
        &DebugReport {
            bundled_pcsc_lite: Some("2.5.1".to_owned()),
            ..a_flatpak_report()
        },
        &ana(),
    );

    assert!(
        text.contains("Idioma: es_ES.UTF-8\n  pcsc-lite: 2.5.1\n"),
        "{text}"
    );
}

#[test]
fn without_a_bundled_client_no_pcsc_lite_line_appears() {
    let text = debug_report_text(&a_flatpak_report(), &ana());

    assert!(!text.contains("pcsc-lite"), "{text}");
}

fn modules_section(report: DebugReport) -> String {
    debug_report_text(&report, &ana())
        .split("\n\n")
        .find(|block| block.starts_with("Módulos PKCS#11"))
        .unwrap_or_default()
        .to_owned()
}

fn a_native_report() -> DebugReport {
    DebugReport {
        installation: Installation::Deb,
        ..a_flatpak_report()
    }
}

fn module(
    name: &str,
    library: Option<&str>,
    registration: Option<&str>,
    discard: Option<DiscardReason>,
) -> Pkcs11Module {
    Pkcs11Module {
        name: name.to_owned(),
        library: library.map(PathBuf::from),
        registration: registration.map(PathBuf::from),
        discard,
    }
}

#[test]
fn a_usable_module_shows_its_library_and_the_file_that_registers_it() {
    assert_eq!(
        modules_section(DebugReport {
            pkcs11_modules: Some(Pkcs11Modules::Discovered(vec![module(
                "opensc",
                Some("/usr/lib/x86_64-linux-gnu/opensc-pkcs11.so"),
                Some("/home/ana/.config/pkcs11/modules/opensc.module"),
                None,
            )])),
            ..a_native_report()
        }),
        "Módulos PKCS#11\n  \
         opensc: encontrado\n    \
           módulo: /usr/lib/x86_64-linux-gnu/opensc-pkcs11.so\n    \
           alta: ~/.config/pkcs11/modules/opensc.module"
    );
}

#[test]
fn a_discarded_module_says_why() {
    let discarded = |reason| {
        modules_section(DebugReport {
            pkcs11_modules: Some(Pkcs11Modules::Discovered(vec![module(
                "bit4id",
                Some("/usr/lib/libbit4xpki.so"),
                Some("/etc/pkcs11/modules/bit4id.module"),
                Some(reason),
            )])),
            ..a_native_report()
        })
        .lines()
        .nth(1)
        .unwrap_or_default()
        .to_owned()
    };

    assert_eq!(
        discarded(DiscardReason::DisabledInRfirma),
        "  bit4id: descartado · disable-in"
    );
    assert_eq!(
        discarded(DiscardReason::EnabledOnlyElsewhere),
        "  bit4id: descartado · enable-in sin rfirma"
    );
    assert_eq!(
        discarded(DiscardReason::TrustPolicy),
        "  bit4id: descartado · trust-policy"
    );
    assert_eq!(
        discarded(DiscardReason::MissingModule),
        "  bit4id: descartado · falta el módulo"
    );
}

#[test]
fn a_discarded_module_without_library_shows_only_its_registration() {
    assert_eq!(
        modules_section(DebugReport {
            pkcs11_modules: Some(Pkcs11Modules::Discovered(vec![module(
                "p11-kit-trust",
                None,
                Some("/usr/share/p11-kit/modules/p11-kit-trust.module"),
                Some(DiscardReason::TrustPolicy),
            )])),
            ..a_native_report()
        }),
        "Módulos PKCS#11\n  \
         p11-kit-trust: descartado · trust-policy\n    \
           alta: /usr/share/p11-kit/modules/p11-kit-trust.module"
    );
}

#[test]
fn a_fixed_candidate_shows_no_registration() {
    assert_eq!(
        modules_section(DebugReport {
            pkcs11_modules: Some(Pkcs11Modules::Discovered(vec![module(
                "libsofthsm2",
                Some("/usr/lib/softhsm/libsofthsm2.so"),
                None,
                None,
            )])),
            ..a_native_report()
        }),
        "Módulos PKCS#11\n  \
         libsofthsm2: encontrado\n    \
           módulo: /usr/lib/softhsm/libsofthsm2.so"
    );
}

#[test]
fn in_the_flatpak_the_section_says_the_sandbox_does_not_see_the_host_modules() {
    assert_eq!(
        modules_section(DebugReport {
            pkcs11_modules: Some(Pkcs11Modules::Discovered(vec![module(
                "opensc",
                Some("/app/lib/pkcs11/opensc-pkcs11.so"),
                Some("/app/share/p11-kit/modules/opensc.module"),
                None,
            )])),
            ..a_flatpak_report()
        }),
        "Módulos PKCS#11\n  \
         Módulos del anfitrión: no visibles · el sandbox solo ve el OpenSC incluido\n  \
         opensc: encontrado\n    \
           módulo: /app/lib/pkcs11/opensc-pkcs11.so\n    \
           alta: /app/share/p11-kit/modules/opensc.module"
    );
}

#[test]
fn the_module_variable_says_it_overrides_the_discovery() {
    assert_eq!(
        modules_section(DebugReport {
            pkcs11_modules: Some(Pkcs11Modules::Overridden(PathBuf::from(
                "/home/ana/softhsm/libsofthsm2.so"
            ))),
            ..a_native_report()
        }),
        "Módulos PKCS#11\n  \
         Descubrimiento: anulado · RFIRMA_PKCS11_MODULE\n    \
           módulo: ~/softhsm/libsofthsm2.so"
    );
}

#[test]
fn without_module_discovery_there_is_no_section() {
    assert_eq!(modules_section(a_native_report()), "");
}
