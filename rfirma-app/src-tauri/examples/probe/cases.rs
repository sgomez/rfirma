//! El catálogo de casos del sondeo: sus nombres y cómo se corre cada uno contra el sujeto.

use std::io::IsTerminal;
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{sleep, spawn, JoinHandle};
use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;

use crate::dossier::{CaseState, Dossier, Verdict};
use crate::errand::{THE_DRIVER_CRASH, THE_EXHAUSTED_PATIENCE};
use crate::verdicts::{the_verdict_for_private_key_check, the_verdict_for_saf_code, CaseOutcome};
use crate::Probe;

/// Mide la divergencia en `selectcert` exigiendo clave privada: AutoFirma exige clave
/// privada (al pedir el PIN de un token PKCS#11 sin respuesta resulta en cancelación),
/// mientras que rFirma lista sin sesión y devuelve el certificado.
pub(crate) const THE_PRIVATE_KEY_CHECK_CASE: &str = "selectcert_checks_private_key";

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

/// El almacén de pruebas que exige la ficha de curva elíptica: sin él, un `SAF_03` no distingue
/// el rechazo del parámetro del rechazo de una clave que ni siquiera es elíptica.
const THE_ELLIPTIC_CURVE_TEST_STORE: &str = "rfirma-test-ecc";

/// El guion de `signandsave` con un algoritmo `SHA256withECDSA`.
const THE_SIGN_AND_SAVE_WITH_ECDSA_SCRIPT: &str = "signandsavewithecdsa";

/// El caso que mide BUG-05: si `signandsave` sigue rechazando un algoritmo de curva elíptica con
/// un certificado ECC de verdad detrás.
const THE_ECDSA_ALGORITHM_CASE: &str =
    "signandsave_rejects_ecdsa_signatures_from_the_elliptic_curve_token";

/// El código con el que `signandsave` rechaza un algoritmo de firma que no reconoce.
const SAF_03_INVALID_PARAMS: &str = "SAF_03";

/// El guion de `sign` en CAdES con un `tsaURL` de sintaxis inválida —lleva un espacio—, para que
/// `TsaParams` reviente al construirse.
const THE_BROKEN_TSA_SCRIPT: &str = "signwithbrokentsa";

/// El caso que mide BUG-23: si un `tsaURL` mal formado deja la firma sin sello de tiempo y sin
/// avisar, en vez de reportar el error de configuración.
const THE_TIMESTAMP_DEGRADATION_CASE: &str =
    "a_broken_tsa_url_returns_an_unstamped_signature_without_a_warning";

/// El OID PKCS#9 `id-aa-signatureTimeStampToken` (1.2.840.113549.1.9.16.2.14), con su etiqueta y
/// su longitud DER: si aparece en la firma, el sello de tiempo se estampó de verdad.
const THE_TIMESTAMP_TOKEN_OID: [u8; 13] = [
    0x06, 0x0B, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x10, 0x02, 0x0E,
];

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
    THE_ECDSA_ALGORITHM_CASE,
    THE_TIMESTAMP_DEGRADATION_CASE,
    THE_PRIVATE_KEY_CHECK_CASE,
];

impl Probe {
    pub(crate) fn run_one(&self, dossier: &mut Dossier, case: &str, relaunch: bool) {
        if crate::protocol::KNOWN_CONDITIONS
            .iter()
            .any(|c| c.id == case)
        {
            println!("«{case}» es una condición del carril de protocolo; usa la orden «protocol» para ejecutar el carril");
            return;
        }
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
        self.monitor
            .display_header(dossier.subject(), dossier.header());
        self.run_case_and_resolve(dossier, case, 1, 1);
    }

