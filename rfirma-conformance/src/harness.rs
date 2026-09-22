//! El registro de arneses: cada uno reúne cómo mide, qué ficheros prepara y cómo juzga una
//! comprobación que necesita más que conducir y leer lo que viajó.

use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{sleep, spawn, JoinHandle};
use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;

use crate::catalogue::{Check, Drive};
use crate::errand::ErrandOutcome;
use crate::outcome::Outcome;
use crate::verdicts::{
    the_outcome_for_a_bind_failure, the_outcome_for_a_cancelled_dialogue,
    the_outcome_for_a_headless_batch, the_outcome_for_a_pinned_certificate,
    the_outcome_for_a_private_key_check, the_outcome_for_a_proposed_save_name,
    the_outcome_for_a_requested_input_document, the_outcome_for_a_save_confirmation,
    the_outcome_for_a_saved_signature, the_outcome_for_a_timestamp,
    the_outcome_for_a_visible_signature_area, the_outcome_for_an_automatic_selection,
    the_outcome_for_an_interactive_load, the_outcome_for_an_overwrite_confirmation, CheckOutcome,
};
use crate::Probe;

type Measure = fn(&Probe, &Check, &Drive) -> ErrandOutcome;
type Judge = fn(&Probe, &Check, &ErrandOutcome, &str) -> CheckOutcome;

/// Un arnés que el catálogo liga por nombre.
#[derive(Debug)]
pub(crate) struct Harness {
    pub name: &'static str,
    pub fixtures: &'static [(&'static str, &'static str)],
    measure: Measure,
    judge: Judge,
}

impl Harness {
    /// Conduce el trámite de la comprobación con lo que el arnés monte alrededor.
    pub(crate) fn measure(&self, probe: &Probe, check: &Check, drive: &Drive) -> ErrandOutcome {
        (self.measure)(probe, check, drive)
    }

    /// El resultado de la comprobación a partir de lo que viajó y de lo que respondió la persona.
    pub(crate) fn the_outcome_for(
        &self,
        probe: &Probe,
        check: &Check,
        outcome: &ErrandOutcome,
        answer: &str,
    ) -> CheckOutcome {
        (self.judge)(probe, check, outcome, answer)
    }
}

/// El arnés con ese nombre; `None` si el registro no lo tiene.
pub(crate) fn the_harness_named(name: &str) -> Option<&'static Harness> {
    THE_HARNESSES.iter().find(|harness| harness.name == name)
}

const JUST_DRIVE: Measure = |probe, check, drive| probe.drive(check, drive);

pub(crate) const THE_HARNESSES: &[Harness] = &[
    Harness {
        name: "automatic_certificate_selection",
        fixtures: &[],
        measure: JUST_DRIVE,
        judge: |_, _, outcome, answer| the_outcome_for_an_automatic_selection(outcome, answer),
    },
    Harness {
        name: "cancelled_dialogue",
        fixtures: &[],
        measure: JUST_DRIVE,
        judge: |_, _, outcome, answer| the_outcome_for_a_cancelled_dialogue(outcome, answer),
    },
    Harness {
        name: "headless_batch_item",
        fixtures: &[],
        measure: JUST_DRIVE,
        judge: |_, _, outcome, answer| the_outcome_for_a_headless_batch(outcome, answer),
    },
    Harness {
        name: "interactive_file_load",
        fixtures: &[
            ("primero.bin", "Primer fichero de carga.\n"),
            ("segundo.bin", "Segundo fichero de carga.\n"),
        ],
        measure: JUST_DRIVE,
        judge: |_, check, outcome, answer| {
            the_outcome_for_an_interactive_load(check, outcome, answer)
        },
    },
    Harness {
        name: "occupied_service_ports",
        fixtures: &[],
        measure: |probe, check, drive| {
            let _occupied = OccupiedPorts::at(&check.ports);
            probe.drive(check, drive)
        },
        judge: |_, _, outcome, answer| the_outcome_for_a_bind_failure(outcome, answer),
    },
    Harness {
        name: "overwrite_confirmation",
        fixtures: &[("challenge.bin", "Este fichero se sobrescribe.\n")],
        measure: JUST_DRIVE,
        judge: |_, _, outcome, answer| the_outcome_for_an_overwrite_confirmation(outcome, answer),
    },
    Harness {
        name: "pinned_certificate",
        fixtures: &[],
        measure: JUST_DRIVE,
        judge: |_, _, outcome, answer| the_outcome_for_a_pinned_certificate(outcome, answer),
    },
    Harness {
        name: "private_key_check",
        fixtures: &[],
        measure: JUST_DRIVE,
        judge: |_, _, outcome, answer| the_outcome_for_a_private_key_check(outcome, answer),
    },
    Harness {
        name: "proposed_save_name",
        fixtures: &[],
        measure: JUST_DRIVE,
        judge: |_, _, outcome, answer| the_outcome_for_a_proposed_save_name(outcome, answer),
    },
    Harness {
        name: "requested_input_document",
        fixtures: &[("documento.txt", "Documento para firmar.\n")],
        measure: JUST_DRIVE,
        judge: |_, _, outcome, answer| the_outcome_for_a_requested_input_document(outcome, answer),
    },
    Harness {
        name: "save_confirmation",
        fixtures: &[],
        measure: JUST_DRIVE,
        judge: |_, _, outcome, answer| the_outcome_for_a_save_confirmation(outcome, answer),
    },
    Harness {
        name: "signature_saved_to_disk",
        fixtures: &[],
        measure: JUST_DRIVE,
        judge: |_, _, outcome, answer| the_outcome_for_a_saved_signature(outcome, answer),
    },
    Harness {
        name: "supported_websocket_versions",
        fixtures: &[],
        measure: JUST_DRIVE,
        judge: |probe, _, outcome, _| the_outcome_for_both_channel_versions(probe, outcome),
    },
    Harness {
        name: "timestamp_in_the_signature",
        fixtures: &[],
        measure: JUST_DRIVE,
        judge: |_, _, outcome, _| {
            the_outcome_for_a_timestamp(
                outcome,
                outcome
                    .signature
                    .as_deref()
                    .is_some_and(the_signature_carries_a_timestamp),
            )
        },
    },
    Harness {
        name: "visible_signature_area",
        fixtures: &[],
        measure: JUST_DRIVE,
        judge: |_, _, outcome, answer| the_outcome_for_a_visible_signature_area(outcome, answer),
    },
];

