//! La línea base de la suite de conformidad: qué se espera de cada perfil de sujeto y cómo se
//! contrasta con lo observado.

use serde::{Deserialize, Deserializer, Serialize};

use crate::catalogue::Check;
use crate::dossier::{CheckRecord, CheckState, Verdict};

/// El perfil del sujeto, que es lo que selecciona la línea base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Profile {
    Autofirma,
    Rfirma,
}

impl Profile {
    pub(crate) fn named(name: &str) -> Option<Self> {
        match name {
            "autofirma" => Some(Self::Autofirma),
            "rfirma" => Some(Self::Rfirma),
            _ => None,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Autofirma => "autofirma",
            Self::Rfirma => "rfirma",
        }
    }
}

/// Lo que la línea base espera de un perfil en una comprobación.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct Expectation {
    #[serde(deserialize_with = "the_verdict_named")]
    pub verdict: Verdict,
    #[serde(default)]
    pub cause: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

fn the_verdict_named<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Verdict, D::Error> {
    let name = String::deserialize(deserializer)?;
    verdict_of(&name).ok_or_else(|| {
        serde::de::Error::custom(format!(
            "«{name}» no es un veredicto: conforme, no-conforme o no-observable"
        ))
    })
}

pub(crate) fn verdict_of(name: &str) -> Option<Verdict> {
    match name {
        "conforme" => Some(Verdict::Compliant),
        "no-conforme" => Some(Verdict::Noncompliant),
        "no-observable" => Some(Verdict::NotObservable),
        _ => None,
    }
}

pub(crate) fn verdict_name(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Compliant => "CONFORME",
        Verdict::Noncompliant => "NO CONFORME",
        Verdict::NotObservable => "NO OBSERVABLE",
    }
}

/// Cómo cae lo observado frente a lo declarado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Contrast {
    Matches,
    Surprise,
    Unmeasured,
}

impl Contrast {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Matches => "coincide",
            Self::Surprise => "SORPRESA",
            Self::Unmeasured => "sin medida",
        }
    }
}

