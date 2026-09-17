//! El catálogo de casos del sondeo: sus nombres y cómo se corre cada uno contra el sujeto.

use std::io::IsTerminal;
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{sleep, spawn, JoinHandle};
use std::time::Duration;

use crate::cli::ask;
use crate::dossier::{CaseState, Dossier, Verdict};
use crate::errand::{THE_DRIVER_CRASH, THE_EXHAUSTED_PATIENCE};
use crate::verdicts::{the_verdict_for_saf_code, CaseOutcome};
use crate::Probe;

/// El guion de una sola selección, el más corto que hace saludar al cliente publicado.
const THE_SINGLE_SELECTION: &str = "selectcert";

/// El guion de `sign` con `format=CAdES` y `mode=explicit`: un trámite entero, de punta a punta.
const THE_FULL_ERRAND: &str = "signcades";

/// El modo que el cliente publicado habla de por sí: puertos sorteados y `v=4`.
const THE_FOURTH_PROTOCOL: &str = "v4";

/// El caso que mide BUG-25: si el canal rechaza igual una versión obsoleta que una no
/// soportada, en vez de distinguirlas.
const THE_PROTOCOL_FRESHNESS_CASE: &str = "obsolete_and_unsupported_protocol_share_error_code";

/// El guion de `save`, el que dispara la ventana nativa de destino: necesita a una persona
/// delante para completarse, así que no cabe entre los casos automáticos.
const THE_SAVE_SCRIPT: &str = "save";

/// El caso interactivo que mide BUG-18: si el guardado por WebSocket pide de verdad un destino.
const THE_SAVE_DESTINATION_CASE: &str = "save_over_websocket_asks_for_a_destination";

/// El modo del conductor que hace hablar al cliente publicado con el bucle local IPv6.
const THE_IPV6_LOOPBACK_MODE: &str = "v4-ipv6";

/// El caso que mide BUG-11: si el canal de la versión 4 sigue rechazando el bucle local IPv6.
const THE_IPV6_LOOPBACK_CASE: &str = "ipv6_loopback_is_rejected_on_the_v4_channel";

/// El código con el que el canal de la versión 4 rechaza una procedencia que no es exactamente
/// `127.0.0.1`.
const SAF_47_EXTERNAL_REQUEST: &str = "SAF_47";

/// El modo del conductor que abre el carril `service`: sin `WebSocket`, hablando por el socket
/// TCP local al que ya ningún navegador llega, pero que el original sigue sirviendo.
const THE_SERVICE_MODE: &str = "service";

/// El caso feliz del carril `service`: sin él, ninguna de sus fichas es de fiar, porque el
/// cliente publicado se apoya en `XMLHttpRequest`/`WebSocket`, que Node no trae de por sí.
const THE_SERVICE_CHANNEL_REACHES_THE_SUBJECT_CASE: &str =
    "the_service_channel_reaches_the_subject";

/// El modo del conductor que fuerza el transporte `service` a hablar por una lista fija de
/// puertos, la misma que el caso ocupa de antemano.
const THE_SOCKET_BIND_FAILURE_MODE: &str = "service-bind-failure";

/// El caso que mide BUG-10: si un fallo al ligar el socket sigue siendo indistinguible, para el
/// cliente publicado, de que la aplicación no está instalada.
const THE_SOCKET_BIND_FAILURE_CASE: &str =
    "an_occupied_socket_makes_the_client_report_the_app_as_missing";

/// El error con el que el cliente publicado se rinde tras agotar los reintentos de conexión: el
/// mismo que arroja cuando la aplicación no está instalada.
const APPLICATION_NOT_FOUND_EXCEPTION: &str =
    "es.gob.afirma.standalone.ApplicationNotFoundException";

/// Cada cuánto mira el ocupante si le han dicho que suelte el puerto.
const THE_OCCUPIER_HEARTBEAT: Duration = Duration::from_millis(50);

/// Los puertos fijos que fuerza `THE_SOCKET_BIND_FAILURE_MODE`, y que el caso ocupa antes de
/// invocar al sujeto para que no le quede ninguno libre.
const THE_SOCKET_BIND_FAILURE_PORTS: [u16; 3] = [63131, 63132, 63133];

/// El código con el que `signandsave` debería rechazar la falta del verbo (`cop`), si validara
/// su presencia como hace `sign`.
const THE_SIGN_AND_SAVE_SCRIPT: &str = "signandsavewithoutaverb";

/// El caso que mide BUG-15: qué código de error llega de verdad al cliente publicado cuando
/// `signandsave` no recibe verbo.
const THE_VERB_VALIDATION_CASE: &str = "signandsave_without_a_verb_reports_its_real_error_code";