    pub(crate) fn run_pending(&self, dossier: &mut Dossier) {
        let pending: Vec<&str> = KNOWN_CASES
            .iter()
            .copied()
            .filter(|case| !matches!(dossier.state_of(case), Some(CaseState::Resolved(_))))
            .collect();

        if pending.is_empty() {
            let total = KNOWN_CASES.len();
            println!("{}", no_pending_cases_message(total));
            return;
        }

        self.monitor
            .display_header(dossier.subject(), dossier.header());
        let total = pending.len();
        for (i, case) in pending.into_iter().enumerate() {
            self.run_case_and_resolve(dossier, case, i + 1, total);
        }
    }

    fn run_case_and_resolve(
        &self,
        dossier: &mut Dossier,
        case: &str,
        current: usize,
        total: usize,
    ) {
        self.monitor.start_progress("Caso", current, total, case);
        let start = std::time::Instant::now();
        let outcome = self.run_case(dossier, case);
        let duration = start.elapsed();

        match outcome {
            CaseOutcome::Resolved {
                verdict,
                observation,
            } => {
                let (badge_text, color) = match verdict {
                    Verdict::Confirmed => ("[CONFIRMADO]", crate::verdicts::GREEN),
                    Verdict::Refuted => ("[REFUTADO]", crate::verdicts::RED),
                    Verdict::NotObservable => ("[NO OBSERVABLE]", crate::verdicts::YELLOW),
                };
                self.monitor
                    .finish_item(badge_text, color, case, duration, observation.as_deref());
                dossier
                    .resolve(case, verdict, observation)
                    .unwrap_or_else(|complaint| {
                        eprintln!("{complaint}");
                        std::process::exit(1);
                    });
            }
            CaseOutcome::StillPending => {
                self.monitor.finish_item(
                    "[PENDIENTE]",
                    crate::verdicts::GRAY,
                    case,
                    duration,
                    Some("no hubo respuesta"),
                );
            }
        }
    }

