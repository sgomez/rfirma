//! El registro de arneses: lo que una comprobación necesita alrededor del trámite para medirse
//! —puertos ocupados, ficheros preparados—, no cómo se juzga.

use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{sleep, spawn, JoinHandle};
use std::time::Duration;

use crate::catalogue::{Check, Provocation};
use crate::checks::the_isolated_home_of;
use crate::errand::{ErrandOutcome, ProtocolConditionResult};
use crate::judge::decoded;
use crate::outcome::Outcome;
use crate::Probe;

type Measure = fn(&Probe, &Check, &Provocation) -> ErrandOutcome;

/// Un arnés que el catálogo liga por nombre.
#[derive(Debug)]
pub(crate) struct Harness {
    pub name: &'static str,
    pub fixtures: &'static [(&'static str, &'static str)],
    /// Los permisos con los que quedan sus ficheros, para provocar un fallo de disco.
    pub mode: u32,
    measure: Measure,
}

impl Harness {
    /// Conduce el trámite de la comprobación con lo que el arnés monte alrededor.
    pub(crate) fn measure(
        &self,
        probe: &Probe,
        check: &Check,
        drive: &Provocation,
    ) -> ErrandOutcome {
        (self.measure)(probe, check, drive)
    }
}

/// El arnés con ese nombre; `None` si el registro no lo tiene.
pub(crate) fn the_harness_named(name: &str) -> Option<&'static Harness> {
    THE_HARNESSES.iter().find(|harness| harness.name == name)
}

const JUST_DRIVE: Measure = |probe, check, drive| probe.drive(check, drive);

const READ_AND_WRITE: u32 = 0o644;

pub(crate) const THE_HARNESSES: &[Harness] = &[
    Harness {
        name: "a_document_to_sign",
        fixtures: &[("documento.txt", "Documento para firmar.\n")],
        mode: READ_AND_WRITE,
        measure: JUST_DRIVE,
    },
    Harness {
        name: "a_file_to_overwrite",
        fixtures: &[("challenge.bin", "Este fichero se sobrescribe.\n")],
        mode: READ_AND_WRITE,
        measure: JUST_DRIVE,
    },
    Harness {
        name: "a_file_that_cannot_be_written",
        fixtures: &[("challenge.bin", "Este fichero es de solo lectura.\n")],
        mode: 0o444,
        measure: JUST_DRIVE,
    },
    Harness {
        name: "a_file_that_cannot_be_read",
        fixtures: &[("ilegible.bin", "Este fichero no se puede leer.\n")],
        mode: 0o000,
        measure: JUST_DRIVE,
    },
    Harness {
        name: "files_to_load",
        fixtures: &[
            ("primero.bin", "Primer fichero de carga.\n"),
            ("segundo.bin", "Segundo fichero de carga.\n"),
        ],
        mode: READ_AND_WRITE,
        measure: JUST_DRIVE,
    },
    Harness {
        name: "the_saved_file_read_back",
        fixtures: &[],
        mode: READ_AND_WRITE,
        measure: |probe, check, drive| {
            let mut outcome = probe.drive(check, drive);
            let saved = the_isolated_home_of(&probe.client).join(THE_SAVED_NAME);
            outcome.protocol_conditions.push(the_saved_bytes_against(
                std::fs::read(saved).ok().as_deref(),
                &the_data_the_site_saves(),
            ));
            outcome
        },
    },
    Harness {
        name: "the_saved_signature_read_back",
        fixtures: &[],
        mode: READ_AND_WRITE,
        measure: |probe, check, drive| {
            let home = the_isolated_home_of(&probe.client);
            let (declared, name) = the_destination_proposed_by(drive);
            let saved = declared
                .map_or_else(|| home.to_path_buf(), PathBuf::from)
                .join(name);
            let astray = declared.map(|_| home.join(name));
            for path in std::iter::once(&saved).chain(astray.as_ref()) {
                let _ = std::fs::remove_file(path);
            }
            let mut outcome = probe.drive(check, drive);
            let returned = outcome.signature.as_deref().and_then(decoded);
            let found = std::fs::read(&saved).ok();
            outcome.protocol_conditions.push(
                match astray.filter(|astray| found.is_none() && astray.exists()) {
                    Some(_) => a_signature_saved_astray(),
                    None => the_saved_signature_against(found.as_deref(), returned.as_deref()),
                },
            );
            outcome
        },
    },
    Harness {
        name: "a_file_with_a_non_ascii_name",
        fixtures: &[("señal-año.bin", "Fichero con un nombre que no es ASCII.\n")],
        mode: READ_AND_WRITE,
        measure: JUST_DRIVE,
    },
    Harness {
        name: "occupied_service_ports",
        fixtures: &[],
        mode: READ_AND_WRITE,
        measure: |probe, check, drive| {
            let _occupied = OccupiedPorts::at(&drive.ports);
            probe.drive(check, drive)
        },
    },
];

