//! Cómo se presenta un veredicto: insignias, colores, tarjetas, listados y el cierre de una tanda.

use std::io::IsTerminal;

use crate::baseline::{
    contrast_of, the_exit_code_of, the_expectation_of, the_surprises_of, verdict_name,
    BaselineTally, Contrast, Expectation,
};
use crate::catalogue::Check;
use crate::checks::the_reason_it_is_still_pending;
use crate::dossier::{CheckRecord, CheckState, Dossier, Verdict};

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
const PREFIX_ESPERADO: &str = "  Esperado:     ";
const PREFIX_NOTA: &str = "  Nota:         ";

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

    pub(crate) fn conformance_line(&self) -> String {
        format!(
            "{} conformes · {} no conformes · {} no observables · {} pendientes",
            self.compliant, self.noncompliant, self.not_observable, self.pending
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

pub(crate) fn contrast_color(contrast: Contrast) -> &'static str {
    match contrast {
        Contrast::Matches => GREEN,
        Contrast::Surprise => RED,
        Contrast::Unmeasured => YELLOW,
    }
}

/// La línea de la línea base: qué se esperaba, por qué, y cómo cayó lo observado frente a ello.
fn the_expected_line(record: &CheckRecord, expectation: &Expectation, use_color: bool) -> String {
    let mut line = verdict_name(expectation.verdict).to_owned();
    if let Some(cause) = &expectation.cause {
        line.push_str(&format!(" ({cause})"));
    }
    if let CheckState::Resolved(observed) = record.state {
        let contrast = contrast_of(observed, expectation.verdict);
        let label = contrast.label();
        let shown = if use_color {
            format!("{}{label}{RESET}", contrast_color(contrast))
        } else {
            label.to_owned()
        };
        line.push_str(&format!(" — {shown}"));
    }
    line
}

pub(crate) fn format_check_card(
    id: &str,
    record: &CheckRecord,
    expectation: Option<&Expectation>,
    use_color: bool,
) -> String {
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
    if let Some(expectation) = expectation {
        lines.push(format!(
            "{PREFIX_ESPERADO}{}",
            the_expected_line(record, expectation, use_color)
        ));
        if let Some(note) = &expectation.note {
            push_field(&mut lines, PREFIX_NOTA, note);
        }
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
    let profile = dossier.profile();
    let summary = Summary::of(shown.iter().map(|(_, record)| *record));
    let tally = BaselineTally::of(
        shown
            .iter()
            .map(|(id, record)| (*record, the_expectation_of(catalogue, id, profile))),
    );

    let mut out = format!(
        "tanda del {}: {} {}, sujeto {} ({}), transporte {}, almacén {}\n\n",
        header.date,
        header.os,
        header.os_version,
        header.subject_version,
        profile.name(),
        header.transport,
        header.store
    );
    if let Some(wanted) = suite {
        out.push_str(&format!(
            "Conjunto {wanted}: {} comprobaciones\n",
            summary.total
        ));
    }
    out.push_str(&format!(
        "Conformidad del sujeto:  {}\n",
        summary.conformance_line()
    ));
    out.push_str(&format!("Frente a la línea base:  {}\n", tally.line()));

    let mut current_suite = None;
    for (id, record) in shown {
        if current_suite != Some(record.suite.as_str()) {
            out.push_str(&format!("\n── {} ──\n\n", record.suite));
            current_suite = Some(record.suite.as_str());
        }
        out.push_str(&format_check_card(
            id,
            record,
            the_expectation_of(catalogue, id, profile),
            use_color,
        ));
        out.push_str("\n\n");
    }

    out
}

/// El cierre de una tanda: las dos filas del resumen, las sorpresas nombradas una a una y el
/// código de salida en el que se traducen.
pub(crate) fn the_closing_of(
    dossier: &Dossier,
    catalogue: &[Check],
    suite: Option<&str>,
) -> (String, i32) {
    let profile = dossier.profile();
    let shown: Vec<(&str, &CheckRecord)> = catalogue
        .iter()
        .filter(|check| suite.is_none_or(|wanted| check.suite == wanted))
        .filter_map(|check| dossier.checks().find(|(id, _)| *id == check.id))
        .collect();
    let summary = Summary::of(shown.iter().map(|(_, record)| *record));
    let tally = BaselineTally::of(
        shown
            .iter()
            .map(|(id, record)| (*record, the_expectation_of(catalogue, id, profile))),
    );
    let surprises = the_surprises_of(
        shown
            .iter()
            .map(|(id, record)| (*id, *record, the_expectation_of(catalogue, id, profile))),
    );

    let mut out = format!(
        "\nConformidad del sujeto:  {}\nFrente a la línea base:  {}\n",
        summary.conformance_line(),
        tally.line()
    );
    if surprises.is_empty() {
        out.push_str("\nSin sorpresas frente a la línea base.\n");
    } else {
        out.push_str("\nSorpresas frente a la línea base:\n");
        for surprise in &surprises {
            out.push_str(&format!("  {surprise}\n"));
        }
    }
    out.push_str(&the_pending_section_of(dossier, catalogue, suite));
    (out, the_exit_code_of(&tally))
}

/// Lo que quedó pendiente al cierre, con el motivo por el que se saltó cada una.
fn the_pending_section_of(dossier: &Dossier, catalogue: &[Check], suite: Option<&str>) -> String {
    let declared_store = &dossier.header().store;
    let terminal = std::io::stdin().is_terminal();
    let pending: Vec<String> = catalogue
        .iter()
        .filter(|check| suite.is_none_or(|wanted| check.suite == wanted))
        .filter(|check| matches!(dossier.state_of(&check.id), Some(CheckState::Pending)))
        .map(|check| {
            format!(
                "  {}: {}",
                check.id,
                the_reason_it_is_still_pending(check, declared_store, terminal)
            )
        })
        .collect();
    if pending.is_empty() {
        String::new()
    } else {
        format!("\nPendientes:\n{}\n", pending.join("\n"))
    }
}

pub(crate) fn list(dossier: &Dossier, catalogue: &[Check], suite: Option<&str>) {
    let use_color = std::io::stdout().is_terminal();
    print!("{}", format_list(dossier, catalogue, suite, use_color));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baseline::Profile;
    use crate::catalogue::the_catalogue_in;
    use crate::dossier::HeaderCoordinates;

    #[test]
    fn the_summary_counts_every_state_once() {
        let dossier = a_dossier_with_four_checks();
        let summary = Summary::of(dossier.checks().map(|(_, record)| record));
        assert_eq!(
            summary.conformance_line(),
            "1 conformes · 1 no conformes · 1 no observables · 1 pendientes"
        );
    }

    #[test]
    fn the_closing_names_each_surprise_and_breaks_the_exit_code() {
        let dossier = a_dossier_with_four_checks();
        let catalogue = the_catalogue_in(FOUR_CHECKS).unwrap();

        let (closing, code) = the_closing_of(&dossier, &catalogue, None);

        assert!(closing.contains("Conformidad del sujeto:  1 conformes"));
        assert!(closing.contains(
            "Frente a la línea base:  1 coinciden · 1 SORPRESA · 1 sin medida · 1 pendientes"
        ));
        assert!(closing.contains("a_two: se esperaba CONFORME y salió NO CONFORME"));
        assert!(closing.contains("Pendientes:"));
        assert!(closing.contains("a_four: no hubo respuesta la última vez"));
        assert_eq!(code, 1);
    }

    #[test]
    fn the_closing_names_the_store_a_pending_check_is_missing() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let catalogue = the_catalogue_in(
            r#"
[[check]]
id = "needs_the_ecc_store"
suite = "operaciones"
chapter = "07"
citation = "A.java:1"
statement = "Algo."
drive = { mode = "v4", script = "selectcert" }
needs = ["almacén:rfirma-test-ecc"]

[check.expect.autofirma]
verdict = "conforme"
"#,
        )
        .unwrap();
        let coordinates = HeaderCoordinates {
            os: "Linux".to_owned(),
            os_version: "6.0".to_owned(),
            subject_version: "1.9.2".to_owned(),
            transport: "websocket".to_owned(),
            store: "rfirma-test".to_owned(),
        };
        let dossier = Dossier::open(
            &path,
            "autofirma",
            Profile::Autofirma,
            &catalogue,
            Some(coordinates),
        )
        .unwrap();

        let (closing, _) = the_closing_of(&dossier, &catalogue, None);

        assert!(closing.contains("needs_the_ecc_store"));
        assert!(closing.contains(
            "just conformance --dossier <expediente-nuevo> --store rfirma-test-ecc run \
             needs_the_ecc_store"
        ));
    }

    const FOUR_CHECKS: &str = r#"
[[check]]
id = "a_one"
suite = "errores"
chapter = "15"
citation = "ProtocolInvocationLauncher.java:741"
statement = "Uno."
drive = { mode = "v4", script = "selectcert" }

[check.expect.autofirma]
verdict = "conforme"

[[check]]
id = "a_two"
suite = "errores"
chapter = "15"
citation = "ProtocolInvocationLauncher.java:741"
statement = "Dos."
drive = { mode = "v4", script = "selectcert" }

[check.expect.autofirma]
verdict = "conforme"

[[check]]
id = "a_three"
suite = "versiones"
chapter = "14"
citation = "ProtocolVersion.java:60-62"
statement = "Tres."
drive = { mode = "v4", script = "selectcert" }

[check.expect.autofirma]
verdict = "conforme"

[[check]]
id = "a_four"
suite = "versiones"
chapter = "14"
citation = "ProtocolVersion.java:60-62"
statement = "Cuatro."
drive = { mode = "v4", script = "selectcert" }

[check.expect.autofirma]
verdict = "conforme"
"#;

    fn a_dossier_with_four_checks() -> Dossier {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let catalogue = the_catalogue_in(FOUR_CHECKS).unwrap();
        let coordinates = HeaderCoordinates {
            os: "Linux".to_owned(),
            os_version: "6.0".to_owned(),
            subject_version: "1.9.2".to_owned(),
            transport: "websocket".to_owned(),
            store: "softhsm2".to_owned(),
        };
        let mut dossier = Dossier::open(
            &path,
            "autofirma",
            Profile::Autofirma,
            &catalogue,
            Some(coordinates),
        )
        .unwrap();
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
        let expectation = Expectation {
            verdict: Verdict::Noncompliant,
            cause: Some("BUG-15".to_owned()),
            note: Some("Revienta antes de validar.".to_owned()),
        };
        let formatted = format_check_card(
            "invalid_parameters_syntax_rejected",
            &record,
            Some(&expectation),
            false,
        );
        assert_eq!(
            formatted,
            "[CONFORME]      [Cap. 15] invalid_parameters_syntax_rejected\n  Fecha:        2026-09-17\n  Enunciado:    Parámetros de entrada con sintaxis inválida se rechazan con SAF_03.\n  Cita:         ProtocolInvocationLauncher.java:741\n  Observación:  SAF_03\n  Esperado:     NO CONFORME (BUG-15) — SORPRESA\n  Nota:         Revienta antes de validar."
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
            Some(&Expectation {
                verdict: Verdict::Compliant,
                cause: None,
                note: None,
            }),
            false,
        );
        assert!(formatted.starts_with(
            "[PENDIENTE]     [Cap. 09] selectcert_only_offers_certificates_with_a_private_key"
        ));
        assert!(!formatted.contains("Fecha"));
        assert!(!formatted.contains("Observación"));
        assert!(formatted.ends_with("  Esperado:     CONFORME"));
        assert!(!formatted.contains("coincide"));
    }

    #[test]
    fn no_card_speaks_of_confirmed_or_refuted_any_more() {
        let dossier = a_dossier_with_four_checks();
        let rendered: String = dossier
            .checks()
            .map(|(id, record)| format_check_card(id, record, None, true))
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
        assert!(output.contains("sujeto 1.9.2 (autofirma), transporte websocket, almacén softhsm2"));
        assert_eq!(output.matches("Conformidad del sujeto:").count(), 1);
        assert_eq!(output.matches("Frente a la línea base:").count(), 1);
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
        let dossier = Dossier::open(
            &path,
            "autofirma",
            Profile::Autofirma,
            &catalogue,
            Some(coordinates),
        )
        .unwrap();

        let output = format_list(&dossier, &catalogue, Some("transporte.websocket"), false);

        assert!(output.contains("Conjunto transporte.websocket: 11 comprobaciones"));
        assert!(output.contains("v4_echo_greeting"));
        assert!(!output.contains("invalid_parameters_syntax_rejected"));
    }
}
