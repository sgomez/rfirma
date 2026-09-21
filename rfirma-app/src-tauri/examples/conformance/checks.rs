//! El cuerpo ejecutable de la suite de conformidad: cómo se conduce cada entrada del catálogo
//! contra el sujeto y cómo se resuelve su veredicto.

use std::collections::{BTreeMap, BTreeSet};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{sleep, spawn, JoinHandle};
use std::time::{Duration, Instant};

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;

use crate::baseline::{contrast_of, verdict_name, Contrast, Profile};
use crate::catalogue::{Check, Drive};
use crate::dossier::{CheckState, Verdict};
use crate::errand::ErrandOutcome;
use crate::verdicts::{
    the_verdict_for_a_bind_failure, the_verdict_for_a_cancelled_dialogue,
    the_verdict_for_a_headless_batch, the_verdict_for_a_pinned_certificate,
    the_verdict_for_a_private_key_check, the_verdict_for_a_proposed_save_name,
    the_verdict_for_a_requested_input_document, the_verdict_for_a_save_confirmation,
    the_verdict_for_a_saved_signature, the_verdict_for_a_timestamp,
    the_verdict_for_a_visible_signature_area, the_verdict_for_an_automatic_selection,
    the_verdict_for_an_interactive_load, the_verdict_for_an_overwrite_confirmation, the_verdict_of,
    CheckOutcome,
};
use crate::Probe;

/// Los arneses que la suite sabe correr; el catálogo los liga por nombre y la guarda de grada A
/// exige que ninguno sobre ni falte.
pub(crate) const THE_HARNESSES: &[&str] = &[
    "automatic_certificate_selection",
    "cancelled_dialogue",
    "headless_batch_item",
    "interactive_file_load",
    "occupied_service_ports",
    "overwrite_confirmation",
    "pinned_certificate",
    "private_key_check",
    "proposed_save_name",
    "requested_input_document",
    "save_confirmation",
    "signature_saved_to_disk",
    "supported_websocket_versions",
    "timestamp_in_the_signature",
    "visible_signature_area",
];

/// El OID PKCS#9 `id-aa-signatureTimeStampToken` (1.2.840.113549.1.9.16.2.14), con su etiqueta y
/// su longitud DER: si aparece en la firma, el sello de tiempo se estampó de verdad.
const THE_TIMESTAMP_TOKEN_OID: [u8; 13] = [
    0x06, 0x0B, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x10, 0x02, 0x0E,
];

/// Cada cuánto mira el ocupante si le han dicho que suelte el puerto.
const THE_OCCUPIER_HEARTBEAT: Duration = Duration::from_millis(50);

/// Cómo quedó una entrada al terminar su grupo: resuelta, o pendiente con su motivo.
#[derive(Debug)]
pub(crate) enum Settlement {
    Resolved {
        id: String,
        verdict: Verdict,
        observation: Option<String>,
        duration: Duration,
    },
    Pending {
        id: String,
        why: String,
    },
}

impl Settlement {
    fn pending(check: &Check, why: impl Into<String>) -> Self {
        Self::Pending {
            id: check.id.clone(),
            why: why.into(),
        }
    }
}

