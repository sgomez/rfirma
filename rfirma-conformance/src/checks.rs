//! El cuerpo ejecutable de la suite de conformidad: cómo se conduce cada entrada del catálogo
//! contra el cliente y cómo se resuelve su resultado.

use std::collections::{BTreeMap, BTreeSet};
use std::net::TcpListener;
use std::time::{Duration, Instant};

use crate::catalogue::{Check, Drive};
use crate::errand::ErrandOutcome;
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
    pub(crate) fn run_group(&self, group: &[&Check]) -> Vec<Settlement> {
        let head = group[0];
        if let Some(motive) = &head.unmeasurable {
            return vec![self.settle(
                head,
                CheckOutcome::of(Outcome::NotObservable, motive.clone()),
                Duration::ZERO,
            )];
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

/// Lo que la comprobación necesita y el equipo no le da; `None` si no le falta nada.
fn the_unmet_precondition_of(check: &Check) -> Option<String> {
    the_occupied_port_complaint(check)
}

/// Las comprobaciones que comparten trámite con `head` y pueden resolverse del mismo trámite: las
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
    check.greeting && state != Some(CheckState::Resolved(Outcome::Compliant))
}

/// Los conjuntos cuyo saludo quedó resuelto sin cumplirse en una pasada anterior, con el saludo que
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
        .map(|check| (check.set.as_str(), check.id.as_str()))
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

/// El perfil aislado, que es el HOME con el que el envoltorio lanza al cliente y donde abre sus
/// diálogos.
fn the_isolated_home_of(launcher: &std::path::Path) -> &std::path::Path {
    launcher.parent().unwrap_or(launcher)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogue::{read_the_catalogue, the_catalogue_in};

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
needs = ["persona"]
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

    const A_SET_WITH_A_GREETING: &str = r#"
[[check]]
id = "the_greeting"
set = "transporte.service"
chapter = "04"
citation = "A.java:1"
statement = "Saludo."
drive = { mode = "service", script = "selectcert" }
greeting = true

[[check]]
id = "a_driven_alike"
set = "transporte.service"
chapter = "04"
citation = "A.java:2"
statement = "Otra."
drive = { mode = "service", script = "selectcert" }

[[check]]
id = "a_greeting_elsewhere"
set = "saludo"
chapter = "09"
citation = "A.java:3"
statement = "Tres."
drive = { mode = "v4", script = "selectcert" }
greeting = true
"#;

    #[test]
    fn a_greeting_runs_alone_even_beside_checks_driven_alike() {
        let catalogue = the_catalogue_in(A_SET_WITH_A_GREETING).unwrap();
        let pending: Vec<&Check> = catalogue.iter().collect();

        assert_eq!(the_group_of(pending[0], &pending).len(), 1);
    }

    #[test]
    fn a_greeting_fails_unless_it_was_resolved_compliant() {
        let catalogue = the_catalogue_in(A_SET_WITH_A_GREETING).unwrap();
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
            &catalogue[1],
            Some(CheckState::Resolved(Outcome::Noncompliant))
        ));
    }

    #[test]
    fn a_greeting_that_failed_before_stops_its_set_and_no_other() {
        let catalogue = the_catalogue_in(A_SET_WITH_A_GREETING).unwrap();

        let failed = the_greetings_already_failed(&catalogue, |id| match id {
            "a_greeting_elsewhere" => Some(CheckState::Resolved(Outcome::Compliant)),
            _ => Some(CheckState::Resolved(Outcome::Noncompliant)),
        });

        assert_eq!(
            failed.into_iter().collect::<Vec<_>>(),
            [("transporte.service", "the_greeting")]
        );
    }

    #[test]
    fn a_greeting_still_pending_stops_nothing_before_it_runs() {
        let catalogue = the_catalogue_in(A_SET_WITH_A_GREETING).unwrap();

        let failed = the_greetings_already_failed(&catalogue, |_| Some(CheckState::Pending));

        assert!(failed.is_empty());
    }

    #[test]
    fn the_reason_behind_a_failed_greeting_names_the_greeting() {
        assert!(the_reason_behind_a_failed_greeting("the_greeting").contains("«the_greeting»"));
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

    fn a_check_that_needs(needs: &str) -> Check {
        the_catalogue_in(&format!(
            r#"
[[check]]
id = "a_check"
set = "errores"
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
