//! El registro de arneses: lo que una comprobación necesita alrededor del trámite para medirse
//! —puertos ocupados, ficheros preparados—, no cómo se juzga.

use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{sleep, spawn, JoinHandle};
use std::time::Duration;

use crate::catalogue::{Check, Drive};
use crate::errand::ErrandOutcome;
use crate::Probe;

type Measure = fn(&Probe, &Check, &Drive) -> ErrandOutcome;

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
    pub(crate) fn measure(&self, probe: &Probe, check: &Check, drive: &Drive) -> ErrandOutcome {
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
        name: "occupied_service_ports",
        fixtures: &[],
        mode: READ_AND_WRITE,
        measure: |probe, check, drive| {
            let _occupied = OccupiedPorts::at(&check.ports);
            probe.drive(check, drive)
        },
    },
];

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
            .filter_map(|check| check.harness)
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