impl Probe {
    /// Corre un grupo de comprobaciones que comparten trámite —el conductor arranca una sola vez y
    /// cada entrada lee de lo que viajó lo suyo— y dice cómo quedó cada una, sin escribir nada.
    pub(crate) fn run_group(&self, group: &[&Check], declared_store: &str) -> Vec<Settlement> {
        let head = group[0];
        if let Some(motive) = &head.unmeasurable {
            return vec![self.settle(
                head,
                CheckOutcome::of(Verdict::NotObservable, motive.clone()),
                Duration::ZERO,
            )];
        }
        if let Some(why) = the_unmet_precondition_of(head, declared_store) {
            self.witness.harness(&format!("no se corre: {why}"));
            return vec![Settlement::pending(head, why)];
        }
        if head.needs_a_person() {
            let fixtures = match self.prepare_the_fixtures_of(head) {
                Ok(fixtures) => fixtures,
                Err(why) => return vec![Settlement::pending(head, why)],
            };
            if !self
                .witness
                .brief(&head.id, &the_briefing_of(group, fixtures.as_deref()))
            {
                return vec![Settlement::pending(head, "se saltó antes de empezar")];
            }
        } else {
            for warning in the_warnings_of(group) {
                self.witness.harness(&warning);
            }
        }

        let start = Instant::now();
        let outcome = self.measure(head);
        let duration = start.elapsed();
        if self.witness.aborted() {
            return group
                .iter()
                .map(|check| Settlement::pending(check, "se interrumpió mientras se ejecutaba"))
                .collect();
        }

        let answer = match head.question.as_deref() {
            None => None,
            Some(question) => match self.witness.ask(&head.id, question) {
                Some(answer) => Some(answer),
                None => return vec![Settlement::pending(head, "se descartó la pregunta")],
            },
        };
        let mut settled = vec![self.settle(
            head,
            self.the_verdict_for(head, &outcome, answer.as_deref()),
            duration,
        )];
        settled.extend(
            group[1..]
                .iter()
                .map(|member| self.settle(member, the_verdict_of(member, &outcome), duration)),
        );
        settled
    }

    fn settle(&self, check: &Check, outcome: CheckOutcome, duration: Duration) -> Settlement {
        match outcome {
            CheckOutcome::Resolved {
                verdict,
                observation,
            } => {
                let said = the_note_of(check, verdict, observation.as_deref(), self.profile)
                    .map_or_else(
                        || verdict_name(verdict).to_owned(),
                        |note| format!("{} — {note}", verdict_name(verdict)),
                    );
                self.witness.harness(&format!("{}: {said}", check.id));
                Settlement::Resolved {
                    id: check.id.clone(),
                    verdict,
                    observation,
                    duration,
                }
            }
            CheckOutcome::StillPending => Settlement::pending(check, "no hubo respuesta"),
        }
    }

    /// Escribe, en el perfil aislado donde el sujeto abre sus diálogos, los ficheros que necesita
    /// la comprobación, tras borrar los que dejó cualquier otra; `None` si no necesita ninguno.
    fn prepare_the_fixtures_of(&self, check: &Check) -> Result<Option<String>, String> {
        let directory = the_isolated_home_of(&self.subject);
        for (name, _) in THE_HARNESSES
            .iter()
            .flat_map(|harness| the_fixtures_for(Some(harness)))
        {
            let _ = std::fs::remove_file(directory.join(name));
        }
        let fixtures = the_fixtures_for(check.harness.as_deref());
        if fixtures.is_empty() {
            return Ok(None);
        }
        let unprepared = |error: std::io::Error| {
            format!(
                "no se pudieron preparar los ficheros en {}: {error}",
                directory.display()
            )
        };
        for (name, content) in fixtures {
            std::fs::write(directory.join(name), content).map_err(unprepared)?;
        }
        let names: Vec<&str> = fixtures.iter().map(|(name, _)| *name).collect();
        Ok(Some(format!(
            "Ficheros preparados en la carpeta donde se abre el diálogo ({}): {}.",
            directory.display(),
            names.join(", ")
        )))
    }

    /// Conduce el trámite de la comprobación, con el arnés que declare si necesita más que
    /// conducir.
    fn measure(&self, check: &Check) -> ErrandOutcome {
        let drive = check
            .drive
            .as_ref()
            .unwrap_or_else(|| panic!("la comprobación «{}» no dice cómo conducirse", check.id));
        if let Some(harness) = check.harness.as_deref() {
            assert!(
                THE_HARNESSES.contains(&harness),
                "la comprobación «{}» pide un arnés que no existe: {harness}",
                check.id
            );
        }
        match check.harness.as_deref() {
            Some("occupied_service_ports") => {
                let _occupied = OccupiedPorts::at(&check.required_ports());
                self.drive(check, drive)
            }
            _ => self.drive(check, drive),
        }
    }

    fn drive(&self, check: &Check, drive: &Drive) -> ErrandOutcome {
        self.run_errand(
            &check.id,
            &drive.script,
            &drive.mode,
            check.declared_patience().unwrap_or(self.patience),
        )
    }

