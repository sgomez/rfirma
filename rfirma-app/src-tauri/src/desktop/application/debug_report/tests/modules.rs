use std::path::PathBuf;

use super::super::*;
use super::{a_flatpak_report, ana};

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
