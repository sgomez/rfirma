//! El cuerpo ejecutable de la suite de conformidad: cómo se conduce cada entrada del catálogo
//! contra el cliente y cómo se resuelve su resultado.

use std::collections::BTreeSet;
use std::net::TcpListener;
use std::time::{Duration, Instant};

use crate::catalogue::{Assistance, Check, Drive};
use crate::errand::{ErrandOutcome, THE_EXHAUSTED_PATIENCE};
use crate::harness::THE_HARNESSES;
use crate::outcome::outcome_name;
use crate::outcome::{CheckState, Outcome};
use crate::verdicts::{the_outcome_of, CheckOutcome};
use crate::Probe;

/// Cómo quedó una entrada al terminar su grupo: resuelta, o pendiente con su motivo.
#[derive(Debug)]
pub(crate) enum Settlement {
    Resolved {
        id: String,
        outcome: Outcome,
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
    pub(crate) fn run_group(
        &self,
        group: &[&Check],
        opening: Option<Assistance>,
    ) -> Vec<Settlement> {
        let head = group[0];
        if let Some(motive) = &head.unmeasurable {
            return vec![self.settle(
                head,
                CheckOutcome::of(Outcome::NotObservable, motive.clone()),
                Duration::ZERO,
            )];
        }
        if let Some(tranche) = opening {
            if !self.witness.stand_by(&head.id, tranche) {
                return all_pending(group, "la cola se paró antes de abrir su tramo");
            }
        }
        if let Some(why) = the_unmet_precondition_of(head) {
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
            return all_pending(group, "se interrumpió mientras se ejecutaba");
        }
        if head.assistance() == Assistance::None
            && outcome.error_type.as_deref() == Some(THE_EXHAUSTED_PATIENCE)
        {
            for check in group {
                self.witness.suite_failure(&check.id, AN_UNATTENDED_TIMEOUT);
            }
            return all_pending(group, AN_UNATTENDED_TIMEOUT);
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
            self.the_outcome_for(head, &outcome, answer.as_deref()),
            duration,
        )];
        settled.extend(
            group[1..]
                .iter()
                .map(|member| self.settle(member, the_outcome_of(member, &outcome), duration)),
        );
        settled
    }

    fn settle(&self, check: &Check, outcome: CheckOutcome, duration: Duration) -> Settlement {
        match outcome {
            CheckOutcome::Resolved {
                outcome,
                observation,
            } => {
                let said = observation.as_deref().map_or_else(
                    || outcome_name(outcome).to_owned(),
                    |observation| format!("{} — {observation}", outcome_name(outcome)),
                );
                self.witness.harness(&format!("{}: {said}", check.id));
                Settlement::Resolved {
                    id: check.id.clone(),
                    outcome,
                    observation,
                    duration,
                }
            }
            CheckOutcome::StillPending => Settlement::pending(check, "no hubo respuesta"),
        }
    }

    /// Escribe, en el perfil aislado donde el cliente abre sus diálogos, los ficheros que necesita
    /// la comprobación, tras borrar los que dejó cualquier otra; `None` si no necesita ninguno.
    fn prepare_the_fixtures_of(&self, check: &Check) -> Result<Option<String>, String> {
        let directory = the_isolated_home_of(&self.client);
        for (name, _) in THE_HARNESSES.iter().flat_map(|harness| harness.fixtures) {
            let _ = std::fs::remove_file(directory.join(name));
        }
        let fixtures = check.harness.map_or(&[][..], |harness| harness.fixtures);
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
        match check.harness {
            Some(harness) => harness.measure(self, check, drive),
            None => self.drive(check, drive),
        }
    }

    pub(crate) fn drive(&self, check: &Check, drive: &Drive) -> ErrandOutcome {
        self.run_errand(
            &check.id,
            &drive.script,
            &drive.mode,
            check.declared_patience().unwrap_or(self.patience),
        )
    }

    fn the_outcome_for(
        &self,
        check: &Check,
        outcome: &ErrandOutcome,
        answer: Option<&str>,
    ) -> CheckOutcome {
        match check.harness {
            Some(harness) => {
                harness.the_outcome_for(self, check, outcome, answer.unwrap_or_default())
            }
            None => the_outcome_of(check, outcome),
        }
    }
}

/// El motivo de la guarda: una comprobación sin persona no puede quedarse esperando a nadie.
const AN_UNATTENDED_TIMEOUT: &str =
    "fallo de la suite: una comprobación sin persona agotó su espera, y se mató al cliente";