/// El nombre que propone el guion `savereadback`, que ya borra la preparación de cada comprobación.
const THE_SAVED_NAME: &str = "challenge.bin";

const THE_DECODED_BYTES_ON_DISK: &str = "the-decoded-bytes-on-disk";

/// Lo que el guion de guardar manda en `dat`, ya decodificado: el reto del banco de referencia.
fn the_data_the_site_saves() -> Vec<u8> {
    let challenge = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../testdata/reference/challenge.bin");
    std::fs::read(&challenge)
        .unwrap_or_else(|error| panic!("falta {}: {error}", challenge.display()))
}

fn the_saved_bytes_against(saved: Option<&[u8]>, data: &[u8]) -> ProtocolConditionResult {
    let (outcome, observation) = match saved {
        None => (
            Outcome::NotObservable,
            format!("no hay {THE_SAVED_NAME} en la carpeta del diálogo: no se guardó ahí"),
        ),
        Some(saved) if saved == data => (
            Outcome::Compliant,
            format!(
                "{THE_SAVED_NAME} tiene los {} bytes decodificados de dat",
                data.len()
            ),
        ),
        Some(saved) => (
            Outcome::Noncompliant,
            format!(
                "{THE_SAVED_NAME} tiene {} bytes que no son los {} decodificados de dat",
                saved.len(),
                data.len()
            ),
        ),
    };
    ProtocolConditionResult {
        name: THE_DECODED_BYTES_ON_DISK.to_owned(),
        outcome,
        observation: Some(observation),
    }
}

/// Cada guion de firmar y guardar cuya firma se relee, con la carpeta que declara en
/// `filenameSaveCurrentDir`, si declara una, y el nombre que propone en `filename`.
const THE_SAVED_SIGNATURES: &[(&str, Option<&str>, &str)] = &[
    ("signandsave", None, "challenge-signed.csig"),
    (
        "signandsavewithsavingparameters",
        Some("/tmp"),
        "challenge-signed.csig",
    ),
    ("signandsavecadestri", None, "challenge-signed.csig"),
];

const THE_RETURNED_SIGNATURE_ON_DISK: &str = "the-returned-signature-on-disk";

fn the_destination_proposed_by(drive: &Provocation) -> (Option<&'static str>, &'static str) {
    THE_SAVED_SIGNATURES
        .iter()
        .find(|(script, _, _)| *script == drive.script)
        .map(|(_, directory, name)| (*directory, *name))
        .unwrap_or_else(|| panic!("el guion {} no propone un nombre conocido", drive.script))
}

fn a_signature_saved_astray() -> ProtocolConditionResult {
    ProtocolConditionResult {
        name: THE_RETURNED_SIGNATURE_ON_DISK.to_owned(),
        outcome: Outcome::Noncompliant,
        observation: Some(
            "la firma se guardó en la carpeta del perfil y no en la que declara la petición"
                .to_owned(),
        ),
    }
}

fn the_saved_signature_against(
    saved: Option<&[u8]>,
    returned: Option<&[u8]>,
) -> ProtocolConditionResult {
    let (outcome, observation) = match (saved, returned) {
        (None, _) => (
            Outcome::NotObservable,
            "no hay firma guardada con el nombre propuesto en la carpeta del diálogo".to_owned(),
        ),
        (Some(_), None) => (
            Outcome::NotObservable,
            "se guardó un fichero, pero la sede no recibió firma con la que compararlo".to_owned(),
        ),
        (Some(saved), Some(returned)) if saved == returned => (
            Outcome::Compliant,
            format!(
                "el fichero guardado tiene los {} bytes de la firma que recibió la sede",
                saved.len()
            ),
        ),
        (Some(saved), Some(returned)) => (
            Outcome::Noncompliant,
            format!(
                "el fichero guardado tiene {} bytes que no son los {} de la firma que recibió la sede",
                saved.len(),
                returned.len()
            ),
        ),
    };
    ProtocolConditionResult {
        name: THE_RETURNED_SIGNATURE_ON_DISK.to_owned(),
        outcome,
        observation: Some(observation),
    }
}

/// Cada cuánto mira el ocupante si le han dicho que suelte el puerto.
const THE_OCCUPIER_HEARTBEAT: Duration = Duration::from_millis(50);