    fn run_case(&self, dossier: &Dossier, case: &str) -> CaseOutcome {
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
            THE_ECDSA_ALGORITHM_CASE => self.run_ecdsa_algorithm_case(dossier),
            THE_TIMESTAMP_DEGRADATION_CASE => self.run_timestamp_degradation_case(),
            THE_PRIVATE_KEY_CHECK_CASE => self.run_private_key_check_case(),
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
        let answer = self
            .monitor
            .ask("¿se pidió elegir dónde guardar el fichero? [s/n]");
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

    /// El caso que mide BUG-05: exige el almacén de curva elíptica para que el rechazo se juegue
    /// sobre un certificado ECC de verdad, y no sobre un parámetro sin nada detrás.
    fn run_ecdsa_algorithm_case(&self, dossier: &Dossier) -> CaseOutcome {
        if !std::io::stdin().is_terminal() {
            return CaseOutcome::StillPending;
        }
        let store = &dossier.header().store;
        let has_ecc = store == THE_ELLIPTIC_CURVE_TEST_STORE || store.contains("softhsm");
        if !has_ecc {
            return CaseOutcome::Resolved {
                verdict: Verdict::NotObservable,
                observation: Some(format!(
                    "la tanda declara el almacén «{store}»; esta ficha exige un almacén con token de curva elíptica («{THE_ELLIPTIC_CURVE_TEST_STORE}» o softhsm2)"
                )),
            };
        }
        println!(
            "van a aparecer el diálogo de certificado y el de PIN: si el sujeto lo solicita, \
             elige el certificado de curva elíptica (token 'rfirma-test-ecc', PIN 1234)"
        );
        let outcome = self.run_errand(
            THE_ECDSA_ALGORITHM_CASE,
            THE_SIGN_AND_SAVE_WITH_ECDSA_SCRIPT,
            THE_FOURTH_PROTOCOL,
        );
        the_verdict_for_saf_code(outcome, SAF_03_INVALID_PARAMS)
    }

    /// El caso que mide BUG-23: el veredicto se juega sobre si la firma resultante lleva el
    /// sello de tiempo, no sobre si la aplicación avisó de algo.
    fn run_timestamp_degradation_case(&self) -> CaseOutcome {
        if !std::io::stdin().is_terminal() {
            return CaseOutcome::StillPending;
        }
        println!("va a aparecer el diálogo de certificado y el de PIN");
        let outcome = self.run_errand(
            THE_TIMESTAMP_DEGRADATION_CASE,
            THE_BROKEN_TSA_SCRIPT,
            THE_FOURTH_PROTOCOL,
        );
        if !outcome.launched {
            return CaseOutcome::resolved(Verdict::NotObservable);
        }
        if let Some(code) = outcome.error_code {
            return CaseOutcome::Resolved {
                verdict: Verdict::Refuted,
                observation: Some(code),
            };
        }
        match outcome.signature {
            None => CaseOutcome::resolved(Verdict::NotObservable),
            Some(signature) => {
                let stamped = the_signature_carries_a_timestamp(&signature);
                CaseOutcome::Resolved {
                    verdict: if stamped {
                        Verdict::Refuted
                    } else {
                        Verdict::Confirmed
                    },
                    observation: Some(
                        if stamped {
                            "la firma lleva el sello de tiempo"
                        } else {
                            "la firma salió sin sello de tiempo y sin error"
                        }
                        .to_owned(),
                    ),
                }
            }
        }
    }

    /// El caso de divergencia que mide si `selectcert` exige clave privada contra los tokens
    /// de prueba del proyecto: AutoFirma pide PIN o cancela al requerirlo (confirmado); rFirma
    /// devuelve el certificado sin pedir PIN ni comprobar clave privada (refutado).
    fn run_private_key_check_case(&self) -> CaseOutcome {
        if !std::io::stdin().is_terminal() {
            return CaseOutcome::StillPending;
        }
        println!("van a aparecer el diálogo de selección de certificado y el de PIN (si el sujeto exige clave privada)");
        let outcome = self.run_errand(
            THE_PRIVATE_KEY_CHECK_CASE,
            THE_SINGLE_SELECTION,
            THE_FOURTH_PROTOCOL,
        );
        if !outcome.launched {
            return CaseOutcome::resolved(Verdict::NotObservable);
        }
        let answer = self
            .monitor
            .ask("¿se pidió el PIN del token/certificado? [s/n]");
        if answer.is_empty() {
            return CaseOutcome::StillPending;
        }
        let asked_pin = answer.to_lowercase().starts_with('s');
        the_verdict_for_private_key_check(outcome, asked_pin)
    }
}

/// Si la firma en Base64 lleva el sello de tiempo estampado de verdad: busca el OID del
/// atributo no firmado, no basta con que la petición llevara un `tsaURL`.
fn the_signature_carries_a_timestamp(signature: &str) -> bool {
    let Ok(bytes) = STANDARD.decode(signature) else {
        return false;
    };
    bytes
        .windows(THE_TIMESTAMP_TOKEN_OID.len())
        .any(|window| window == THE_TIMESTAMP_TOKEN_OID)
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

pub(crate) fn no_pending_cases_message(total: usize) -> String {
    format!(
        "No quedan casos pendientes en el expediente ({total}/{total} resueltos).\n\n        Opciones para continuar:\n          - Listar el expediente:   list\n          - Relanzar un caso:       run <caso> --relaunch\n          - Carril de protocolo:    protocol"
    )
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
    use super::*;

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
        let signature = STANDARD.encode(der);
        assert!(the_signature_carries_a_timestamp(&signature));
    }

    #[test]
    fn a_signature_that_is_not_base64_is_not_stamped() {
        assert!(!the_signature_carries_a_timestamp(
            "no es base64 ni de lejos: %%%"
        ));
    }
    #[test]
    fn no_pending_cases_message_informs_complete_and_lists_options() {
        let msg = no_pending_cases_message(11);
        assert!(msg.contains("No quedan casos pendientes en el expediente (11/11 resueltos)"));
        assert!(msg.contains("list"));
        assert!(msg.contains("run <caso> --relaunch"));
        assert!(msg.contains("protocol"));
    }
}
