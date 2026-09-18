//! El cuerpo ejecutable de la suite de conformidad: cómo se conduce cada entrada del catálogo
//! contra el sujeto y cómo se resuelve su veredicto.

use std::collections::BTreeSet;
use std::io::IsTerminal;
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{sleep, spawn, JoinHandle};
use std::time::{Duration, Instant};

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;

use crate::baseline::{contrast_of, verdict_name, Contrast, Profile};
use crate::catalogue::{Check, Drive};
use crate::dossier::{CheckState, Dossier, Verdict};
use crate::errand::ErrandOutcome;
use crate::listing::{chapter_tag, the_closing_of, verdict_badge, GRAY, PENDING_BADGE};
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

impl Probe {
    pub(crate) fn run_one(
        &self,
        dossier: &mut Dossier,
        catalogue: &[Check],
        id: &str,
        relaunch: bool,
    ) {
        let Some(check) = catalogue.iter().find(|check| check.id == id) else {
            eprintln!(
                "no conozco la comprobación «{id}»; están en {}",
                crate::catalogue::the_catalogue_dir().display()
            );
            std::process::exit(2);
        };
        if !relaunch && matches!(dossier.state_of(id), Some(CheckState::Resolved(_))) {
            println!("la comprobación «{id}» ya está resuelta; usa --relaunch para repetirla");
            return;
        }
        self.monitor
            .display_header(dossier.subject(), dossier.header());
        self.run_group(dossier, &[check], &mut 0, 1);
    }

    pub(crate) fn run_pending(
        &self,
        dossier: &mut Dossier,
        catalogue: &[Check],
        suite: Option<&str>,
    ) {
        let pending: Vec<&Check> = catalogue
            .iter()
            .filter(|check| suite.is_none_or(|wanted| check.suite == wanted))
            .filter(|check| !matches!(dossier.state_of(&check.id), Some(CheckState::Resolved(_))))
            .collect();
        if pending.is_empty() {
            println!("{}", no_pending_checks_message(catalogue.len(), suite));
            self.close_the_run(dossier, catalogue, suite);
            return;
        }

        self.monitor
            .display_header(dossier.subject(), dossier.header());
        let total = pending.len();
        let mut done = 0;
        let mut already_run: BTreeSet<&str> = BTreeSet::new();
        for (index, check) in pending.iter().enumerate() {
            if already_run.contains(check.id.as_str()) {
                continue;
            }
            let group = the_group_of(check, &pending[index..]);
            for member in &group {
                already_run.insert(member.id.as_str());
            }
            self.run_group(dossier, &group, &mut done, total);
        }
        self.close_the_run(dossier, catalogue, suite);
    }

    /// El cierre de la tanda: las dos filas del resumen, las sorpresas nombradas y el código de
    /// salida que las traduce.
    fn close_the_run(&self, dossier: &Dossier, catalogue: &[Check], suite: Option<&str>) {
        let (closing, code) = the_closing_of(dossier, catalogue, suite);
        print!("{closing}");
        if code != 0 {
            std::process::exit(code);
        }
    }

    /// Corre un grupo de comprobaciones que comparten trámite: el conductor arranca una sola vez y
    /// cada entrada lee de lo que viajó lo suyo.
    fn run_group(&self, dossier: &mut Dossier, group: &[&Check], done: &mut usize, total: usize) {
        let head = group[0];
        *done += 1;
        self.monitor.announce_check(head);
        self.monitor
            .start_progress("Comprobación", *done, total, &head.id);

        if let Some(motive) = &head.unmeasurable {
            self.resolve(
                dossier,
                head,
                CheckOutcome::of(Verdict::NotObservable, motive.clone()),
                Duration::ZERO,
            );
            return;
        }
        if let Some(why) = self.the_unmet_precondition_of(head, dossier) {
            self.leave_pending(head, &why);
            return;
        }
        for warning in the_warnings_of(group) {
            println!("{warning}");
        }

        let start = Instant::now();
        let outcome = self.measure(head);
        let duration = start.elapsed();

        let answer = head
            .question
            .as_deref()
            .map(|question| self.monitor.ask(question));
        self.resolve(
            dossier,
            head,
            self.the_verdict_for(head, &outcome, answer.as_deref()),
            duration,
        );
        for member in &group[1..] {
            *done += 1;
            self.monitor.announce_check(member);
            self.monitor
                .start_progress("Comprobación", *done, total, &member.id);
            self.resolve(dossier, member, the_verdict_of(member, &outcome), duration);
        }
    }

