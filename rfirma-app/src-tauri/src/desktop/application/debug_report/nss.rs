//! Las líneas del informe sobre los almacenes NSS.

use super::*;

pub(super) fn nss_lines(stores: &NssStores, owner: &ReportOwner) -> Vec<String> {
    let rfirma_store = if stores.rfirma_store_installed {
        "instalado"
    } else {
        "no instalado"
    };
    stores
        .profiles
        .iter()
        .flat_map(|profile| {
            [
                format!("{}: {}", profile.browser, profile_state(&profile.state)),
                format!(
                    "  perfil: {}",
                    anonymized_profile(&profile.directory, owner)
                ),
            ]
        })
        .chain(std::iter::once(format!(
            "Almacén de rFirma: {rfirma_store}"
        )))
        .collect()
}

fn profile_state(state: &NssProfileState) -> String {
    match state {
        NssProfileState::TrustsLocalChannel { until: Some(date) } => {
            format!("confía en el canal local · hasta {date}")
        }
        NssProfileState::TrustsLocalChannel { until: None } => {
            "confía en el canal local".to_owned()
        }
        NssProfileState::DoesNotTrustLocalChannel => "no confía en el canal local".to_owned(),
        NssProfileState::IgnoredWithoutCertificateDatabase => "ignorado · sin cert9.db".to_owned(),
    }
}

fn anonymized_profile(directory: &Path, owner: &ReportOwner) -> String {
    match directory.parent() {
        Some(parent) => format!("{}/<perfil>", owner.anonymized(parent)),
        None => "<perfil>".to_owned(),
    }
}