/// El código engañoso con el que BUG-15 documenta que se reporta la falta de verbo.
const SAF_09_MISLEADING_ERROR: &str = "SAF_09";

/// Los nombres de los casos que el sondeo sabe ejecutar.
pub(crate) const KNOWN_CASES: &[&str] = &[
    "saludo",
    "tramite",
    THE_PROTOCOL_FRESHNESS_CASE,
    THE_SAVE_DESTINATION_CASE,
    THE_IPV6_LOOPBACK_CASE,
    THE_SERVICE_CHANNEL_REACHES_THE_SUBJECT_CASE,
    THE_SOCKET_BIND_FAILURE_CASE,
    THE_VERB_VALIDATION_CASE,
];

impl Probe {
    pub(crate) fn run_one(&self, dossier: &mut Dossier, case: &str, relaunch: bool) {
        if !KNOWN_CASES.contains(&case) {
            eprintln!(
                "no conozco el caso «{case}»; los que hay son: {}",
                KNOWN_CASES.join(", ")
            );
            std::process::exit(2);
        }
        if !relaunch && matches!(dossier.state_of(case), Some(CaseState::Resolved(_))) {
            println!("el caso «{case}» ya está resuelto; usa --relaunch para repetirlo");
            return;
        }
        self.run_case_and_resolve(dossier, case);
    }

    pub(crate) fn run_pending(&self, dossier: &mut Dossier) {
        for case in KNOWN_CASES {
            if matches!(dossier.state_of(case), Some(CaseState::Resolved(_))) {
                continue;
            }
            self.run_case_and_resolve(dossier, case);
        }
    }

    fn run_case_and_resolve(&self, dossier: &mut Dossier, case: &str) {
        match self.run_case(case) {
            CaseOutcome::Resolved {
                verdict,
                observation,
            } => {
                dossier
                    .resolve(case, verdict, observation)
                    .unwrap_or_else(|complaint| {
                        eprintln!("{complaint}");
                        std::process::exit(1);
                    });
            }
            CaseOutcome::StillPending => {
                println!("el caso «{case}» sigue pendiente: no hubo respuesta");
            }
        }
    }

    fn run_case(&self, case: &str) -> CaseOutcome {
        match case {
            "saludo" => {
                self.run_errand(case, THE_SINGLE_SELECTION, THE_FOURTH_PROTOCOL);
                CaseOutcome::confirmed()
            }
            "tramite" => {
                self.run_errand(case, THE_FULL_ERRAND, THE_FOURTH_PROTOCOL);
                CaseOutcome::confirmed()
            }
            THE_PROTOCOL_FRESHNESS_CASE => self.run_protocol_freshness_case(),
            THE_SAVE_DESTINATION_CASE => self.run_save_destination_case(),
            THE_IPV6_LOOPBACK_CASE => self.run_ipv6_loopback_case(),
            THE_SERVICE_CHANNEL_REACHES_THE_SUBJECT_CASE => self.run_service_channel_case(),
            THE_SOCKET_BIND_FAILURE_CASE => self.run_socket_bind_failure_case(),
            THE_VERB_VALIDATION_CASE => self.run_verb_validation_case(),
            other => unreachable!("caso sin arnés: {other}"),
        }
    }

    /// El código SAF que el sujeto emite al rechazar una versión obsoleta y una no soportada:
    /// si coincide, BUG-25 sigue vigente; si distingue, está corregido.
    fn run_protocol_freshness_case(&self) -> CaseOutcome {
        let obsolete = self
            .run_errand(
                &format!("{THE_PROTOCOL_FRESHNESS_CASE}-obsolete"),
                THE_SINGLE_SELECTION,
                "v1",
            )
            .error_code;
        let unsupported = self
            .run_errand(
                &format!("{THE_PROTOCOL_FRESHNESS_CASE}-unsupported"),
                THE_SINGLE_SELECTION,
                "v99",
            )
            .error_code;
        match (obsolete, unsupported) {
            (Some(obsolete), Some(unsupported)) if obsolete == unsupported => {
                CaseOutcome::Resolved {
                    verdict: Verdict::Confirmed,
                    observation: Some(obsolete),
                }
            }
            (Some(obsolete), Some(unsupported)) => CaseOutcome::Resolved {
                verdict: Verdict::Refuted,
                observation: Some(format!("{obsolete} frente a {unsupported}")),
            },
            _ => CaseOutcome::resolved(Verdict::NotObservable),
        }
    }