    /// Lo que la comprobación necesita y la tanda no trae; `None` si no le falta nada.
    fn the_unmet_precondition_of(&self, check: &Check, dossier: &Dossier) -> Option<String> {
        the_unmet_need_of(
            check,
            &dossier.header().store,
            std::io::stdin().is_terminal(),
        )
        .or_else(|| the_occupied_port_complaint(check))
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
            Some("occupied_service_ports") => the_verdict_for_a_bind_failure(outcome),
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

    fn leave_pending(&self, check: &Check, why: &str) {
        self.monitor.finish_item(
            PENDING_BADGE,
            GRAY,
            &format!("{} {}", chapter_tag(&check.chapter), check.id),
            Duration::ZERO,
            Some(why),
        );
    }

    fn resolve(
        &self,
        dossier: &mut Dossier,
        check: &Check,
        outcome: CheckOutcome,
        duration: Duration,
    ) {
        match outcome {
            CheckOutcome::Resolved {
                verdict,
                observation,
            } => {
                let (badge, color) = verdict_badge(verdict);
                let note = the_note_of(check, verdict, observation.as_deref(), self.profile);
                self.monitor.finish_item(
                    badge,
                    color,
                    &format!("{} {}", chapter_tag(&check.chapter), check.id),
                    duration,
                    note.as_deref(),
                );
                dossier
                    .resolve(&check.id, verdict, observation)
                    .unwrap_or_else(|complaint| {
                        eprintln!("{complaint}");
                        std::process::exit(1);
                    });
            }
            CheckOutcome::StillPending => self.leave_pending(check, "no hubo respuesta"),
        }
    }
}

/// Lo que se dice al pie de la comprobación recién resuelta: su observación y, si lo observado no
/// es lo que la línea base declara, la sorpresa dicha en el momento.
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
fn the_group_of<'a>(head: &'a Check, rest: &[&'a Check]) -> Vec<&'a Check> {
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
        && check.harness.is_none()
        && check.question.is_none()
        && check.unmeasurable.is_none()
        && !check.needs_a_person()
        && check.required_store().is_none()
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

/// Lo que le falta a la comprobación de lo que declara `needs`, con el mensaje que dice qué falta y
/// cómo relanzarla; `None` si la persona y el almacén que pide están.
fn the_unmet_need_of(check: &Check, declared_store: &str, terminal: bool) -> Option<String> {
    if check.needs_a_person() && !terminal {
        return Some("no hay nadie delante para contestar".to_owned());
    }
    let wanted = check.required_store()?;
    let has_it = declared_store == wanted || declared_store.contains("softhsm");
    (!has_it).then(|| {
        format!(
            "la tanda declara el almacén «{declared_store}»; esta comprobación exige «{wanted}»: \
             relánzala en un expediente nuevo con `just conformance --dossier <expediente-nuevo> \
             --store {wanted} run {}`",
            check.id
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

/// El motivo por el que una comprobación sigue pendiente al cierre de la tanda: la misma
/// precondición incumplida si sigue sin cumplirse, o que no hubo respuesta la última vez.
pub(crate) fn the_reason_it_is_still_pending(
    check: &Check,
    declared_store: &str,
    terminal: bool,
) -> String {
    the_unmet_need_of(check, declared_store, terminal)
        .or_else(|| the_occupied_port_complaint(check))
        .unwrap_or_else(|| {
            format!(
                "no hubo respuesta la última vez: relánzala con `just conformance run {} --relaunch`",
                check.id
            )
        })
}

pub(crate) fn no_pending_checks_message(total: usize, suite: Option<&str>) -> String {
    let scope = suite
        .map(|wanted| format!("del conjunto {wanted}"))
        .unwrap_or_else(|| "del catálogo".to_owned());
    format!(
        "No quedan comprobaciones pendientes {scope} en el expediente ({total} en el catálogo).\n\n\
         Opciones para continuar:\n  \
         - Listar el expediente:        list\n  \
         - Listar un conjunto:          list --suite <conjunto>\n  \
         - Relanzar una comprobación:   run <id> --relaunch"
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
    fn the_message_of_a_complete_run_lists_what_to_do_next() {
        let message = no_pending_checks_message(34, None);
        assert!(message.contains("34 en el catálogo"));
        assert!(message.contains("list --suite <conjunto>"));
        assert!(message.contains("run <id> --relaunch"));
        assert!(!message.contains("protocol"));
    }

    #[test]
    fn the_message_of_a_complete_suite_names_the_suite() {
        let message = no_pending_checks_message(34, Some("versiones"));
        assert!(message.contains("del conjunto versiones"));
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
    fn a_check_that_needs_a_person_without_a_terminal_is_left_pending() {
        let check = a_check_that_needs(r#""persona""#);
        assert_eq!(
            the_unmet_need_of(&check, "cualquier-almacen", false).as_deref(),
            Some("no hay nadie delante para contestar")
        );
    }

    #[test]
    fn a_check_that_needs_a_person_with_a_terminal_needs_nothing_more() {
        let check = a_check_that_needs(r#""persona""#);
        assert_eq!(the_unmet_need_of(&check, "cualquier-almacen", true), None);
    }

    #[test]
    fn a_store_mismatch_names_the_missing_store_and_the_relaunch_order() {
        let check = a_check_that_needs(r#""almacén:rfirma-test-ecc""#);
        let reason = the_unmet_need_of(&check, "rfirma-test", true).unwrap();
        assert!(reason.contains("rfirma-test-ecc"));
        assert!(reason.contains("rfirma-test"));
        assert!(reason.contains(
            "just conformance --dossier <expediente-nuevo> --store rfirma-test-ecc run a_check"
        ));
    }

    #[test]
    fn softhsm_satisfies_any_declared_store() {
        let check = a_check_that_needs(r#""almacén:rfirma-test-ecc""#);
        assert_eq!(
            the_unmet_need_of(&check, "softhsm2-token-generico", true),
            None
        );
    }

    #[test]
    fn a_check_without_a_declared_store_needs_nothing_from_it() {
        let check = a_check_that_needs(r#""espera:5""#);
        assert_eq!(the_unmet_need_of(&check, "cualquier-almacen", true), None);
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

    #[test]
    fn the_reason_still_pending_falls_back_to_relaunch_when_nothing_is_unmet() {
        let check = a_check_that_needs(r#""persona""#);
        let reason = the_reason_it_is_still_pending(&check, "cualquier-almacen", true);
        assert!(reason.contains("just conformance run a_check --relaunch"));
    }

    #[test]
    fn the_reason_still_pending_prefers_the_unmet_need() {
        let check = a_check_that_needs(r#""persona""#);
        let reason = the_reason_it_is_still_pending(&check, "cualquier-almacen", false);
        assert_eq!(reason, "no hay nadie delante para contestar");
    }
}