    fn the_verdict_for(
        &self,
        check: &Check,
        outcome: &ErrandOutcome,
        answer: Option<&str>,
    ) -> CheckOutcome {
        match check.harness.as_deref() {
            Some("save_confirmation") => {
                the_verdict_for_a_save_confirmation(outcome, answer.unwrap_or_default())
            }
            Some("private_key_check") => {
                the_verdict_for_a_private_key_check(outcome, answer.unwrap_or_default())
            }
            Some("signature_saved_to_disk") => {
                the_verdict_for_a_saved_signature(outcome, answer.unwrap_or_default())
            }
            Some("proposed_save_name") => {
                the_verdict_for_a_proposed_save_name(outcome, answer.unwrap_or_default())
            }
            Some("requested_input_document") => {
                the_verdict_for_a_requested_input_document(outcome, answer.unwrap_or_default())
            }
            Some("overwrite_confirmation") => {
                the_verdict_for_an_overwrite_confirmation(outcome, answer.unwrap_or_default())
            }
            Some("cancelled_dialogue") => {
                the_verdict_for_a_cancelled_dialogue(outcome, answer.unwrap_or_default())
            }
            Some("interactive_file_load") => {
                the_verdict_for_an_interactive_load(check, outcome, answer.unwrap_or_default())
            }
            Some("automatic_certificate_selection") => {
                the_verdict_for_an_automatic_selection(outcome, answer.unwrap_or_default())
            }
            Some("pinned_certificate") => {
                the_verdict_for_a_pinned_certificate(outcome, answer.unwrap_or_default())
            }
            Some("headless_batch_item") => {
                the_verdict_for_a_headless_batch(outcome, answer.unwrap_or_default())
            }
            Some("visible_signature_area") => {
                the_verdict_for_a_visible_signature_area(outcome, answer.unwrap_or_default())
            }
            Some("timestamp_in_the_signature") => the_verdict_for_a_timestamp(
                outcome,
                outcome
                    .signature
                    .as_deref()
                    .is_some_and(the_signature_carries_a_timestamp),
            ),
            Some("occupied_service_ports") => {
                the_verdict_for_a_bind_failure(outcome, answer.unwrap_or_default())
            }
            Some("supported_websocket_versions") => {
                self.the_verdict_for_both_channel_versions(outcome)
            }
            _ => the_verdict_of(check, outcome),
        }
    }

    /// Las dos versiones que el canal WebSocket admite se miden abriendo las dos: la que trae el
    /// trámite y la de la versión 4, que se conduce aquí mismo.
    fn the_verdict_for_both_channel_versions(&self, over_v3: &ErrandOutcome) -> CheckOutcome {
        let over_v4 = self.run_errand(
            "websocket_channel_supported_versions_accepted-v4",
            "protocol-v4",
            "v4",
            self.patience,
        );
        match (the_channel_opened(over_v3), the_channel_opened(&over_v4)) {
            (Some(true), Some(true)) => {
                CheckOutcome::of(Verdict::Compliant, "las versiones 3 y 4 abren canal")
            }
            (Some(false), _) => {
                CheckOutcome::of(Verdict::Noncompliant, "la versión 3 no abrió canal")
            }
            (_, Some(false)) => {
                CheckOutcome::of(Verdict::Noncompliant, "la versión 4 no abrió canal")
            }
            _ => CheckOutcome::of(
                Verdict::NotObservable,
                "el sujeto no llegó a hablar por uno de los dos canales",
            ),
        }
    }
}

/// Lo que la comprobación necesita y la tanda no trae; `None` si no le falta nada.
fn the_unmet_precondition_of(check: &Check, declared_store: &str) -> Option<String> {
    the_unmet_need_of(check, declared_store).or_else(|| the_occupied_port_complaint(check))
}

