//! La traducción de lo observado en cada comprobación a su veredicto, y el listado que las
//! muestra.

use crate::catalogue::Check;
use crate::dossier::{CheckRecord, CheckState, Dossier, Verdict};
use crate::errand::{ErrandOutcome, THE_DRIVER_CRASH, THE_EXHAUSTED_PATIENCE};

/// La excepción con la que el cliente publicado reporta la cancelación de la operación.
#[cfg(test)]
pub(crate) const THE_CANCELLED_OPERATION_EXCEPTION: &str =
    "es.gob.afirma.core.AOCancelledOperationException";

/// El error con el que el cliente publicado se rinde tras agotar los reintentos de conexión: el
/// mismo que arroja cuando la aplicación no está instalada.
pub(crate) const APPLICATION_NOT_FOUND_EXCEPTION: &str =
    "es.gob.afirma.standalone.ApplicationNotFoundException";

/// Lo que deja una comprobación al correr: un veredicto, con su observación si la hubo, o nada si
/// se quedó sin respuesta y hay que seguir ofreciéndola.
pub(crate) enum CheckOutcome {
    Resolved {
        verdict: Verdict,
        observation: Option<String>,
    },
    StillPending,
}

impl CheckOutcome {
    pub(crate) fn of(verdict: Verdict, observation: impl Into<String>) -> Self {
        Self::Resolved {
            verdict,
            observation: Some(observation.into()),
        }
    }
}

/// La etiqueta de un veredicto, la que ve quien lee el listado.
pub(crate) fn verdict_badge(verdict: Verdict) -> (&'static str, &'static str) {
    match verdict {
        Verdict::Compliant => ("[CONFORME]", GREEN),
        Verdict::Noncompliant => ("[NO CONFORME]", RED),
        Verdict::NotObservable => ("[NO OBSERVABLE]", YELLOW),
    }
}

pub(crate) const PENDING_BADGE: &str = "[PENDIENTE]";

pub(crate) const RESET: &str = "\x1b[0m";
pub(crate) const GREEN: &str = "\x1b[32m";
pub(crate) const RED: &str = "\x1b[31m";
pub(crate) const YELLOW: &str = "\x1b[33m";
pub(crate) const GRAY: &str = "\x1b[90m";

const PREFIX_FECHA: &str = "  Fecha:        ";
const PREFIX_OBSERVACION: &str = "  Observación:  ";
const PREFIX_ENUNCIADO: &str = "  Enunciado:    ";
const PREFIX_CITA: &str = "  Cita:         ";

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Summary {
    pub total: usize,
    pub compliant: usize,
    pub noncompliant: usize,
    pub not_observable: usize,
    pub pending: usize,
}

impl Summary {
    pub(crate) fn of<'a>(records: impl Iterator<Item = &'a CheckRecord>) -> Self {
        let mut summary = Self::default();
        for record in records {
            summary.total += 1;
            match record.state {
                CheckState::Pending => summary.pending += 1,
                CheckState::Resolved(Verdict::Compliant) => summary.compliant += 1,
                CheckState::Resolved(Verdict::Noncompliant) => summary.noncompliant += 1,
                CheckState::Resolved(Verdict::NotObservable) => summary.not_observable += 1,
            }
        }
        summary
    }

    pub(crate) fn line(&self) -> String {
        format!(
            "{} total, {} conformes, {} no conformes, {} no observables, {} pendientes",
            self.total, self.compliant, self.noncompliant, self.not_observable, self.pending
        )
    }
}

pub(crate) fn format_badge(badge: &str, color: &str, use_color: bool) -> String {
    let char_count = badge.chars().count();
    let pad = 15usize.saturating_sub(char_count);
    let spaces = " ".repeat(pad);
    if use_color {
        format!("{color}{badge}{RESET}{spaces}")
    } else {
        format!("{badge}{spaces}")
    }
}

/// El capítulo del manual, entre corchetes, tal y como encabeza cada tarjeta.
pub(crate) fn chapter_tag(chapter: &str) -> String {
    if chapter.starts_with('[') {
        chapter.to_owned()
    } else if chapter.starts_with("Cap.") {
        format!("[{chapter}]")
    } else {
        format!("[Cap. {chapter}]")
    }
}