    /// El caso que estrena las preguntas a la persona: `save` por WebSocket dispara la ventana
    /// nativa de destino, y solo alguien delante puede confirmar que se pidió.
    fn run_save_destination_case(&self) -> CaseOutcome {
        if !std::io::stdin().is_terminal() {
            return CaseOutcome::StillPending;
        }
        println!("va a aparecer la ventana para elegir dónde guardar el fichero");
        let outcome = self.run_errand(
            THE_SAVE_DESTINATION_CASE,
            THE_SAVE_SCRIPT,
            THE_FOURTH_PROTOCOL,
        );
        if !outcome.launched {
            return CaseOutcome::resolved(Verdict::NotObservable);
        }
        let answer = ask("¿se pidió elegir dónde guardar el fichero? [s/n]");
        if answer.is_empty() {
            return CaseOutcome::StillPending;
        }
        let confirmed = answer.to_lowercase().starts_with('s');
        let verdict = if confirmed && outcome.error_type.is_none() {
            Verdict::Confirmed
        } else {
            Verdict::Refuted
        };
        CaseOutcome::Resolved {
            verdict,
            observation: Some(answer),
        }
    }

    /// El código SAF con el que el canal de la versión 4 responde a una conexión por el bucle
    /// local IPv6: si es `SAF_47`, BUG-11 sigue vigente; si el trámite tira para adelante, está
    /// corregido.
    fn run_ipv6_loopback_case(&self) -> CaseOutcome {
        let outcome = self.run_errand(
            THE_IPV6_LOOPBACK_CASE,
            THE_SINGLE_SELECTION,
            THE_IPV6_LOOPBACK_MODE,
        );
        the_verdict_for_saf_code(outcome, SAF_47_EXTERNAL_REQUEST)
    }

    /// El caso feliz que abre el carril `service`: si el sujeto no llega a arrancar, el carril
    /// no existe todavía y ninguna de sus fichas es de fiar, así que el caso sale no observable
    /// en vez de confirmarse a ciegas.
    fn run_service_channel_case(&self) -> CaseOutcome {
        let outcome = self.run_errand(
            THE_SERVICE_CHANNEL_REACHES_THE_SUBJECT_CASE,
            THE_SINGLE_SELECTION,
            THE_SERVICE_MODE,
        );
        if outcome.launched {
            CaseOutcome::confirmed()
        } else {
            CaseOutcome::Resolved {
                verdict: Verdict::NotObservable,
                observation: outcome.error_code.or(outcome.error_type),
            }
        }
    }

    /// Ocupa de antemano los puertos que fuerza `THE_SOCKET_BIND_FAILURE_MODE`, para que
    /// `tryPorts` no encuentre ninguno libre, y mide si el cliente publicado acaba reportando la
    /// aplicación como no instalada en vez de un fallo de arranque.
    fn run_socket_bind_failure_case(&self) -> CaseOutcome {
        let _occupied_ports = OccupiedPorts::at(&THE_SOCKET_BIND_FAILURE_PORTS);
        let outcome = self.run_errand(
            THE_SOCKET_BIND_FAILURE_CASE,
            THE_SINGLE_SELECTION,
            THE_SOCKET_BIND_FAILURE_MODE,
        );
        if !outcome.launched {
            return CaseOutcome::resolved(Verdict::NotObservable);
        }
        let verdict = match outcome.error_type.as_deref() {
            Some(APPLICATION_NOT_FOUND_EXCEPTION) => Verdict::Confirmed,
            Some(THE_DRIVER_CRASH | THE_EXHAUSTED_PATIENCE) | None => Verdict::NotObservable,
            Some(_) => Verdict::Refuted,
        };
        CaseOutcome::Resolved {
            verdict,
            observation: outcome.error_code.or(outcome.error_type),
        }
    }

    /// El código con el que `signandsave` responde de verdad cuando no recibe verbo: si es el
    /// `SAF_09` engañoso que documenta BUG-15, sigue vigente.
    fn run_verb_validation_case(&self) -> CaseOutcome {
        let outcome = self.run_errand(
            THE_VERB_VALIDATION_CASE,
            THE_SIGN_AND_SAVE_SCRIPT,
            THE_FOURTH_PROTOCOL,
        );
        the_verdict_for_saf_code(outcome, SAF_09_MISLEADING_ERROR)
    }
}

/// Los puertos que el caso del socket ocupa para que el sujeto no pueda ligarlos, cerrando cada
/// conexión que les llegue: un ocupante mudo colgaría al cliente publicado en el primer eco, y
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