/// Lo que se dice de la comprobación recién resuelta: su observación y, si lo observado no es lo que
/// la línea base declara, la sorpresa dicha en el momento.
fn the_note_of(
    check: &Check,
    verdict: Verdict,
    observation: Option<&str>,
    profile: Profile,
) -> Option<String> {
    let Some(expectation) = check.expect.get(profile.name()) else {
        return observation.map(str::to_owned);
    };
    let contrast = contrast_of(verdict, expectation.verdict);
    if contrast == Contrast::Matches {
        return observation.map(str::to_owned);
    }
    let contrast_said = format!(
        "{}: se esperaba {}",
        contrast.label(),
        verdict_name(expectation.verdict)
    );
    Some(match observation {
        Some(observation) => format!("{observation} — {contrast_said}"),
        None => contrast_said,
    })
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
            .any(|condition| condition.verdict == Verdict::Compliant),
    )
}

/// Las comprobaciones que comparten trámite con `head` y pueden resolverse de la misma tanda: las
/// que se conducen igual y no piden ni arnés, ni persona, ni un almacén concreto.
pub(crate) fn the_group_of<'a>(head: &'a Check, rest: &[&'a Check]) -> Vec<&'a Check> {
    let mut group = vec![head];
    if !shares_an_errand(head) {
        return group;
    }
    group.extend(
        rest.iter()
            .skip(1)
            .filter(|check| shares_an_errand(check) && check.drive == head.drive)
            .copied(),
    );
    group
}

fn shares_an_errand(check: &Check) -> bool {
    check.drive.is_some()
        && !check.greeting
        && check.harness.is_none()
        && check.question.is_none()
        && check.unmeasurable.is_none()
        && !check.needs_a_person()
        && check.required_store().is_none()
}

/// Si la comprobación es el saludo de su conjunto y no se cumplió: ni resuelto de otro color ni
/// pendiente deja medir el resto.
fn a_failed_greeting(check: &Check, state: Option<CheckState>) -> bool {
    check.greeting && state != Some(CheckState::Resolved(Verdict::Compliant))
}

/// Los conjuntos cuyo saludo quedó resuelto sin cumplirse en una tanda anterior, con el saludo que
/// los detiene.
pub(crate) fn the_greetings_already_failed(
    catalogue: &[Check],
    state_of: impl Fn(&str) -> Option<CheckState>,
) -> BTreeMap<&str, &str> {
    catalogue
        .iter()
        .filter(|check| {
            let state = state_of(&check.id);
            matches!(state, Some(CheckState::Resolved(_))) && a_failed_greeting(check, state)
        })
        .map(|check| (check.suite.as_str(), check.id.as_str()))
        .collect()
}

pub(crate) fn the_reason_behind_a_failed_greeting(greeting: &str) -> String {
    format!("no se corre: el saludo de su conjunto, «{greeting}», no se cumplió")
}

fn the_warnings_of(group: &[&Check]) -> Vec<String> {
    let mut said = BTreeSet::new();
    group
        .iter()
        .flat_map(|check| {
            check
                .warning
                .clone()
                .into_iter()
                .chain(the_wait_announcement_of(check))
        })
        .filter(|warning| said.insert(warning.clone()))
        .collect()
}

/// Lo que se cuenta a la persona antes de conducir el trámite: los avisos, dónde están los
/// ficheros preparados y la pregunta que vendrá al terminar.
fn the_briefing_of(group: &[&Check], fixtures: Option<&str>) -> String {
    let mut briefing = the_warnings_of(group);
    if let Some(fixtures) = fixtures {
        briefing.push(fixtures.to_owned());
    }
    if let Some(question) = &group[0].question {
        let question = question.trim_end_matches("[s/n]").trim_end();
        briefing.push(format!("Al terminar se te preguntará: {question}"));
    }
    briefing.join("\n\n")
}

/// El perfil aislado, que es el HOME con el que el envoltorio lanza al sujeto y donde abre sus
/// diálogos.
fn the_isolated_home_of(launcher: &std::path::Path) -> &std::path::Path {
    launcher.parent().unwrap_or(launcher)
}

/// Los ficheros que la persona tiene que encontrar ya hechos para poder completar el trámite.
fn the_fixtures_for(harness: Option<&str>) -> &'static [(&'static str, &'static str)] {
    match harness {
        Some("overwrite_confirmation") => &[("challenge.bin", "Este fichero se sobrescribe.\n")],
        Some("interactive_file_load") => &[
            ("primero.bin", "Primer fichero de carga.\n"),
            ("segundo.bin", "Segundo fichero de carga.\n"),
        ],
        Some("requested_input_document") => &[("documento.txt", "Documento para firmar.\n")],
        _ => &[],
    }
}

