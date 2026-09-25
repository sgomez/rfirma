//! El cuerpo ejecutable de la suite de conformidad: cómo se conduce cada entrada del catálogo
//! contra el cliente y cómo se resuelve su resultado.

use std::collections::BTreeSet;
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
use std::time::{Duration, Instant};

use crate::catalogue::{Assistance, Check, Provocation};
use crate::errand::{ErrandKey, ErrandOutcome, ObservedErrand, THE_DRIVER_CRASH};
use crate::harness::THE_HARNESSES;
use crate::judge::{judge, Verdict};
use crate::outcome::outcome_name;
use crate::outcome::{CheckState, Outcome};
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

/// Lo que deja un grupo: cómo quedó cada entrada y el trámite que se observó, si se lanzó uno
/// que valga guardar.
#[derive(Debug)]
pub(crate) struct Settled {
    pub(crate) settlements: Vec<Settlement>,
    pub(crate) observed: Option<ObservedErrand>,
}

impl From<Vec<Settlement>> for Settled {
    fn from(settlements: Vec<Settlement>) -> Self {
        Self {
            settlements,
            observed: None,
        }
    }
}

impl Probe {
    /// Corre un grupo de comprobaciones que comparten trámite —el conductor arranca una sola vez y
    /// cada entrada lee de lo que viajó lo suyo— y dice cómo quedó cada una, sin escribir nada; si
    /// el trámite ya estaba observado, lo juzga sin lanzar el cliente.
    pub(crate) fn run_group(
        &self,
        group: &[&Check],
        opening: Option<Assistance>,
        already: Option<&ObservedErrand>,
    ) -> Settled {
        let head = group[0];
        if let Some(motive) = head.unmeasurable() {
            return vec![self.settle(
                head,
                Verdict::of(Outcome::NotObservable, motive),
                Duration::ZERO,
            )]
            .into();
        }
        if let Some(observed) = already {
            self.witness.harness(&format!(
                "lo que hay que observar ya se vio en «{}»: se juzga sin volver a lanzar el cliente",
                observed.transcribed_in
            ));
            return self
                .judge_the_group(group, &observed.outcome, observed.duration())
                .into();
        }
        if let Some(tranche) = opening {
            if !self.witness.stand_by(&head.id, tranche) {
                return all_pending(
                    group,
                    "la cola se detuvo antes de llegar a las comprobaciones que te necesitan",
                )
                .into();
            }
        }
        if let Some(why) = the_unmet_precondition_of(head) {
            self.witness.harness(&format!("no se ejecuta: {why}"));
            return vec![Settlement::pending(head, why)].into();
        }
        if head.needs_a_person() {
            let fixtures = match self.prepare_the_fixtures_of(head) {
                Ok(fixtures) => fixtures,
                Err(why) => return vec![Settlement::pending(head, why)].into(),
            };
            if !self
                .witness
                .brief(&head.id, &the_briefing_of(group, fixtures.as_deref()))
            {
                return vec![Settlement::pending(head, "se saltó antes de empezar")].into();
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
            return all_pending(group, "se interrumpió mientras se ejecutaba").into();
        }
        if head.assistance() == Assistance::None && outcome.exhausted_its_patience() {
            for check in group {
                self.witness.suite_failure(&check.id, AN_UNATTENDED_TIMEOUT);
            }
            return all_pending(group, AN_UNATTENDED_TIMEOUT).into();
        }
        let observed = worth_keeping(head, &outcome).then(|| ObservedErrand {
            outcome: outcome.clone(),
            transcribed_in: head.id.clone(),
            duration_ms: u64::try_from(duration.as_millis()).unwrap_or(u64::MAX),
        });
        Settled {
            settlements: self.judge_the_group(group, &outcome, duration),
            observed,
        }
    }

    /// Juzga cada entrada del grupo con lo observado.
    fn judge_the_group(
        &self,
        group: &[&Check],
        outcome: &ErrandOutcome,
        duration: Duration,
    ) -> Vec<Settlement> {
        group
            .iter()
            .filter_map(|check| {
                let trial = check.trial()?;
                Some(self.settle(check, judge(outcome, &trial.expects), duration))
            })
            .collect()
    }

    fn settle(&self, check: &Check, verdict: Verdict, duration: Duration) -> Settlement {
        let Verdict {
            outcome,
            observation,
        } = verdict;
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

    /// Escribe, en el perfil aislado donde el cliente abre sus diálogos, los ficheros que necesita
    /// la comprobación, tras borrar los que dejó cualquier otra; `None` si no necesita ninguno.
    fn prepare_the_fixtures_of(&self, check: &Check) -> Result<Option<String>, String> {
        let directory = the_isolated_home_of(&self.client);
        for (name, _) in THE_HARNESSES.iter().flat_map(|harness| harness.fixtures) {
            let _ = std::fs::remove_file(directory.join(name));
        }
        let fixtures = check.harness().map_or(&[][..], |harness| harness.fixtures);
        if fixtures.is_empty() {
            return Ok(None);
        }
        let unprepared = |error: std::io::Error| {
            format!(
                "no se pudieron preparar los ficheros en {}: {error}",
                directory.display()
            )
        };
        let mode = check.harness().map_or(0o644, |harness| harness.mode);
        for (name, content) in fixtures {
            write_a_fixture(&directory.join(name), content, mode).map_err(unprepared)?;
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
            .provocation()
            .unwrap_or_else(|| panic!("la comprobación «{}» no dice cómo conducirse", check.id));
        match drive.harness {
            Some(harness) => harness.measure(self, check, drive),
            None => self.drive(check, drive),
        }
    }

    pub(crate) fn drive(&self, check: &Check, drive: &Provocation) -> ErrandOutcome {
        self.run_errand(
            &check.id,
            &drive.script,
            &drive.mode,
            check.declared_patience().unwrap_or(self.patience),
        )
    }
}

/// El motivo de la guarda: una comprobación sin persona no puede quedarse esperando a nadie.
const AN_UNATTENDED_TIMEOUT: &str =
    "fallo de la suite: una comprobación sin persona agotó su espera, y se mató al cliente";

/// Si lo observado vale para juzgar otra vez sin relanzar: tiene clave, terminó y no reventó el
/// conductor.
fn worth_keeping(head: &Check, outcome: &ErrandOutcome) -> bool {
    ErrandKey::of(head).is_some()
        && !outcome.exhausted_its_patience()
        && outcome.error_type.as_deref() != Some(THE_DRIVER_CRASH)
}

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
/// de su misma clave y su mismo tramo.
pub(crate) fn the_group_of<'a>(head: &'a Check, rest: &[&'a Check]) -> Vec<&'a Check> {
    let mut group = vec![head];
    if !shares_an_errand(head) {
        return group;
    }
    let key = ErrandKey::of(head);
    group.extend(
        rest.iter()
            .skip(1)
            .filter(|check| {
                shares_an_errand(check)
                    && ErrandKey::of(check) == key
                    && check.assistance() == head.assistance()
            })
            .copied(),
    );
    group
}

fn shares_an_errand(check: &Check) -> bool {
    ErrandKey::of(check).is_some() && !check.greeting()
}

/// Si la comprobación es un saludo y no se cumplió: ni resuelto de otro color ni pendiente deja
/// medir lo que abre.
fn a_failed_greeting(check: &Check, state: Option<CheckState>) -> bool {
    check.greeting() && state != Some(CheckState::Resolved(Outcome::Compliant))
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
    format!("no se ejecuta porque falló «{greeting}», que comprueba lo básico de su familia")
}

fn the_warnings_of(group: &[&Check]) -> Vec<String> {
    let mut said = BTreeSet::new();
    group
        .iter()
        .flat_map(|check| {
            check
                .instruction()
                .map(str::to_owned)
                .into_iter()
                .chain(the_wait_announcement_of(check))
        })
        .filter(|warning| said.insert(warning.clone()))
        .collect()
}

/// Lo que se cuenta a la persona antes de conducir el trámite: lo que tiene que hacer y dónde
/// están los ficheros preparados.
fn the_briefing_of(group: &[&Check], fixtures: Option<&str>) -> String {
    let mut briefing = the_warnings_of(group);
    if let Some(fixtures) = fixtures {
        briefing.push(fixtures.to_owned());
    }
    briefing.join("\n\n")
}

/// El perfil aislado, que es el HOME con el que el envoltorio lanza al cliente y donde abre sus
/// diálogos.
pub(crate) fn the_isolated_home_of(launcher: &std::path::Path) -> &std::path::Path {
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

fn write_a_fixture(path: &std::path::Path, content: &str, mode: u32) -> std::io::Result<()> {
    std::fs::write(path, content)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
}

/// Si alguno de los puertos que la comprobación necesita libres está ocupado por otra cosa; `None`
/// si no declara `ports` o todos están libres.
fn the_occupied_port_complaint(check: &Check) -> Option<String> {
    check
        .ports()
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
    use std::sync::Arc;

    use super::*;
    use crate::catalogue::{read_the_catalogue, the_catalogue_in};
    use crate::errand::fake::RecordedRunner;
    use crate::errand::{the_recorded, ErrandRunner};
    use crate::manifest::Family;
    use crate::witness::fake::FakeWitness;
    use crate::witness::Witness;

    #[test]
    fn the_briefing_names_what_to_do_and_the_fixtures() {
        let catalogue = the_catalogue_in(
            r#"
[[check]]
id = "a_save"
set = "operaciones.disco"
chapter = "10"
citation = "A.java:1"
statement = "Uno."

[check.drive]
mode = "v4"
script = "save"
harness = "a_file_to_overwrite"
act.refuse = "Se va a pedir dónde guardar."
expects.code = "CANCEL"
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
            "Se va a pedir dónde guardar.\n\nFicheros preparados en /tmp/x: ya-existe.txt."
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
    fn a_fixture_is_left_with_the_permissions_of_its_harness() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("ilegible.bin");

        write_a_fixture(&path, "contenido", 0o000).unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o000);
    }

    #[test]
    fn the_checks_that_ask_for_files_find_them_prepared() {
        let catalogue = read_the_catalogue().unwrap();
        let prepared = |id: &str| {
            catalogue
                .iter()
                .find(|check| check.id == id)
                .unwrap()
                .harness()
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

[check.drive]
mode = "v4"
script = "protocol-v4"
expects.completes.conditions = ["a-candidate-port-bound"]

[[check]]
id = "a_two"
set = "errores"
chapter = "15"
citation = "A.java:2"
statement = "Dos."

[check.drive]
mode = "v4"
script = "protocol-v4"
expects.completes.conditions = ["a-candidate-port-bound"]

[[check]]
id = "a_three"
set = "errores"
chapter = "15"
citation = "A.java:3"
statement = "Tres."

[check.drive]
mode = "v3"
script = "protocol-v3"
expects.completes.conditions = ["the-fixed-port-bound"]
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
    fn a_check_with_a_harness_or_a_person_runs_alone() {
        let catalogue = the_catalogue_in(
            r#"
[[check]]
id = "a_one"
set = "operaciones"
chapter = "10"
citation = "A.java:1"
statement = "Uno."

[check.drive]
mode = "v4"
script = "protocol-v4"
harness = "files_to_load"
act.pick_file = "Elige primero.bin."
expects.completes.conditions = ["a-candidate-port-bound"]

[[check]]
id = "a_two"
set = "errores"
chapter = "15"
citation = "A.java:2"
statement = "Dos."

[check.drive]
mode = "v4"
script = "protocol-v4"
expects.completes.conditions = ["a-candidate-port-bound"]
"#,
        )
        .unwrap();
        let pending: Vec<&Check> = catalogue.iter().collect();

        assert_eq!(the_group_of(pending[0], &pending).len(), 1);
    }

    fn with_families(mut catalogue: Vec<Check>) -> Vec<Check> {
        for check in &mut catalogue {
            check.family = match check.provocation().map(|drive| drive.script.as_str()) {
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

[check.drive]
mode = "v4"
script = "signwithanunknownformat"
greeting = true
expects.code = "SAF_06"

[[check]]
id = "the_click_greeting"
set = "saludo"
chapter = "06"
citation = "A.java:2"
statement = "Saludo con clic."

[check.drive]
mode = "v4"
script = "signcades"
greeting = true
act.consent = "Elige."
expects.completes = {}

[[check]]
id = "an_unattended_one"
set = "parametros"
chapter = "06"
citation = "A.java:3"
statement = "Otra sin ventana."

[check.drive]
mode = "v4"
script = "signwithoutaformat"
expects.code = "SAF_03"

[[check]]
id = "a_click_one"
set = "operaciones.firma"
chapter = "06"
citation = "A.java:4"
statement = "Otra con clic."

[check.drive]
mode = "v4"
script = "signcades"
act.consent = "Elige."
expects.completes = {}

[[check]]
id = "an_echo"
set = "transporte.websocket"
chapter = "05"
citation = "A.java:5"
statement = "Un eco."

[check.drive]
mode = "v4"
script = "protocol-v4"
expects.completes.conditions = ["a-candidate-port-bound"]
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

    fn a_runner(timed_out: bool) -> Arc<RecordedRunner> {
        let events = if timed_out {
            vec![r#"{"event":"timeout"}"#.to_owned()]
        } else {
            the_recorded("a-rejection-with-saf-03")
        };
        RecordedRunner::replaying(&[("signwithoutaformat", events)])
    }

    fn a_probe(witness: &Arc<FakeWitness>, runner: &Arc<RecordedRunner>) -> Probe {
        Probe {
            client: PathBuf::from("/nowhere/launch-subject"),
            trust_root: PathBuf::from("/nowhere/root.pem"),
            report: tempfile::tempdir().unwrap().keep(),
            patience: Duration::from_secs(1),
            witness: Arc::clone(witness) as Arc<dyn Witness>,
            runner: Arc::clone(runner) as Arc<dyn ErrandRunner>,
        }
    }

    fn a_rejection(act: &str) -> Vec<Check> {
        the_catalogue_in(&format!(
            r#"
[[check]]
id = "a_rejection"
set = "errores"
chapter = "15"
citation = "A.java:1"
statement = "Se rechaza."

[check.drive]
mode = "v4"
script = "signwithoutaformat"
{act}
expects.code = "SAF_03"

[[check]]
id = "its_twin"
set = "errores"
chapter = "15"
citation = "A.java:2"
statement = "También."

[check.drive]
mode = "v4"
script = "signwithoutaformat"
{act}
expects.code = "SAF_03"
"#
        ))
        .unwrap()
    }

    const CLICKED: &str = "act.consent = \"Elige.\"";

    const CANCELLED: &str = "act.cancel = \"Cancela.\"";

    fn resolved(settled: &Settled) -> Vec<(&str, Outcome)> {
        settled
            .settlements
            .iter()
            .filter_map(|settlement| match settlement {
                Settlement::Resolved { id, outcome, .. } => Some((id.as_str(), *outcome)),
                Settlement::Pending { .. } => None,
            })
            .collect()
    }

    fn pending(settled: &Settled) -> Vec<(&str, &str)> {
        settled
            .settlements
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
        let runner = a_runner(false);
        let catalogue = a_rejection(CLICKED);
        let group: Vec<&Check> = catalogue.iter().collect();

        let settled = a_probe(&witness, &runner).run_group(&group, Some(Assistance::Click), None);

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
        let runner = a_runner(false);
        let catalogue = a_rejection(CLICKED);
        let group: Vec<&Check> = catalogue.iter().collect();

        a_probe(&witness, &runner).run_group(&group, None, None);

        assert!(witness.said().is_empty());
        assert_eq!(runner.runs().len(), 1);
    }

    #[test]
    fn nobody_there_between_tranches_leaves_the_group_pending_without_driving_it() {
        let witness = Arc::new(FakeWitness {
            refuses_to_stand_by: true,
            ..FakeWitness::default()
        });
        let runner = a_runner(false);
        let catalogue = a_rejection(CANCELLED);
        let group: Vec<&Check> = catalogue.iter().take(1).collect();

        let settled = a_probe(&witness, &runner).run_group(&group, Some(Assistance::Person), None);

        assert_eq!(runner.runs().len(), 0);
        assert_eq!(
            pending(&settled),
            [(
                "a_rejection",
                "la cola se detuvo antes de llegar a las comprobaciones que te necesitan"
            )]
        );
    }

    #[test]
    fn an_unattended_check_that_exhausts_its_patience_is_a_failure_of_the_suite() {
        let witness = Arc::new(FakeWitness::default());
        let runner = a_runner(true);
        let catalogue = a_rejection("");
        let group: Vec<&Check> = catalogue.iter().collect();

        let settled = a_probe(&witness, &runner).run_group(&group, None, None);

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
        let runner = a_runner(true);
        let catalogue = a_rejection(CLICKED);
        let group: Vec<&Check> = catalogue.iter().take(1).collect();

        let settled = a_probe(&witness, &runner).run_group(&group, None, None);

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

[check.drive]
mode = "v4"
script = "protocol-v4"
act.consent = "el mismo aviso"
expects.completes.conditions = ["a-candidate-port-bound"]

[[check]]
id = "a_two"
set = "errores"
chapter = "15"
citation = "A.java:2"
statement = "Dos."

[check.drive]
mode = "v4"
script = "protocol-v4"
act.consent = "el mismo aviso"
expects.completes.conditions = ["a-candidate-port-bound"]
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

[check.drive]
mode = "v4"
script = "protocol-v4"
expects.completes.conditions = ["a-candidate-port-bound"]
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
        let check = a_check_with("act.cancel = \"Cancela.\"");
        assert_eq!(the_wait_announcement_of(&check), None);
    }

    #[test]
    fn checks_in_different_stores_or_tranches_do_not_share_an_errand() {
        let one = a_check_with("act.consent = \"Elige.\"");
        let in_ec = a_check_with("act.consent = \"Elige.\"\nstore = \"ec\"");
        let unattended = a_check_with("");
        let pending = [&one, &in_ec, &unattended];

        assert_eq!(the_group_of(&one, &pending).len(), 1);
    }

    #[test]
    fn a_group_whose_errand_was_already_observed_is_judged_without_launching_the_client() {
        let witness = Arc::new(FakeWitness::default());
        let runner = a_runner(false);
        let catalogue = a_rejection(CLICKED);
        let group: Vec<&Check> = catalogue.iter().collect();
        let first = a_probe(&witness, &runner).run_group(&group[..1], None, None);
        let observed = first.observed.expect("un rechazo terminado se guarda");

        let settled = a_probe(&witness, &runner).run_group(
            &group[1..],
            Some(Assistance::Click),
            Some(&observed),
        );

        assert_eq!(runner.runs(), ["v4/signwithoutaformat"]);
        assert_eq!(resolved(&settled), [("its_twin", Outcome::Compliant)]);
        assert_eq!(settled.observed, None);
        assert_eq!(witness.said(), Vec::<String>::new());
    }

    #[test]
    fn an_errand_that_exhausted_its_patience_is_not_kept_to_be_judged_again() {
        let witness = Arc::new(FakeWitness::default());
        let runner = a_runner(true);
        let catalogue = a_rejection(CLICKED);
        let group: Vec<&Check> = catalogue.iter().take(1).collect();

        let settled = a_probe(&witness, &runner).run_group(&group, None, None);

        assert_eq!(settled.observed, None);
    }
}