pub(crate) fn format_check_card(id: &str, record: &CheckRecord, use_color: bool) -> String {
    let (badge_text, color) = match record.state {
        CheckState::Resolved(verdict) => verdict_badge(verdict),
        CheckState::Pending => (PENDING_BADGE, GRAY),
    };
    let badge_formatted = format_badge(badge_text, color, use_color);
    let mut lines = vec![format!(
        "{badge_formatted} {} {id}",
        chapter_tag(&record.chapter)
    )];

    if let CheckState::Resolved(_) = record.state {
        if let Some(ref date) = record.date {
            push_field(&mut lines, PREFIX_FECHA, date);
        }
    }
    push_field(&mut lines, PREFIX_ENUNCIADO, &record.statement);
    push_field(&mut lines, PREFIX_CITA, &record.citation);
    if let Some(ref observation) = record.observation {
        push_field(&mut lines, PREFIX_OBSERVACION, observation);
    }

    lines.join("\n")
}

fn push_field(lines: &mut Vec<String>, prefix: &str, value: &str) {
    let trimmed = value.trim();
    if !trimmed.is_empty() && trimmed != "-" {
        lines.push(format!("{prefix}{trimmed}"));
    }
}

pub(crate) fn format_list(
    dossier: &Dossier,
    catalogue: &[Check],
    suite: Option<&str>,
    use_color: bool,
) -> String {
    let header = dossier.header();
    let shown: Vec<(&str, &CheckRecord)> = catalogue
        .iter()
        .filter(|check| suite.is_none_or(|wanted| check.suite == wanted))
        .filter_map(|check| dossier.checks().find(|(id, _)| *id == check.id))
        .collect();
    let summary = Summary::of(shown.iter().map(|(_, record)| *record));

    let mut out = format!(
        "tanda del {}: {} {}, sujeto {}, transporte {}, almacén {}\n\n",
        header.date,
        header.os,
        header.os_version,
        header.subject_version,
        header.transport,
        header.store
    );
    if let Some(wanted) = suite {
        out.push_str(&format!("Conjunto {wanted}: {}\n", summary.line()));
    } else {
        out.push_str(&format!("Comprobaciones: {}\n", summary.line()));
    }

    let mut current_suite = None;
    for (id, record) in shown {
        if current_suite != Some(record.suite.as_str()) {
            out.push_str(&format!("\n── {} ──\n\n", record.suite));
            current_suite = Some(record.suite.as_str());
        }
        out.push_str(&format_check_card(id, record, use_color));
        out.push_str("\n\n");
    }

    out
}

pub(crate) fn list(dossier: &Dossier, catalogue: &[Check], suite: Option<&str>) {
    use std::io::IsTerminal;
    let use_color = std::io::stdout().is_terminal();
    print!("{}", format_list(dossier, catalogue, suite, use_color));
}

/// El veredicto de una comprobación que sólo necesita que la conduzcan: el código SAF que el
/// protocolo exige si lo declaró, y si no, que el trámite se completara.
pub(crate) fn the_verdict_of(check: &Check, outcome: &ErrandOutcome) -> CheckOutcome {
    if let Some(condition) = outcome
        .protocol_conditions
        .iter()
        .find(|condition| condition.id == check.id)
    {
        return CheckOutcome::Resolved {
            verdict: condition.verdict,
            observation: condition.observation.clone(),
        };
    }
    if !outcome.launched {
        return CheckOutcome::of(
            Verdict::NotObservable,
            "el sujeto no llegó a arrancar en esta tanda",
        );
    }
    match check.expects_saf.as_deref() {
        Some(expected) => the_verdict_for_saf_code(outcome, expected),
        None => the_verdict_for_a_completed_errand(outcome),
    }
}