/// Cuánto va a tardar una comprobación que tarda por diseño; `None` si no declara
/// `espera:<segundos>`.
fn the_wait_announcement_of(check: &Check) -> Option<String> {
    check.declared_patience().map(|patience| {
        format!(
            "Esta comprobación tarda por diseño: hasta {}s. El silencio mientras tanto no es un \
             cuelgue.",
            patience.as_secs()
        )
    })
}

/// El almacén que la comprobación declara en `needs` y la tanda no trae, con el mensaje que dice
/// cómo correrla; `None` si lo trae o no pide ninguno.
fn the_unmet_need_of(check: &Check, declared_store: &str) -> Option<String> {
    let wanted = check.required_store()?;
    let has_it = declared_store == wanted || declared_store.contains("softhsm");
    (!has_it).then(|| {
        format!(
            "el informe declara el almacén «{declared_store}»; esta comprobación exige \
             «{wanted}»: córrela en un informe nuevo creado con el almacén «{wanted}»"
        )
    })
}

/// Si alguno de los puertos que la comprobación necesita libres está ocupado por otra cosa; `None`
/// si no declara `puertos:<lista>` o todos están libres.
fn the_occupied_port_complaint(check: &Check) -> Option<String> {
    check
        .required_ports()
        .into_iter()
        .find(|port| port_is_occupied(*port))
        .map(|port| format!("el puerto {port} está ocupado por otra cosa"))
}

