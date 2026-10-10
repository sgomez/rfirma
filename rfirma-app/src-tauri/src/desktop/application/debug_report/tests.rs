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
        channel: "flatpak".to_owned(),
        operating_system: "linux".to_owned(),
        architecture: "x86_64".to_owned(),
        linux: Some(LinuxEnvironment {
            distribution: "Ubuntu 24.04".to_owned(),
            desktop: "GNOME".to_owned(),
            session: "wayland".to_owned(),
        }),
        graphics: Graphics::default(),
        locale: "es_ES.UTF-8".to_owned(),
        protocol_handler: ProtocolHandlerStatus::NotQueryableFromTheSandbox,
        native_library: NativeLibrary {
            status: NativeLibraryStatus::Loaded,
            path: Some(PathBuf::from("/app/lib/rfirma/librfirma_crypto.so")),
        },
        library_directory_override: None,
        pkcs11_module_override: None,
    }
}

#[test]
fn the_report_is_a_header_then_system_then_integration() {
    assert_eq!(
        debug_report_text(&a_flatpak_report(), &ana()),
        "rFirma 1.2.3\n\
         \n\
         Sistema\n  \
           Instalación: flatpak\n  \
           Distribución: Ubuntu 24.04\n  \
           Arquitectura: x86_64\n  \
           Escritorio: GNOME\n  \
           Sesión: wayland\n  \
           Idioma: es_ES.UTF-8\n\
         \n\
         Integración\n  \
           afirma://: no consultable · sandbox de flatpak\n  \
           Biblioteca nativa: cargada\n    \
             ruta: /app/lib/rfirma/librfirma_crypto.so"
    );
}

fn nvidia() -> Gpu {
    Gpu {
        vendor: "NVIDIA".to_owned(),
        driver: "nvidia".to_owned(),
        driver_version: Some("580.95".to_owned()),
    }
}

fn intel() -> Gpu {
    Gpu {
        vendor: "Intel".to_owned(),
        driver: "i915".to_owned(),
        driver_version: None,
    }
}

fn graphics_section(graphics: Graphics) -> String {
    let text = debug_report_text(
        &DebugReport {
            graphics,
            ..a_flatpak_report()
        },
        &ana(),
    );
    text.split("\n\n")
        .find(|block| block.starts_with("Gráficos"))
        .unwrap_or_default()
        .to_owned()
}

fn renderer(origin: RendererOrigin) -> RendererVariable {
    RendererVariable {
        name: "WEBKIT_DISABLE_DMABUF_RENDERER".to_owned(),
        value: "1".to_owned(),
        origin,
    }
}

#[test]
fn graphics_sit_between_system_and_integration() {
    let text = debug_report_text(
        &DebugReport {
            graphics: Graphics {
                gpus: vec![intel()],
                ..Graphics::default()
            },
            ..a_flatpak_report()
        },
        &ana(),
    );

    let at = |title: &str| text.find(title).unwrap_or(usize::MAX);
    assert!(
        at("Sistema") < at("Gráficos") && at("Gráficos") < at("Integración"),
        "{text}"
    );
}

#[test]
fn a_gpu_line_carries_vendor_driver_and_version_when_there_is_one() {
    assert_eq!(
        graphics_section(Graphics {
            gpus: vec![nvidia()],
            ..Graphics::default()
        }),
        "Gráficos\n  GPU: NVIDIA · nvidia 580.95"
    );
}

#[test]
fn a_hybrid_laptop_gets_one_gpu_line_per_card() {
    assert_eq!(
        graphics_section(Graphics {
            gpus: vec![nvidia(), intel()],
            ..Graphics::default()
        }),
        "Gráficos\n  GPU: NVIDIA · nvidia 580.95\n  GPU: Intel · i915"
    );
}

#[test]
fn a_renderer_variable_set_by_rfirma_says_so() {
    assert_eq!(
        graphics_section(Graphics {
            renderer: Some(renderer(RendererOrigin::SetByRfirma)),
            ..Graphics::default()
        }),
        "Gráficos\n  Renderizador: WEBKIT_DISABLE_DMABUF_RENDERER=1 · fijado por rFirma"
    );
}

#[test]
fn a_renderer_variable_from_the_environment_says_so() {
    assert_eq!(
        graphics_section(Graphics {
            renderer: Some(renderer(RendererOrigin::FromTheEnvironment)),
            ..Graphics::default()
        }),
        "Gráficos\n  Renderizador: WEBKIT_DISABLE_DMABUF_RENDERER=1 · del entorno"
    );
}

#[test]
fn gdk_backend_only_shows_when_defined() {
    let defined = graphics_section(Graphics {
        gpus: vec![intel()],
        display_backend: Some("x11".to_owned()),
        ..Graphics::default()
    });
    let undefined = graphics_section(Graphics {
        gpus: vec![intel()],
        ..Graphics::default()
    });

    assert!(defined.contains("  GDK_BACKEND: x11"), "{defined}");
    assert!(!undefined.contains("GDK_BACKEND"), "{undefined}");
}

#[test]
fn the_gl_extension_says_whether_it_is_installed_or_missing() {
    let extension = |installed| {
        graphics_section(Graphics {
            gl_extension: Some(GlExtension {
                driver_version: "580.95".to_owned(),
                installed,
            }),
            ..Graphics::default()
        })
    };

    assert_eq!(
        extension(true),
        "Gráficos\n  Extensión GL: instalada · 580.95"
    );
    assert_eq!(extension(false), "Gráficos\n  Extensión GL: falta · 580.95");
}

#[test]
fn a_report_without_graphics_has_no_graphics_section() {
    let text = debug_report_text(&a_flatpak_report(), &ana());

    assert!(!text.contains("Gráficos"), "{text}");
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
        channel: "windows".to_owned(),
        operating_system: "windows".to_owned(),
        protocol_handler: ProtocolHandlerStatus::Registered("rfirma.desktop".to_owned()),
        ..a_flatpak_report()
    };

    let text = debug_report_text(&report, &ana());

    assert!(
        text.contains("  Instalación: windows\n  Sistema operativo: windows\n  Arquitectura:"),
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
        text.ends_with("  Biblioteca nativa: no encontrada"),
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
            text.ends_with(&format!(
                "  Biblioteca nativa: {line}\n    ruta: ~/rfirma/lib/librfirma_crypto.so"
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
        text.ends_with(
            "  RFIRMA_LIB_DIR: ~/rfirma/target/lib\n  \
             RFIRMA_PKCS11_MODULE: /usr/lib/softhsm/libsofthsm2.so"
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