/// El veredicto de una exigencia que se juega a un código SAF concreto: sin código en el cable no
/// hay nada que afirmar, y sale no observable en vez de no conforme.
fn the_verdict_for_saf_code(outcome: &ErrandOutcome, expected: &str) -> CheckOutcome {
    match outcome.error_code.as_deref() {
        Some(code) if code == expected => CheckOutcome::of(Verdict::Compliant, code),
        Some(code) => CheckOutcome::of(
            Verdict::Noncompliant,
            format!("{code} donde el protocolo exige {expected}"),
        ),
        None => CheckOutcome::Resolved {
            verdict: Verdict::NotObservable,
            observation: outcome.error_type.clone(),
        },
    }
}

/// El veredicto de una exigencia que pide que el trámite salga entero: ni una excepción interna ni
/// el rechazo de una petición válida se pintan de verde.
fn the_verdict_for_a_completed_errand(outcome: &ErrandOutcome) -> CheckOutcome {
    if let Some(code) = outcome.error_code.as_deref() {
        return CheckOutcome::of(
            Verdict::Noncompliant,
            format!("{code} donde el protocolo exige que el trámite se complete"),
        );
    }
    if let Some(THE_DRIVER_CRASH | THE_EXHAUSTED_PATIENCE) = outcome.error_type.as_deref() {
        return CheckOutcome::Resolved {
            verdict: Verdict::NotObservable,
            observation: outcome.error_type.clone(),
        };
    }
    if outcome.signature.is_some() || outcome.data.is_some() {
        return CheckOutcome::of(Verdict::Compliant, "el trámite se completó");
    }
    CheckOutcome::Resolved {
        verdict: Verdict::NotObservable,
        observation: outcome.error_type.clone(),
    }
}

/// El veredicto de la confirmación de guardado: quien está delante dice si se pidió destino, y el
/// cable dice si el cliente publicado la tomó por una firma.
pub(crate) fn the_verdict_for_a_save_confirmation(
    outcome: &ErrandOutcome,
    answer: &str,
) -> CheckOutcome {
    if !outcome.launched {
        return CheckOutcome::of(
            Verdict::NotObservable,
            "el sujeto no llegó a arrancar en esta tanda",
        );
    }
    if answer.is_empty() {
        return CheckOutcome::StillPending;
    }
    let asked_for_a_destination = answer.to_lowercase().starts_with('s');
    if !asked_for_a_destination {
        return CheckOutcome::of(
            Verdict::Noncompliant,
            "no se pidió dónde guardar el fichero",
        );
    }
    match outcome.error_type.as_deref() {
        None => CheckOutcome::of(
            Verdict::Compliant,
            "se pidió destino y la confirmación llegó como tal",
        ),
        Some(THE_DRIVER_CRASH | THE_EXHAUSTED_PATIENCE) => CheckOutcome::Resolved {
            verdict: Verdict::NotObservable,
            observation: outcome.error_type.clone(),
        },
        Some(other) => CheckOutcome::of(
            Verdict::Noncompliant,
            format!("se pidió destino, pero la respuesta no se reconoció: {other}"),
        ),
    }
}

/// El veredicto del filtro de clave privada: quien está delante dice si se pidió el PIN, y el
/// cable dice si el certificado volvió sin pedirlo.
pub(crate) fn the_verdict_for_a_private_key_check(
    outcome: &ErrandOutcome,
    answer: &str,
) -> CheckOutcome {
    if !outcome.launched {
        return CheckOutcome::of(
            Verdict::NotObservable,
            "el sujeto no llegó a arrancar en esta tanda",
        );
    }
    if let Some(THE_DRIVER_CRASH | THE_EXHAUSTED_PATIENCE) = outcome.error_type.as_deref() {
        return CheckOutcome::Resolved {
            verdict: Verdict::NotObservable,
            observation: outcome.error_type.clone(),
        };
    }
    if answer.is_empty() {
        return CheckOutcome::StillPending;
    }
    if answer.to_lowercase().starts_with('s') {
        return CheckOutcome::of(
            Verdict::Compliant,
            if outcome.data.is_some() {
                "se pidió el PIN del token y devolvió el certificado tras autenticación"
            } else {
                "se pidió el PIN del token y la operación se canceló"
            },
        );
    }
    if outcome.data.is_some() {
        return CheckOutcome::of(
            Verdict::Noncompliant,
            "certificado devuelto sin pedir PIN ni comprobar clave privada",
        );
    }
    CheckOutcome::Resolved {
        verdict: Verdict::NotObservable,
        observation: outcome.error_type.clone(),
    }
}

