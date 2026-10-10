//! Las líneas del informe sobre los módulos PKCS#11.

use super::*;

pub(super) fn module_lines(
    modules: &Pkcs11Modules,
    installation: &Installation,
    owner: &ReportOwner,
) -> Vec<String> {
    match modules {
        Pkcs11Modules::Overridden(path) => vec![
            "Descubrimiento: anulado · RFIRMA_PKCS11_MODULE".to_owned(),
            format!("  módulo: {}", owner.anonymized(path)),
        ],
        Pkcs11Modules::Discovered(found) => {
            let sandbox = matches!(installation, Installation::Flatpak(_)).then(|| {
                "Módulos del anfitrión: no visibles · el sandbox solo ve el OpenSC incluido"
                    .to_owned()
            });
            sandbox
                .into_iter()
                .chain(found.iter().flat_map(|module| module_block(module, owner)))
                .collect()
        }
    }
}

fn module_block(module: &Pkcs11Module, owner: &ReportOwner) -> Vec<String> {
    let state = match module.discard {
        None => "encontrado".to_owned(),
        Some(reason) => format!("descartado · {}", discard_reason_text(reason)),
    };
    std::iter::once(format!("{}: {state}", module.name))
        .chain(
            module
                .library
                .iter()
                .map(|path| format!("  módulo: {}", owner.anonymized(path))),
        )
        .chain(
            module
                .registration
                .iter()
                .map(|path| format!("  alta: {}", owner.anonymized(path))),
        )
        .collect()
}

fn discard_reason_text(reason: DiscardReason) -> &'static str {
    match reason {
        DiscardReason::DisabledInRfirma => "disable-in",
        DiscardReason::EnabledOnlyElsewhere => "enable-in sin rfirma",
        DiscardReason::TrustPolicy => "trust-policy",
        DiscardReason::MissingModule => "falta el módulo",
    }
}