/// El OID PKCS#9 `id-aa-signatureTimeStampToken` (1.2.840.113549.1.9.16.2.14), con su etiqueta y
/// su longitud DER: si aparece en la firma, el sello de tiempo se estampó de verdad.
const THE_TIMESTAMP_TOKEN_OID: [u8; 13] = [
    0x06, 0x0B, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x10, 0x02, 0x0E,
];

/// Cada cuánto mira el ocupante si le han dicho que suelte el puerto.
const THE_OCCUPIER_HEARTBEAT: Duration = Duration::from_millis(50);

/// Las dos versiones que el canal WebSocket admite se miden abriendo las dos: la que trae el
/// trámite y la de la versión 4, que se conduce aquí mismo.
fn the_outcome_for_both_channel_versions(probe: &Probe, over_v3: &ErrandOutcome) -> CheckOutcome {
    let over_v4 = probe.run_errand(
        "websocket_channel_supported_versions_accepted-v4",
        "protocol-v4",
        "v4",
        probe.patience,
    );
    match (the_channel_opened(over_v3), the_channel_opened(&over_v4)) {
        (Some(true), Some(true)) => {
            CheckOutcome::of(Outcome::Compliant, "las versiones 3 y 4 abren canal")
        }
        (Some(false), _) => CheckOutcome::of(Outcome::Noncompliant, "la versión 3 no abrió canal"),
        (_, Some(false)) => CheckOutcome::of(Outcome::Noncompliant, "la versión 4 no abrió canal"),
        _ => CheckOutcome::of(
            Outcome::NotObservable,
            "el cliente no llegó a hablar por uno de los dos canales",
        ),
    }
}

/// Si el canal llegó a abrirse: el conductor lo dice midiendo alguna condición, y no decir nada no
/// es lo mismo que decir que no.
fn the_channel_opened(outcome: &ErrandOutcome) -> Option<bool> {
    if !outcome.launched {
        return None;
    }
    if outcome.protocol_conditions.is_empty() {
        return outcome.error_code.is_some().then_some(false);
    }
    Some(
        outcome
            .protocol_conditions
            .iter()
            .any(|condition| condition.outcome == Outcome::Compliant),
    )
}

/// Si la firma en Base64 lleva el sello de tiempo estampado de verdad: busca el OID del atributo
/// no firmado, no basta con que la petición llevara un `tsaURL`.
fn the_signature_carries_a_timestamp(signature: &str) -> bool {
    let Ok(bytes) = STANDARD.decode(signature) else {
        return false;
    };
    bytes
        .windows(THE_TIMESTAMP_TOKEN_OID.len())
        .any(|window| window == THE_TIMESTAMP_TOKEN_OID)
}

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
            the_harness_named("save_confirmation").map(|harness| harness.name),
            Some("save_confirmation")
        );
        assert!(the_harness_named("an_absent_one").is_none());
    }

    #[test]
    fn a_signature_without_the_timestamp_oid_is_not_stamped() {
        let signature = STANDARD.encode(b"CMS SignedData sin nada de interes");
        assert!(!the_signature_carries_a_timestamp(&signature));
    }

    #[test]
    fn a_signature_with_the_timestamp_oid_is_stamped() {
        let mut der = b"prefacio arbitrario".to_vec();
        der.extend_from_slice(&THE_TIMESTAMP_TOKEN_OID);
        der.extend_from_slice(b"resto arbitrario");
        assert!(the_signature_carries_a_timestamp(&STANDARD.encode(der)));
    }

    #[test]
    fn a_signature_that_is_not_base64_is_not_stamped() {
        assert!(!the_signature_carries_a_timestamp(
            "no es base64 ni de lejos: %%%"
        ));
    }
}