/// El veredicto del sello de tiempo: se juega sobre la firma que volvió, no sobre si la aplicación
/// avisó de algo.
pub(crate) fn the_verdict_for_a_timestamp(outcome: &ErrandOutcome, stamped: bool) -> CheckOutcome {
    if !outcome.launched {
        return CheckOutcome::of(
            Verdict::NotObservable,
            "el sujeto no llegó a arrancar en esta tanda",
        );
    }
    if let Some(code) = outcome.error_code.as_deref() {
        return CheckOutcome::of(
            Verdict::Compliant,
            format!("no se pudo sellar y la operación lo reportó: {code}"),
        );
    }
    if outcome.signature.is_none() {
        return CheckOutcome::Resolved {
            verdict: Verdict::NotObservable,
            observation: outcome.error_type.clone(),
        };
    }
    if stamped {
        CheckOutcome::of(Verdict::Compliant, "la firma lleva el sello de tiempo")
    } else {
        CheckOutcome::of(
            Verdict::Noncompliant,
            "la firma salió sin sello de tiempo y sin error",
        )
    }
}

/// El veredicto del fallo al ligar el socket: el error que la sede recibe distingue el arranque
/// fallido de la aplicación ausente, o no lo distingue.
pub(crate) fn the_verdict_for_a_bind_failure(outcome: &ErrandOutcome) -> CheckOutcome {
    if !outcome.launched {
        return CheckOutcome::of(
            Verdict::NotObservable,
            "el sujeto no llegó a arrancar en esta tanda",
        );
    }
    match outcome.error_type.as_deref() {
        Some(APPLICATION_NOT_FOUND_EXCEPTION) => CheckOutcome::of(
            Verdict::Noncompliant,
            "el fallo al ligar llegó como «la aplicación no está instalada»",
        ),
        Some(THE_DRIVER_CRASH | THE_EXHAUSTED_PATIENCE) | None => CheckOutcome::Resolved {
            verdict: Verdict::NotObservable,
            observation: outcome.error_code.clone().or(outcome.error_type.clone()),
        },
        Some(other) => CheckOutcome::of(
            Verdict::Compliant,
            format!("el fallo al ligar llegó nombrado: {other}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogue::the_catalogue_in;
    use crate::dossier::HeaderCoordinates;

    fn a_check_expecting(saf: Option<&str>) -> Check {
        let expects = saf
            .map(|code| format!("expects_saf = \"{code}\"\n"))
            .unwrap_or_default();
        the_catalogue_in(&format!(
            "[[check]]\nid = \"an_id\"\nsuite = \"errores\"\nchapter = \"15\"\n\
             citation = \"ProtocolInvocationLauncher.java:741\"\n\
             statement = \"Algo se rechaza con SAF_47.\"\n\
             drive = {{ mode = \"v4\", script = \"selectcert\" }}\n{expects}"
        ))
        .unwrap()
        .remove(0)
    }

    fn an_outcome() -> ErrandOutcome {
        ErrandOutcome {
            launched: true,
            error_type: None,
            error_code: None,
            signature: None,
            data: None,
            protocol_conditions: Vec::new(),
            recent_subject_lines: Vec::new(),
        }
    }

    fn the_verdict(outcome: CheckOutcome) -> Verdict {
        let CheckOutcome::Resolved { verdict, .. } = outcome else {
            panic!("la comprobación debería resolverse");
        };
        verdict
    }

    #[test]
    fn the_expected_saf_code_is_compliant() {
        let outcome = ErrandOutcome {
            error_code: Some("SAF_47".to_owned()),
            ..an_outcome()
        };
        assert_eq!(
            the_verdict(the_verdict_of(&a_check_expecting(Some("SAF_47")), &outcome)),
            Verdict::Compliant
        );
    }

    #[test]
    fn another_saf_code_is_noncompliant_and_says_which_was_expected() {
        let outcome = ErrandOutcome {
            error_code: Some("SAF_03".to_owned()),
            ..an_outcome()
        };
        let CheckOutcome::Resolved {
            verdict,
            observation,
        } = the_verdict_of(&a_check_expecting(Some("SAF_47")), &outcome)
        else {
            panic!("la comprobación debería resolverse");
        };
        assert_eq!(verdict, Verdict::Noncompliant);
        assert_eq!(
            observation.as_deref(),
            Some("SAF_03 donde el protocolo exige SAF_47")
        );
    }

    #[test]
    fn a_missing_saf_code_is_not_observable_rather_than_noncompliant() {
        let outcome = ErrandOutcome {
            error_type: Some(THE_DRIVER_CRASH.to_owned()),
            ..an_outcome()
        };
        assert_eq!(
            the_verdict(the_verdict_of(&a_check_expecting(Some("SAF_47")), &outcome)),
            Verdict::NotObservable
        );
    }

    #[test]
    fn a_subject_that_never_launched_is_not_observable() {
        let outcome = ErrandOutcome {
            launched: false,
            ..an_outcome()
        };
        assert_eq!(
            the_verdict(the_verdict_of(&a_check_expecting(None), &outcome)),
            Verdict::NotObservable
        );
    }

    #[test]
    fn a_completed_errand_is_compliant_only_when_something_came_back() {
        let completed = ErrandOutcome {
            data: Some("MIID...".to_owned()),
            ..an_outcome()
        };
        assert_eq!(
            the_verdict(the_verdict_of(&a_check_expecting(None), &completed)),
            Verdict::Compliant
        );

        let silent = an_outcome();
        assert_eq!(
            the_verdict(the_verdict_of(&a_check_expecting(None), &silent)),
            Verdict::NotObservable
        );
    }

    #[test]
    fn a_rejected_valid_request_is_never_painted_green() {
        let rejected = ErrandOutcome {
            error_code: Some("SAF_09".to_owned()),
            ..an_outcome()
        };
        assert_eq!(
            the_verdict(the_verdict_of(&a_check_expecting(None), &rejected)),
            Verdict::Noncompliant
        );
    }

    #[test]
    fn a_condition_the_driver_measured_wins_over_the_default_oracle() {
        let outcome = ErrandOutcome {
            protocol_conditions: vec![crate::errand::ProtocolConditionResult {
                id: "an_id".to_owned(),
                verdict: Verdict::Noncompliant,
                observation: Some("el eco no volvió".to_owned()),
            }],
            data: Some("MIID...".to_owned()),
            ..an_outcome()
        };
        let CheckOutcome::Resolved {
            verdict,
            observation,
        } = the_verdict_of(&a_check_expecting(None), &outcome)
        else {
            panic!("la comprobación debería resolverse");
        };
        assert_eq!(verdict, Verdict::Noncompliant);
        assert_eq!(observation.as_deref(), Some("el eco no volvió"));
    }

    #[test]
    fn a_save_that_asked_for_a_destination_and_answered_cleanly_is_compliant() {
        let outcome = an_outcome();
        assert_eq!(
            the_verdict(the_verdict_for_a_save_confirmation(&outcome, "s")),
            Verdict::Compliant
        );
    }

    #[test]
    fn a_save_that_did_not_ask_for_a_destination_is_noncompliant() {
        let outcome = an_outcome();
        assert_eq!(
            the_verdict(the_verdict_for_a_save_confirmation(&outcome, "n")),
            Verdict::Noncompliant
        );
    }

    #[test]
    fn a_save_nobody_answered_stays_pending() {
        let outcome = an_outcome();
        assert!(matches!(
            the_verdict_for_a_save_confirmation(&outcome, ""),
            CheckOutcome::StillPending
        ));
    }

    #[test]
    fn a_pin_demanded_makes_the_private_key_check_compliant() {
        let cancelled = ErrandOutcome {
            error_type: Some(THE_CANCELLED_OPERATION_EXCEPTION.to_owned()),
            ..an_outcome()
        };
        let CheckOutcome::Resolved {
            verdict,
            observation,
        } = the_verdict_for_a_private_key_check(&cancelled, "s")
        else {
            panic!("la comprobación debería resolverse");
        };
        assert_eq!(verdict, Verdict::Compliant);
        assert_eq!(
            observation.as_deref(),
            Some("se pidió el PIN del token y la operación se canceló")
        );
    }

    #[test]
    fn a_certificate_returned_without_a_pin_is_noncompliant() {
        let returned = ErrandOutcome {
            data: Some("MIID...".to_owned()),
            ..an_outcome()
        };
        assert_eq!(
            the_verdict(the_verdict_for_a_private_key_check(&returned, "n")),
            Verdict::Noncompliant
        );
    }

    #[test]
    fn a_crashed_private_key_check_is_not_observable() {
        let crashed = ErrandOutcome {
            error_type: Some(THE_DRIVER_CRASH.to_owned()),
            ..an_outcome()
        };
        assert_eq!(
            the_verdict(the_verdict_for_a_private_key_check(&crashed, "n")),
            Verdict::NotObservable
        );
    }

    #[test]
    fn a_signature_without_its_requested_stamp_is_noncompliant() {
        let signed = ErrandOutcome {
            signature: Some("TUlJQg==".to_owned()),
            ..an_outcome()
        };
        assert_eq!(
            the_verdict(the_verdict_for_a_timestamp(&signed, false)),
            Verdict::Noncompliant
        );
        assert_eq!(
            the_verdict(the_verdict_for_a_timestamp(&signed, true)),
            Verdict::Compliant
        );
    }

    #[test]
    fn a_reported_stamping_failure_is_compliant() {
        let reported = ErrandOutcome {
            error_code: Some("SAF_03".to_owned()),
            ..an_outcome()
        };
        assert_eq!(
            the_verdict(the_verdict_for_a_timestamp(&reported, false)),
            Verdict::Compliant
        );
    }

    #[test]
    fn a_bind_failure_reported_as_a_missing_application_is_noncompliant() {
        let missing = ErrandOutcome {
            error_type: Some(APPLICATION_NOT_FOUND_EXCEPTION.to_owned()),
            ..an_outcome()
        };
        assert_eq!(
            the_verdict(the_verdict_for_a_bind_failure(&missing)),
            Verdict::Noncompliant
        );

        let named = ErrandOutcome {
            error_type: Some("java.io.IOException".to_owned()),
            ..an_outcome()
        };
        assert_eq!(
            the_verdict(the_verdict_for_a_bind_failure(&named)),
            Verdict::Compliant
        );
    }

    #[test]
    fn the_summary_counts_every_state_once() {
        let dossier = a_dossier_with_four_checks();
        let summary = Summary::of(dossier.checks().map(|(_, record)| record));
        assert_eq!(
            summary.line(),
            "4 total, 1 conformes, 1 no conformes, 1 no observables, 1 pendientes"
        );
    }

    fn a_dossier_with_four_checks() -> Dossier {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let catalogue = the_catalogue_in(
            r#"
[[check]]
id = "a_one"
suite = "errores"
chapter = "15"
citation = "ProtocolInvocationLauncher.java:741"
statement = "Uno."
drive = { mode = "v4", script = "selectcert" }

[[check]]
id = "a_two"
suite = "errores"
chapter = "15"
citation = "ProtocolInvocationLauncher.java:741"
statement = "Dos."
drive = { mode = "v4", script = "selectcert" }

[[check]]
id = "a_three"
suite = "versiones"
chapter = "14"
citation = "ProtocolVersion.java:60-62"
statement = "Tres."
drive = { mode = "v4", script = "selectcert" }

[[check]]
id = "a_four"
suite = "versiones"
chapter = "14"
citation = "ProtocolVersion.java:60-62"
statement = "Cuatro."
drive = { mode = "v4", script = "selectcert" }
"#,
        )
        .unwrap();
        let coordinates = HeaderCoordinates {
            os: "Linux".to_owned(),
            os_version: "6.0".to_owned(),
            subject_version: "1.9.2".to_owned(),
            transport: "websocket".to_owned(),
            store: "softhsm2".to_owned(),
        };
        let mut dossier = Dossier::open(&path, "autofirma", &catalogue, Some(coordinates)).unwrap();
        dossier
            .resolve("a_one", Verdict::Compliant, Some("SAF_03".to_owned()))
            .unwrap();
        dossier
            .resolve("a_two", Verdict::Noncompliant, None)
            .unwrap();
        dossier
            .resolve("a_three", Verdict::NotObservable, None)
            .unwrap();
        dossier
    }

    #[test]
    fn a_card_carries_the_chapter_the_statement_and_the_citation() {
        let record = CheckRecord {
            suite: "errores".to_owned(),
            chapter: "15".to_owned(),
            citation: "ProtocolInvocationLauncher.java:741".to_owned(),
            statement: "Parámetros de entrada con sintaxis inválida se rechazan con SAF_03."
                .to_owned(),
            state: CheckState::Resolved(Verdict::Compliant),
            date: Some("2026-09-17".to_owned()),
            observation: Some("SAF_03".to_owned()),
        };
        let formatted = format_check_card("invalid_parameters_syntax_rejected", &record, false);
        assert_eq!(
            formatted,
            "[CONFORME]      [Cap. 15] invalid_parameters_syntax_rejected\n  Fecha:        2026-09-17\n  Enunciado:    Parámetros de entrada con sintaxis inválida se rechazan con SAF_03.\n  Cita:         ProtocolInvocationLauncher.java:741\n  Observación:  SAF_03"
        );
    }

    #[test]
    fn a_pending_card_omits_the_date_and_the_observation() {
        let record = CheckRecord {
            suite: "operaciones".to_owned(),
            chapter: "09".to_owned(),
            citation: "KeyStoreUtilities.java:248".to_owned(),
            statement: "selectcert filtra los certificados sin clave privada.".to_owned(),
            state: CheckState::Pending,
            date: None,
            observation: None,
        };
        let formatted = format_check_card(
            "selectcert_only_offers_certificates_with_a_private_key",
            &record,
            false,
        );
        assert!(formatted.starts_with(
            "[PENDIENTE]     [Cap. 09] selectcert_only_offers_certificates_with_a_private_key"
        ));
        assert!(!formatted.contains("Fecha"));
        assert!(!formatted.contains("Observación"));
    }

    #[test]
    fn no_card_speaks_of_confirmed_or_refuted_any_more() {
        let dossier = a_dossier_with_four_checks();
        let rendered: String = dossier
            .checks()
            .map(|(id, record)| format_check_card(id, record, true))
            .collect();
        assert!(!rendered.contains("CONFIRMADO"));
        assert!(!rendered.contains("REFUTADO"));
        assert!(!rendered.contains("DISCREPANCIA"));
        assert!(rendered.contains("\x1b[32m[CONFORME]\x1b[0m"));
        assert!(rendered.contains("\x1b[31m[NO CONFORME]\x1b[0m"));
        assert!(rendered.contains("\x1b[33m[NO OBSERVABLE]\x1b[0m"));
        assert!(rendered.contains("\x1b[90m[PENDIENTE]\x1b[0m"));
    }

    #[test]
    fn the_listing_shows_one_summary_and_groups_by_suite() {
        let dossier = a_dossier_with_four_checks();
        let catalogue = crate::catalogue::read_the_catalogue().unwrap();
        let output = format_list(&dossier, &catalogue, None, false);
        assert!(output.contains("tanda del "));
        assert!(output.contains("sujeto 1.9.2, transporte websocket, almacén softhsm2"));
        assert_eq!(output.matches("Comprobaciones:").count(), 1);
    }

    #[test]
    fn the_listing_of_one_suite_leaves_the_others_out() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let catalogue = crate::catalogue::read_the_catalogue().unwrap();
        let coordinates = HeaderCoordinates {
            os: "Linux".to_owned(),
            os_version: "6.0".to_owned(),
            subject_version: "1.9.2".to_owned(),
            transport: "websocket".to_owned(),
            store: "softhsm2".to_owned(),
        };
        let dossier = Dossier::open(&path, "autofirma", &catalogue, Some(coordinates)).unwrap();

        let output = format_list(&dossier, &catalogue, Some("transporte.websocket"), false);

        assert!(output.contains("Conjunto transporte.websocket: 11 total"));
        assert!(output.contains("v4_echo_greeting"));
        assert!(!output.contains("invalid_parameters_syntax_rejected"));
    }
}