fn port_is_occupied(port: u16) -> bool {
    TcpListener::bind(("0.0.0.0", port)).is_err()
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

/// Los puertos que la comprobación del socket ocupa para que el sujeto no pueda ligarlos, cerrando
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
    use super::*;
    use crate::catalogue::{read_the_catalogue, the_catalogue_in};

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

    #[test]
    fn the_briefing_names_the_warning_the_fixtures_and_the_question_to_come() {
        let catalogue = the_catalogue_in(
            r#"
[[check]]
id = "a_save"
suite = "operaciones.disco"
chapter = "10"
citation = "A.java:1"
statement = "Uno."
drive = { mode = "v4", script = "save" }
harness = "overwrite_confirmation"
needs = ["persona"]
question = "¿se pidió confirmación? [s/n]"
warning = "Se va a pedir dónde guardar."
"#,
        )
        .unwrap();
        let group: Vec<&Check> = catalogue.iter().collect();

        let briefing = the_briefing_of(
            &group,
            Some("Ficheros preparados en /tmp/x: ya-existe.txt."),
        );

        assert_eq!(
            briefing,
            "Se va a pedir dónde guardar.\n\nFicheros preparados en /tmp/x: ya-existe.txt.\n\n\
             Al terminar se te preguntará: ¿se pidió confirmación?"
        );
    }

    #[test]
    fn the_fixtures_live_in_the_isolated_home_the_subject_opens_its_dialogues_in() {
        assert_eq!(
            the_isolated_home_of(std::path::Path::new(
                "/home/x/.cache/rfirma/probe-profile/launch-subject"
            )),
            std::path::Path::new("/home/x/.cache/rfirma/probe-profile")
        );
    }

    #[test]
    fn the_checks_that_ask_for_files_find_them_prepared() {
        let catalogue = read_the_catalogue().unwrap();
        let prepared = |id: &str| {
            the_fixtures_for(
                catalogue
                    .iter()
                    .find(|check| check.id == id)
                    .unwrap()
                    .harness
                    .as_deref(),
            )
            .len()
        };

        assert_eq!(
            prepared("save_confirms_before_writing_over_a_file_that_already_exists"),
            1
        );
        assert_eq!(
            prepared("load_answers_the_name_of_the_chosen_file_next_to_its_content"),
            2
        );
        assert_eq!(prepared("multiload_answers_every_chosen_file_apart"), 2);
        assert_eq!(
            prepared("signandsave_asks_for_the_document_when_the_request_brings_no_data"),
            1
        );
        assert_eq!(
            prepared("signandsave_saves_the_signature_and_returns_it_to_the_site"),
            0
        );
    }

    #[test]
    fn checks_driven_alike_share_one_errand() {
        let catalogue = the_catalogue_in(
            r#"
[[check]]
id = "a_one"
suite = "errores"
chapter = "15"
citation = "A.java:1"
statement = "Uno."
drive = { mode = "v4", script = "protocol-v4" }

[[check]]
id = "a_two"
suite = "errores"
chapter = "15"
citation = "A.java:2"
statement = "Dos."
drive = { mode = "v4", script = "protocol-v4" }

[[check]]
id = "a_three"
suite = "errores"
chapter = "15"
citation = "A.java:3"
statement = "Tres."
drive = { mode = "v3", script = "protocol-v3" }
"#,
        )
        .unwrap();
        let pending: Vec<&Check> = catalogue.iter().collect();

        let group = the_group_of(pending[0], &pending);

        assert_eq!(
            group
                .iter()
                .map(|check| check.id.as_str())
                .collect::<Vec<_>>(),
            ["a_one", "a_two"]
        );
    }

    #[test]
    fn a_check_with_a_harness_or_a_question_runs_alone() {
        let catalogue = the_catalogue_in(
            r#"
[[check]]
id = "a_one"
suite = "operaciones"
chapter = "10"
citation = "A.java:1"
statement = "Uno."
drive = { mode = "v4", script = "protocol-v4" }
harness = "save_confirmation"
needs = ["persona"]
question = "¿sí o no? [s/n]"

[[check]]
id = "a_two"
suite = "errores"
chapter = "15"
citation = "A.java:2"
statement = "Dos."
drive = { mode = "v4", script = "protocol-v4" }
"#,
        )
        .unwrap();
        let pending: Vec<&Check> = catalogue.iter().collect();

        assert_eq!(the_group_of(pending[0], &pending).len(), 1);
    }

    const A_SUITE_WITH_A_GREETING: &str = r#"
[[check]]
id = "the_greeting"
suite = "transporte.service"
chapter = "04"
citation = "A.java:1"
statement = "Saludo."
drive = { mode = "service", script = "selectcert" }
greeting = true

[[check]]
id = "a_driven_alike"
suite = "transporte.service"
chapter = "04"
citation = "A.java:2"
statement = "Otra."
drive = { mode = "service", script = "selectcert" }

[[check]]
id = "a_greeting_elsewhere"
suite = "saludo"
chapter = "09"
citation = "A.java:3"
statement = "Tres."
drive = { mode = "v4", script = "selectcert" }
greeting = true
"#;

    #[test]
    fn a_greeting_runs_alone_even_beside_checks_driven_alike() {
        let catalogue = the_catalogue_in(A_SUITE_WITH_A_GREETING).unwrap();
        let pending: Vec<&Check> = catalogue.iter().collect();

        assert_eq!(the_group_of(pending[0], &pending).len(), 1);
    }

    #[test]
    fn a_greeting_fails_unless_it_was_resolved_compliant() {
        let catalogue = the_catalogue_in(A_SUITE_WITH_A_GREETING).unwrap();
        let greeting = &catalogue[0];

        assert!(!a_failed_greeting(
            greeting,
            Some(CheckState::Resolved(Verdict::Compliant))
        ));
        assert!(a_failed_greeting(
            greeting,
            Some(CheckState::Resolved(Verdict::NotObservable))
        ));
        assert!(a_failed_greeting(greeting, Some(CheckState::Pending)));
        assert!(!a_failed_greeting(
            &catalogue[1],
            Some(CheckState::Resolved(Verdict::Noncompliant))
        ));
    }

    #[test]
    fn a_greeting_that_failed_before_stops_its_suite_and_no_other() {
        let catalogue = the_catalogue_in(A_SUITE_WITH_A_GREETING).unwrap();

        let failed = the_greetings_already_failed(&catalogue, |id| match id {
            "a_greeting_elsewhere" => Some(CheckState::Resolved(Verdict::Compliant)),
            _ => Some(CheckState::Resolved(Verdict::Noncompliant)),
        });

        assert_eq!(
            failed.into_iter().collect::<Vec<_>>(),
            [("transporte.service", "the_greeting")]
        );
    }

    #[test]
    fn a_greeting_still_pending_stops_nothing_before_it_runs() {
        let catalogue = the_catalogue_in(A_SUITE_WITH_A_GREETING).unwrap();

        let failed = the_greetings_already_failed(&catalogue, |_| Some(CheckState::Pending));

        assert!(failed.is_empty());
    }

    #[test]
    fn the_reason_behind_a_failed_greeting_names_the_greeting() {
        assert!(the_reason_behind_a_failed_greeting("the_greeting").contains("«the_greeting»"));
    }

    #[test]
    fn every_harness_the_catalogue_names_is_one_this_file_knows() {
        let named: BTreeSet<&str> = read_the_catalogue()
            .unwrap()
            .iter()
            .filter_map(|check| check.harness.clone())
            .map(|harness| {
                THE_HARNESSES
                    .iter()
                    .copied()
                    .find(|known| *known == harness)
                    .unwrap_or_else(|| panic!("arnés sin cuerpo: {harness}"))
            })
            .collect();
        assert_eq!(named.len(), THE_HARNESSES.len());
    }

    #[test]
    fn a_warning_said_once_is_not_said_twice() {
        let catalogue = the_catalogue_in(
            r#"
[[check]]
id = "a_one"
suite = "errores"
chapter = "15"
citation = "A.java:1"
statement = "Uno."
drive = { mode = "v4", script = "protocol-v4" }
warning = "el mismo aviso"

[[check]]
id = "a_two"
suite = "errores"
chapter = "15"
citation = "A.java:2"
statement = "Dos."
drive = { mode = "v4", script = "protocol-v4" }
warning = "el mismo aviso"
"#,
        )
        .unwrap();
        let group: Vec<&Check> = catalogue.iter().collect();

        assert_eq!(the_warnings_of(&group), ["el mismo aviso"]);
    }

    fn a_check_that_needs(needs: &str) -> Check {
        the_catalogue_in(&format!(
            r#"
[[check]]
id = "a_check"
suite = "errores"
chapter = "15"
citation = "A.java:1"
statement = "Algo."
drive = {{ mode = "v4", script = "protocol-v4" }}
needs = [{needs}]
"#,
        ))
        .unwrap()
        .remove(0)
    }

    #[test]
    fn a_store_mismatch_names_the_missing_store_and_where_to_run_it() {
        let check = a_check_that_needs(r#""almacén:rfirma-test-ecc""#);
        let reason = the_unmet_need_of(&check, "rfirma-test").unwrap();
        assert!(reason.contains("«rfirma-test»"));
        assert!(reason.contains("informe nuevo creado con el almacén «rfirma-test-ecc»"));
    }

    #[test]
    fn softhsm_satisfies_any_declared_store() {
        let check = a_check_that_needs(r#""almacén:rfirma-test-ecc""#);
        assert_eq!(the_unmet_need_of(&check, "softhsm2-token-generico"), None);
    }

    #[test]
    fn a_check_without_a_declared_store_needs_nothing_from_it() {
        let check = a_check_that_needs(r#""espera:5""#);
        assert_eq!(the_unmet_need_of(&check, "cualquier-almacen"), None);
    }

    #[test]
    fn an_occupied_port_is_named_and_a_free_one_is_not() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let occupied_port = listener.local_addr().unwrap().port();
        let check = a_check_that_needs(&format!(r#""puertos:{occupied_port}""#));

        let complaint = the_occupied_port_complaint(&check).unwrap();
        assert!(complaint.contains(&occupied_port.to_string()));

        drop(listener);
        assert_eq!(the_occupied_port_complaint(&check), None);
    }

    #[test]
    fn a_check_with_declared_patience_announces_how_long_it_takes() {
        let check = a_check_that_needs(r#""espera:60""#);
        let announcement = the_wait_announcement_of(&check).unwrap();
        assert!(announcement.contains("60s"));
    }

    #[test]
    fn a_check_without_declared_patience_announces_nothing() {
        let check = a_check_that_needs(r#""persona""#);
        assert_eq!(the_wait_announcement_of(&check), None);
    }
}