fn all_pending(group: &[&Check], why: &str) -> Vec<Settlement> {
    group
        .iter()
        .map(|check| Settlement::pending(check, why))
        .collect()
}

/// Lo que la comprobación necesita y el equipo no le da; `None` si no le falta nada.
fn the_unmet_precondition_of(check: &Check) -> Option<String> {
    the_occupied_port_complaint(check)
}

/// Las comprobaciones que comparten trámite con `head` y pueden resolverse del mismo trámite: las
/// que se conducen igual, en el mismo almacén y el mismo tramo, y no piden ni arnés ni persona.
pub(crate) fn the_group_of<'a>(head: &'a Check, rest: &[&'a Check]) -> Vec<&'a Check> {
    let mut group = vec![head];
    if !shares_an_errand(head) {
        return group;
    }
    group.extend(
        rest.iter()
            .skip(1)
            .filter(|check| {
                shares_an_errand(check)
                    && check.drive == head.drive
                    && check.store == head.store
                    && check.assistance() == head.assistance()
            })
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
}

/// Si la comprobación es un saludo y no se cumplió: ni resuelto de otro color ni pendiente deja
/// medir lo que abre.
fn a_failed_greeting(check: &Check, state: Option<CheckState>) -> bool {
    check.greeting && state != Some(CheckState::Resolved(Outcome::Compliant))
}

/// Los saludos que quedaron resueltos sin cumplirse en una pasada anterior.
pub(crate) fn the_greetings_already_failed(
    catalogue: &[Check],
    state_of: impl Fn(&str) -> Option<CheckState>,
) -> Vec<&Check> {
    catalogue
        .iter()
        .filter(|check| {
            let state = state_of(&check.id);
            matches!(state, Some(CheckState::Resolved(_))) && a_failed_greeting(check, state)
        })
        .collect()
}

/// El saludo fallido que detiene a `check`: el de su familia en su tramo o en uno anterior.
pub(crate) fn the_greeting_that_stops<'a>(check: &Check, failed: &[&'a Check]) -> Option<&'a str> {
    failed
        .iter()
        .find(|greeting| {
            greeting.id != check.id
                && greeting.family.is_some()
                && greeting.family == check.family
                && greeting.assistance() <= check.assistance()
        })
        .map(|greeting| greeting.id.as_str())
}