/// Los puertos que la comprobación del socket ocupa para que el cliente no pueda ligarlos, cerrando
/// cada conexión que les llegue: un ocupante mudo colgaría al cliente publicado en el primer eco, y
/// entonces no se rinde nunca y no hay nada que medir.
struct OccupiedPorts {
    release: Arc<AtomicBool>,
    occupiers: Vec<JoinHandle<()>>,
}

impl OccupiedPorts {
    fn at(ports: &[u16]) -> Self {
        let release = Arc::new(AtomicBool::new(false));
        let occupiers = ports
            .iter()
            .map(|port| {
                let listener = TcpListener::bind(("0.0.0.0", *port))
                    .unwrap_or_else(|error| panic!("no pude ocupar el puerto {port}: {error}"));
                listener
                    .set_nonblocking(true)
                    .expect("el ocupante debería poder no bloquearse");
                let release = Arc::clone(&release);
                spawn(move || {
                    while !release.load(Ordering::Relaxed) {
                        match listener.accept() {
                            Ok(_) => continue,
                            Err(_) => sleep(THE_OCCUPIER_HEARTBEAT),
                        }
                    }
                })
            })
            .collect();
        Self { release, occupiers }
    }
}

impl Drop for OccupiedPorts {
    fn drop(&mut self) {
        self.release.store(true, Ordering::Relaxed);
        for occupier in self.occupiers.drain(..) {
            let _ = occupier.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::catalogue::read_the_catalogue;

    #[test]
    fn every_harness_in_the_registry_has_an_entry_in_the_catalogue() {
        let named: BTreeSet<&str> = read_the_catalogue()
            .unwrap()
            .iter()
            .filter_map(|check| check.harness())
            .map(|harness| harness.name)
            .collect();
        let unnamed: Vec<&str> = THE_HARNESSES
            .iter()
            .map(|harness| harness.name)
            .filter(|name| !named.contains(name))
            .collect();
        assert!(unnamed.is_empty(), "arneses sin entrada: {unnamed:?}");
    }

    #[test]
    fn the_saved_file_is_judged_against_the_decoded_data() {
        let data = the_data_the_site_saves();
        let judged = |saved: Option<&[u8]>| the_saved_bytes_against(saved, &data).outcome;
        assert_eq!(judged(Some(&data)), Outcome::Compliant);
        assert_eq!(judged(Some(b"otra cosa")), Outcome::Noncompliant);
        assert_eq!(judged(None), Outcome::NotObservable);
    }

    #[test]
    fn the_saved_file_is_one_that_the_preparation_clears() {
        assert!(THE_HARNESSES
            .iter()
            .flat_map(|harness| harness.fixtures)
            .any(|(name, _)| *name == THE_SAVED_NAME));
    }

    #[test]
    fn the_saved_signature_is_judged_against_the_one_the_site_received() {
        let judged = |saved: Option<&[u8]>, returned: Option<&[u8]>| {
            the_saved_signature_against(saved, returned).outcome
        };
        assert_eq!(judged(Some(b"firma"), Some(b"firma")), Outcome::Compliant);
        assert_eq!(judged(Some(b"otra"), Some(b"firma")), Outcome::Noncompliant);
        assert_eq!(judged(None, Some(b"firma")), Outcome::NotObservable);
        assert_eq!(judged(Some(b"firma"), None), Outcome::NotObservable);
    }

    #[test]
    fn every_check_that_reads_back_the_signature_drives_a_script_that_proposes_its_name() {
        let orphans: Vec<String> = read_the_catalogue()
            .unwrap()
            .iter()
            .filter(|check| {
                check.harness().map(|harness| harness.name) == Some("the_saved_signature_read_back")
            })
            .filter_map(|check| check.provocation())
            .filter(|drive| {
                !THE_SAVED_SIGNATURES
                    .iter()
                    .any(|(script, _, _)| *script == drive.script)
            })
            .map(|drive| drive.script.clone())
            .collect();
        assert!(
            orphans.is_empty(),
            "guiones sin nombre propuesto: {orphans:?}"
        );
    }

    #[test]
    fn no_two_harnesses_share_a_name() {
        let names: BTreeSet<&str> = THE_HARNESSES.iter().map(|harness| harness.name).collect();
        assert_eq!(names.len(), THE_HARNESSES.len());
    }

    #[test]
    fn a_harness_is_found_by_its_name_and_an_unknown_name_finds_none() {
        assert_eq!(
            the_harness_named("files_to_load").map(|harness| harness.name),
            Some("files_to_load")
        );
        assert!(the_harness_named("an_absent_one").is_none());
    }
}
