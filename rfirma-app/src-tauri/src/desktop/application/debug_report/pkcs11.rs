//! Los módulos PKCS#11 del informe y sus líneas.

use std::time::Duration;

use super::*;

/// Por qué rFirma no usa un módulo dado de alta.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscardReason {
    /// Su `disable-in` nombra a rFirma.
    DisabledInRfirma,
    /// Su `enable-in` no nombra a rFirma.
    EnabledOnlyElsewhere,
    /// Es un almacén de confianza.
    TrustPolicy,
    /// Su biblioteca no está instalada.
    MissingModule,
}

/// Qué ha pasado con un módulo: el resultado de su sondeo, o por qué no se usa.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModuleStatus {
    /// Carga, y su biblioteca declara fabricante y versión.
    Loads {
        /// El fabricante que declara la biblioteca.
        manufacturer: String,
        /// La versión de la biblioteca, `mayor.menor`.
        version: String,
    },
    /// No se carga o no se inicializa.
    DoesNotLoad,
    /// No ha terminado dentro del límite que se le dio.
    NotResponding(Duration),
    /// No se usa, por este motivo.
    Discarded(DiscardReason),
}

/// Un módulo PKCS#11 del descubrimiento.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pkcs11Module {
    /// Nombre del `.module`, o de la biblioteca si es un candidato fijo.
    pub name: String,
    /// La biblioteca, si se conoce.
    pub library: Option<PathBuf>,
    /// El `.module` que lo da de alta, si lo hay.
    pub registration: Option<PathBuf>,
    /// Su sondeo, o por qué se descarta.
    pub status: ModuleStatus,
}

/// De dónde salen los módulos PKCS#11 que usa la aplicación.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pkcs11Modules {
    /// `RFIRMA_PKCS11_MODULE` anula el descubrimiento y es el único módulo.
    Overridden(Pkcs11Module),
    /// Lo que el descubrimiento usa y descarta.
    Discovered(Vec<Pkcs11Module>),
}

pub(super) fn module_lines(
    modules: &Pkcs11Modules,
    installation: &Installation,
    owner: &ReportOwner,
) -> Vec<String> {
    match modules {
        Pkcs11Modules::Overridden(module) => {
            std::iter::once("Descubrimiento: anulado · RFIRMA_PKCS11_MODULE".to_owned())
                .chain(module_block(module, owner))
                .collect()
        }
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
    let state = match &module.status {
        ModuleStatus::Loads {
            manufacturer,
            version,
        } => format!("carga · {manufacturer} {version}"),
        ModuleStatus::DoesNotLoad => "no carga".to_owned(),
        ModuleStatus::NotResponding(limit) => format!("no responde · {} s", limit.as_secs()),
        ModuleStatus::Discarded(reason) => format!("descartado · {}", discard_reason_text(*reason)),
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