pub(crate) fn the_reason_behind_a_failed_greeting(greeting: &str) -> String {
    format!("no se corre: el saludo de su familia, «{greeting}», no se cumplió")
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

/// El perfil aislado, que es el HOME con el que el envoltorio lanza al cliente y donde abre sus
/// diálogos.
fn the_isolated_home_of(launcher: &std::path::Path) -> &std::path::Path {
    launcher.parent().unwrap_or(launcher)
}

/// Cuánto va a tardar una comprobación que tarda por diseño; `None` si no declara
/// `patience_secs`.
fn the_wait_announcement_of(check: &Check) -> Option<String> {
    check.declared_patience().map(|patience| {
        format!(
            "Esta comprobación tarda por diseño: hasta {}s. El silencio mientras tanto no es un \
             cuelgue.",
            patience.as_secs()
        )
    })
}

/// Si alguno de los puertos que la comprobación necesita libres está ocupado por otra cosa; `None`
/// si no declara `ports` o todos están libres.
fn the_occupied_port_complaint(check: &Check) -> Option<String> {
    check
        .ports
        .iter()
        .copied()
        .find(|port| port_is_occupied(*port))
        .map(|port| format!("el puerto {port} está ocupado por otra cosa"))
}

fn port_is_occupied(port: u16) -> bool {
    TcpListener::bind(("0.0.0.0", port)).is_err()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::catalogue::{read_the_catalogue, the_catalogue_in};
    use crate::errand::Errands;
    use crate::manifest::Family;
    use crate::witness::fake::FakeWitness;
    use crate::witness::Witness;

    #[test]
    fn the_briefing_names_the_warning_the_fixtures_and_the_question_to_come() {
        let catalogue = the_catalogue_in(
            r#"
[[check]]
id = "a_save"
set = "operaciones.disco"
chapter = "10"
citation = "A.java:1"
statement = "Uno."
drive = { mode = "v4", script = "save" }
harness = "overwrite_confirmation"
assistance = "person"
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
    fn the_fixtures_live_in_the_isolated_home_the_client_opens_its_dialogues_in() {
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
            catalogue
                .iter()
                .find(|check| check.id == id)
                .unwrap()
                .harness
                .map_or(0, |harness| harness.fixtures.len())
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
set = "errores"
chapter = "15"
citation = "A.java:1"
statement = "Uno."
drive = { mode = "v4", script = "protocol-v4" }

[[check]]
id = "a_two"
set = "errores"
chapter = "15"
citation = "A.java:2"
statement = "Dos."
drive = { mode = "v4", script = "protocol-v4" }

[[check]]
id = "a_three"
set = "errores"
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
set = "operaciones"
chapter = "10"
citation = "A.java:1"
statement = "Uno."
drive = { mode = "v4", script = "protocol-v4" }
harness = "save_confirmation"
assistance = "person"
question = "¿sí o no? [s/n]"

[[check]]
id = "a_two"
set = "errores"
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

    fn with_families(mut catalogue: Vec<Check>) -> Vec<Check> {
        for check in &mut catalogue {
            check.family = match check.drive.as_ref().map(|drive| drive.script.as_str()) {
                Some("protocol-v4") => Some(Family::V4Echo),
                Some(_) => Some(Family::EndToEnd),
                None => None,
            };
        }
        catalogue
    }

    const TWO_FAMILIES_WITH_THEIR_GREETINGS: &str = r#"
[[check]]
id = "the_unattended_greeting"
set = "errores"
chapter = "06"
citation = "A.java:1"
statement = "Saludo sin ventana."
drive = { mode = "v4", script = "signwithanunknownformat" }
assistance = "none"
greeting = true

[[check]]
id = "the_click_greeting"
set = "saludo"
chapter = "06"
citation = "A.java:2"
statement = "Saludo con clic."
drive = { mode = "v4", script = "signcades" }
assistance = "click"
greeting = true

[[check]]
id = "an_unattended_one"
set = "parametros"
chapter = "06"
citation = "A.java:3"
statement = "Otra sin ventana."
drive = { mode = "v4", script = "signwithoutaformat" }
assistance = "none"

[[check]]
id = "a_click_one"
set = "operaciones.firma"
chapter = "06"
citation = "A.java:4"
statement = "Otra con clic."
drive = { mode = "v4", script = "signcades" }
assistance = "click"

[[check]]
id = "an_echo"
set = "transporte.websocket"
chapter = "05"
citation = "A.java:5"
statement = "Un eco."
drive = { mode = "v4", script = "protocol-v4" }
assistance = "none"
"#;

    fn the_family_catalogue() -> Vec<Check> {
        with_families(the_catalogue_in(TWO_FAMILIES_WITH_THEIR_GREETINGS).unwrap())
    }

    fn what_stops(catalogue: &[Check], failed: &[&str]) -> Vec<(String, String)> {
        let failed: Vec<&Check> = the_greetings_already_failed(catalogue, |id| {
            Some(CheckState::Resolved(if failed.contains(&id) {
                Outcome::Noncompliant
            } else {
                Outcome::Compliant
            }))
        });
        catalogue
            .iter()
            .filter_map(|check| {
                the_greeting_that_stops(check, &failed)
                    .map(|greeting| (check.id.clone(), greeting.to_owned()))
            })
            .collect()
    }

    #[test]
    fn a_greeting_runs_alone_even_beside_checks_driven_alike() {
        let catalogue = the_family_catalogue();
        let pending: Vec<&Check> = [&catalogue[1], &catalogue[3]].into();

        assert_eq!(the_group_of(pending[0], &pending).len(), 1);
    }

    #[test]
    fn a_greeting_fails_unless_it_was_resolved_compliant() {
        let catalogue = the_family_catalogue();
        let greeting = &catalogue[0];

        assert!(!a_failed_greeting(
            greeting,
            Some(CheckState::Resolved(Outcome::Compliant))
        ));
        assert!(a_failed_greeting(
            greeting,
            Some(CheckState::Resolved(Outcome::NotObservable))
        ));
        assert!(a_failed_greeting(greeting, Some(CheckState::Pending)));
        assert!(!a_failed_greeting(
            &catalogue[2],
            Some(CheckState::Resolved(Outcome::Noncompliant))
        ));
    }

    #[test]
    fn a_failed_unattended_greeting_stops_its_whole_family_in_every_set() {
        let catalogue = the_family_catalogue();

        assert_eq!(
            what_stops(&catalogue, &["the_unattended_greeting"]),
            [
                ("the_click_greeting", "the_unattended_greeting"),
                ("an_unattended_one", "the_unattended_greeting"),
                ("a_click_one", "the_unattended_greeting"),
            ]
            .map(|(check, greeting)| (check.to_owned(), greeting.to_owned()))
        );
    }

    #[test]
    fn a_failed_click_greeting_stops_its_family_from_its_tranche_on() {
        let catalogue = the_family_catalogue();

        assert_eq!(
            what_stops(&catalogue, &["the_click_greeting"]),
            [("a_click_one".to_owned(), "the_click_greeting".to_owned())]
        );
    }

    #[test]
    fn a_greeting_still_pending_stops_nothing_before_it_runs() {
        let catalogue = the_family_catalogue();

        let failed = the_greetings_already_failed(&catalogue, |_| Some(CheckState::Pending));

        assert!(failed.is_empty());
    }

    #[test]
    fn the_reason_behind_a_failed_greeting_names_the_greeting() {
        assert!(the_reason_behind_a_failed_greeting("the_greeting").contains("«the_greeting»"));
    }

    struct FakeErrands {
        timed_out: bool,
        runs: Mutex<usize>,
    }

    impl FakeErrands {
        fn answering(timed_out: bool) -> Arc<Self> {
            Arc::new(Self {
                timed_out,
                runs: Mutex::new(0),
            })
        }

        fn runs(&self) -> usize {
            *self.runs.lock().unwrap()
        }
    }

    impl Errands for FakeErrands {
        fn run(&self, _: &Probe, _: &str, _: &str, _: &str, _: Duration) -> ErrandOutcome {
            *self.runs.lock().unwrap() += 1;
            ErrandOutcome {
                launched: true,
                error_type: self.timed_out.then(|| THE_EXHAUSTED_PATIENCE.to_owned()),
                error_code: (!self.timed_out).then(|| "SAF_03".to_owned()),
                signature: None,
                data: None,
                protocol_conditions: Vec::new(),
                recent_client_lines: Vec::new(),
            }
        }
    }

    fn a_probe(witness: &Arc<FakeWitness>, errands: &Arc<FakeErrands>) -> Probe {
        Probe {
            client: PathBuf::from("/nowhere/launch-subject"),
            trust_root: PathBuf::from("/nowhere/root.pem"),
            report: std::env::temp_dir(),
            patience: Duration::from_secs(1),
            witness: Arc::clone(witness) as Arc<dyn Witness>,
            errands: Arc::clone(errands) as Arc<dyn Errands>,
        }
    }

    fn a_rejection(assistance: &str) -> Vec<Check> {
        the_catalogue_in(&format!(
            r#"
[[check]]
id = "a_rejection"
set = "errores"
chapter = "15"
citation = "A.java:1"
statement = "Se rechaza."
drive = {{ mode = "v4", script = "signwithoutaformat" }}
assistance = "{assistance}"
expects_saf = "SAF_03"

[[check]]
id = "its_twin"
set = "errores"
chapter = "15"
citation = "A.java:2"
statement = "También."
drive = {{ mode = "v4", script = "signwithoutaformat" }}
assistance = "{assistance}"
expects_saf = "SAF_03"
"#
        ))
        .unwrap()
    }

    fn resolved(settled: &[Settlement]) -> Vec<(&str, Outcome)> {
        settled
            .iter()
            .filter_map(|settlement| match settlement {
                Settlement::Resolved { id, outcome, .. } => Some((id.as_str(), *outcome)),
                Settlement::Pending { .. } => None,
            })
            .collect()
    }

    fn pending(settled: &[Settlement]) -> Vec<(&str, &str)> {
        settled
            .iter()
            .filter_map(|settlement| match settlement {
                Settlement::Pending { id, why } => Some((id.as_str(), why.as_str())),
                Settlement::Resolved { .. } => None,
            })
            .collect()
    }

    #[test]
    fn a_group_that_opens_a_tranche_waits_for_the_person_before_driving() {
        let witness = Arc::new(FakeWitness::default());
        let errands = FakeErrands::answering(false);
        let catalogue = a_rejection("click");
        let group: Vec<&Check> = catalogue.iter().collect();

        let settled = a_probe(&witness, &errands).run_group(&group, Some(Assistance::Click));

        assert_eq!(witness.said(), ["stand_by a_rejection clic"]);
        assert_eq!(
            resolved(&settled),
            [
                ("a_rejection", Outcome::Compliant),
                ("its_twin", Outcome::Compliant)
            ]
        );
    }

    #[test]
    fn a_group_inside_its_tranche_does_not_stop() {
        let witness = Arc::new(FakeWitness::default());
        let errands = FakeErrands::answering(false);
        let catalogue = a_rejection("click");
        let group: Vec<&Check> = catalogue.iter().collect();

        a_probe(&witness, &errands).run_group(&group, None);

        assert!(witness.said().is_empty());
        assert_eq!(errands.runs(), 1);
    }

    #[test]
    fn nobody_there_between_tranches_leaves_the_group_pending_without_driving_it() {
        let witness = Arc::new(FakeWitness {
            refuses_to_stand_by: true,
            ..FakeWitness::default()
        });
        let errands = FakeErrands::answering(false);
        let catalogue = a_rejection("person");
        let group: Vec<&Check> = catalogue.iter().take(1).collect();

        let settled = a_probe(&witness, &errands).run_group(&group, Some(Assistance::Person));

        assert_eq!(errands.runs(), 0);
        assert_eq!(
            pending(&settled),
            [("a_rejection", "la cola se paró antes de abrir su tramo")]
        );
    }

    #[test]
    fn an_unattended_check_that_exhausts_its_patience_is_a_failure_of_the_suite() {
        let witness = Arc::new(FakeWitness::default());
        let errands = FakeErrands::answering(true);
        let catalogue = a_rejection("none");
        let group: Vec<&Check> = catalogue.iter().collect();

        let settled = a_probe(&witness, &errands).run_group(&group, None);

        assert_eq!(
            pending(&settled),
            [
                ("a_rejection", AN_UNATTENDED_TIMEOUT),
                ("its_twin", AN_UNATTENDED_TIMEOUT)
            ]
        );
        assert_eq!(
            witness.said(),
            ["suite_failure a_rejection", "suite_failure its_twin"]
        );
    }

    #[test]
    fn a_click_check_that_exhausts_its_patience_is_judged_as_usual() {
        let witness = Arc::new(FakeWitness::default());
        let errands = FakeErrands::answering(true);
        let catalogue = a_rejection("click");
        let group: Vec<&Check> = catalogue.iter().take(1).collect();

        let settled = a_probe(&witness, &errands).run_group(&group, None);

        assert_eq!(
            resolved(&settled),
            [("a_rejection", Outcome::NotObservable)]
        );
        assert!(witness.said().is_empty());
    }

    #[test]
    fn a_warning_said_once_is_not_said_twice() {
        let catalogue = the_catalogue_in(
            r#"
[[check]]
id = "a_one"
set = "errores"
chapter = "15"
citation = "A.java:1"
statement = "Uno."
drive = { mode = "v4", script = "protocol-v4" }
warning = "el mismo aviso"

[[check]]
id = "a_two"
set = "errores"
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

    fn a_check_with(extra: &str) -> Check {
        the_catalogue_in(&format!(
            r#"
[[check]]
id = "a_check"
set = "errores"
chapter = "15"
citation = "A.java:1"
statement = "Algo."
drive = {{ mode = "v4", script = "protocol-v4" }}
{extra}
"#,
        ))
        .unwrap()
        .remove(0)
    }

    #[test]
    fn an_occupied_port_is_named_and_a_free_one_is_not() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let occupied_port = listener.local_addr().unwrap().port();
        let check = a_check_with(&format!("ports = [{occupied_port}]"));

        let complaint = the_occupied_port_complaint(&check).unwrap();
        assert!(complaint.contains(&occupied_port.to_string()));

        drop(listener);
        assert_eq!(the_occupied_port_complaint(&check), None);
    }

    #[test]
    fn a_check_with_declared_patience_announces_how_long_it_takes() {
        let check = a_check_with("patience_secs = 60");
        let announcement = the_wait_announcement_of(&check).unwrap();
        assert!(announcement.contains("60s"));
    }

    #[test]
    fn a_check_without_declared_patience_announces_nothing() {
        let check = a_check_with("assistance = \"person\"");
        assert_eq!(the_wait_announcement_of(&check), None);
    }

    #[test]
    fn checks_in_different_stores_or_tranches_do_not_share_an_errand() {
        let one = a_check_with("assistance = \"click\"");
        let in_ec = a_check_with("assistance = \"click\"\nstore = \"ec\"");
        let unattended = a_check_with("assistance = \"none\"");
        let pending = [&one, &in_ec, &unattended];

        assert_eq!(the_group_of(&one, &pending).len(), 1);
    }
}