/// Lo observado frente a lo declarado: iguales coinciden, una medida distinta es una sorpresa y
/// un `NO OBSERVABLE` donde se esperaba una medida es un hueco del arnés, no un veredicto.
pub(crate) fn contrast_of(observed: Verdict, expected: Verdict) -> Contrast {
    if observed == expected {
        Contrast::Matches
    } else if observed == Verdict::NotObservable {
        Contrast::Unmeasured
    } else {
        Contrast::Surprise
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BaselineTally {
    pub matching: usize,
    pub surprises: usize,
    pub unmeasured: usize,
    pub pending: usize,
}

impl BaselineTally {
    pub(crate) fn of<'a>(
        entries: impl Iterator<Item = (&'a CheckRecord, Option<&'a Expectation>)>,
    ) -> Self {
        let mut tally = Self::default();
        for (record, expectation) in entries {
            match (record.state, expectation) {
                (CheckState::Pending, _) => tally.pending += 1,
                (CheckState::Resolved(observed), Some(expectation)) => {
                    match contrast_of(observed, expectation.verdict) {
                        Contrast::Matches => tally.matching += 1,
                        Contrast::Surprise => tally.surprises += 1,
                        Contrast::Unmeasured => tally.unmeasured += 1,
                    }
                }
                (CheckState::Resolved(_), None) => tally.surprises += 1,
            }
        }
        tally
    }

    /// El verde de una tanda: ni sorpresas ni pendientes.
    pub(crate) fn is_green(&self) -> bool {
        self.surprises == 0 && self.pending == 0
    }
}

/// Las sorpresas de la tanda, una frase por cada una, para cerrar el informe nombrándolas.
pub(crate) fn the_surprises_of<'a>(
    entries: impl Iterator<Item = (&'a str, &'a CheckRecord, Option<&'a Expectation>)>,
) -> Vec<String> {
    entries
        .filter_map(|(id, record, expectation)| {
            let CheckState::Resolved(observed) = record.state else {
                return None;
            };
            let Some(expectation) = expectation else {
                return Some(format!(
                    "{id}: {} y la línea base no dice qué se esperaba",
                    verdict_name(observed)
                ));
            };
            (contrast_of(observed, expectation.verdict) == Contrast::Surprise).then(|| {
                format!(
                    "{id}: se esperaba {} y salió {}",
                    verdict_name(expectation.verdict),
                    verdict_name(observed)
                )
            })
        })
        .collect()
}

/// Lo que la línea base espera de `profile` en `check`, si la declara.
pub(crate) fn the_expectation_of<'a>(
    catalogue: &'a [Check],
    id: &str,
    profile: Profile,
) -> Option<&'a Expectation> {
    catalogue
        .iter()
        .find(|check| check.id == id)?
        .expect
        .get(profile.name())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_record(state: CheckState) -> CheckRecord {
        CheckRecord {
            suite: "errores".to_owned(),
            chapter: "15".to_owned(),
            citation: "ProtocolInvocationLauncher.java:741".to_owned(),
            statement: "Algo.".to_owned(),
            state,
            date: None,
            observation: None,
            duration_ms: None,
        }
    }

    fn expecting(verdict: Verdict) -> Expectation {
        Expectation {
            verdict,
            cause: None,
            note: None,
        }
    }

    #[test]
    fn what_was_expected_and_came_back_matches() {
        assert_eq!(
            contrast_of(Verdict::Noncompliant, Verdict::Noncompliant),
            Contrast::Matches
        );
    }

    #[test]
    fn two_different_measures_are_a_surprise() {
        assert_eq!(
            contrast_of(Verdict::Noncompliant, Verdict::Compliant),
            Contrast::Surprise
        );
    }

    #[test]
    fn a_measure_where_nothing_measurable_was_declared_is_a_surprise_too() {
        assert_eq!(
            contrast_of(Verdict::Compliant, Verdict::NotObservable),
            Contrast::Surprise
        );
    }

    #[test]
    fn nothing_observed_where_a_measure_was_expected_is_not_a_surprise_but_a_gap() {
        assert_eq!(
            contrast_of(Verdict::NotObservable, Verdict::Compliant),
            Contrast::Unmeasured
        );
    }

    #[test]
    fn a_run_without_surprises_or_pending_checks_is_green() {
        let tally = BaselineTally {
            matching: 33,
            surprises: 0,
            unmeasured: 1,
            pending: 0,
        };
        assert!(tally.is_green());
    }

    #[test]
    fn a_surprise_and_a_pending_check_both_break_the_green() {
        assert!(!BaselineTally {
            surprises: 1,
            ..BaselineTally::default()
        }
        .is_green());
        assert!(!BaselineTally {
            pending: 1,
            ..BaselineTally::default()
        }
        .is_green());
    }

    #[test]
    fn the_tally_counts_each_cross_once() {
        let matching = a_record(CheckState::Resolved(Verdict::Compliant));
        let surprise = a_record(CheckState::Resolved(Verdict::Noncompliant));
        let unmeasured = a_record(CheckState::Resolved(Verdict::NotObservable));
        let pending = a_record(CheckState::Pending);
        let compliant = expecting(Verdict::Compliant);

        let tally = BaselineTally::of(
            [
                (&matching, Some(&compliant)),
                (&surprise, Some(&compliant)),
                (&unmeasured, Some(&compliant)),
                (&pending, Some(&compliant)),
            ]
            .into_iter(),
        );

        assert_eq!(
            tally,
            BaselineTally {
                matching: 1,
                surprises: 1,
                unmeasured: 1,
                pending: 1,
            }
        );
    }

    #[test]
    fn the_closing_names_each_surprise_with_both_verdicts() {
        let surprise = a_record(CheckState::Resolved(Verdict::Noncompliant));
        let matching = a_record(CheckState::Resolved(Verdict::Compliant));
        let compliant = expecting(Verdict::Compliant);

        let surprises = the_surprises_of(
            [
                ("a_one", &surprise, Some(&compliant)),
                ("a_two", &matching, Some(&compliant)),
            ]
            .into_iter(),
        );

        assert_eq!(
            surprises,
            vec!["a_one: se esperaba CONFORME y salió NO CONFORME"]
        );
    }

    #[test]
    fn an_expectation_reads_its_verdict_its_cause_and_its_note() {
        let catalogue = crate::catalogue::the_catalogue_in(
            r#"
[[check]]
id = "a_one"
suite = "errores"
chapter = "15"
citation = "ProtocolInvocationLauncher.java:741"
statement = "Algo."
drive = { mode = "v4", script = "selectcert" }

[check.expect.autofirma]
verdict = "no-conforme"
cause = "BUG-15"
note = "Revienta antes de validar."

[check.expect.rfirma]
verdict = "conforme"
"#,
        )
        .unwrap();

        let expectation = the_expectation_of(&catalogue, "a_one", Profile::Autofirma).unwrap();
        assert_eq!(expectation.verdict, Verdict::Noncompliant);
        assert_eq!(expectation.cause.as_deref(), Some("BUG-15"));
        assert_eq!(
            expectation.note.as_deref(),
            Some("Revienta antes de validar.")
        );
        assert_eq!(
            the_expectation_of(&catalogue, "a_one", Profile::Rfirma)
                .unwrap()
                .verdict,
            Verdict::Compliant
        );
    }

    #[test]
    fn a_verdict_outside_the_vocabulary_is_refused_instead_of_read_as_compliant() {
        assert!(crate::catalogue::the_catalogue_in(
            "[[check]]\nid = \"a_one\"\n\n[check.expect.autofirma]\nverdict = \"regular\"\n"
        )
        .is_err());
    }
}
