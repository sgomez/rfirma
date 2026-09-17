//! La traducción de lo observado en cada caso a un veredicto, y el listado que las muestra.

use crate::dossier::{
    CaseRecord, CaseState, Dossier, ProtocolRecord, ProtocolState, ProtocolVerdict, Verdict,
};
use crate::errand::{ErrandOutcome, THE_DRIVER_CRASH, THE_EXHAUSTED_PATIENCE};

/// La excepción con la que el cliente publicado reporta la cancelación de la operación.
pub(crate) const THE_CANCELLED_OPERATION_EXCEPTION: &str =
    "es.gob.afirma.core.AOCancelledOperationException";

/// Lo que deja un caso al correr: un veredicto, con su observación si la hubo, o nada si se
/// quedó sin respuesta y hay que seguir ofreciéndolo.
pub(crate) enum CaseOutcome {
    Resolved {
        verdict: Verdict,
        observation: Option<String>,
    },
    StillPending,
}

impl CaseOutcome {
    pub(crate) fn confirmed() -> Self {
        Self::Resolved {
            verdict: Verdict::Confirmed,
            observation: None,
        }
    }

    pub(crate) fn resolved(verdict: Verdict) -> Self {
        Self::Resolved {
            verdict,
            observation: None,
        }
    }
}

/// La etiqueta en castellano de un veredicto, la que ve quien lee el listado.
#[allow(dead_code)]
fn verdict_label(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Confirmed => "confirmado",
        Verdict::Refuted => "refutado",
        Verdict::NotObservable => "no observable",
    }
}

/// La etiqueta en castellano del resultado de una condición del protocolo.
#[allow(dead_code)]
pub(crate) fn protocol_verdict_label(verdict: ProtocolVerdict) -> &'static str {
    match verdict {
        ProtocolVerdict::Compliant => "conforme",
        ProtocolVerdict::Discrepant => "discrepancia",
        ProtocolVerdict::NotObservable => "no observable",
    }
}

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
pub(crate) struct DivergenceSummary {
    pub total: usize,
    pub confirmed: usize,
    pub refuted: usize,
    pub not_observable: usize,
    pub pending: usize,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProtocolSummary {
    pub total: usize,
    pub compliant: usize,
    pub discrepant: usize,
    pub not_observable: usize,
    pub pending: usize,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DossierSummary {
    pub divergence: DivergenceSummary,
    pub protocol: ProtocolSummary,
}

impl DossierSummary {
    pub(crate) fn from_dossier(dossier: &Dossier) -> Self {
        Self {
            divergence: calculate_divergence_summary(dossier),
            protocol: calculate_protocol_summary(dossier),
        }
    }
}

pub(crate) fn calculate_divergence_summary(dossier: &Dossier) -> DivergenceSummary {
    let mut summary = DivergenceSummary::default();
    for (_case, record) in dossier.cases() {
        summary.total += 1;
        match record.state {
            CaseState::Pending => summary.pending += 1,
            CaseState::Resolved(Verdict::Confirmed) => summary.confirmed += 1,
            CaseState::Resolved(Verdict::Refuted) => summary.refuted += 1,
            CaseState::Resolved(Verdict::NotObservable) => summary.not_observable += 1,
        }
    }
    summary
}

pub(crate) fn calculate_protocol_summary(dossier: &Dossier) -> ProtocolSummary {
    let mut summary = ProtocolSummary::default();
    for (_id, record) in dossier.protocol_conditions() {
        summary.total += 1;
        match record.state {
            ProtocolState::Pending => summary.pending += 1,
            ProtocolState::Resolved(ProtocolVerdict::Compliant) => summary.compliant += 1,
            ProtocolState::Resolved(ProtocolVerdict::Discrepant) => summary.discrepant += 1,
            ProtocolState::Resolved(ProtocolVerdict::NotObservable) => summary.not_observable += 1,
        }
    }
    summary
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

pub(crate) fn format_case_card(case: &str, record: &CaseRecord, use_color: bool) -> String {
    let (badge_text, color) = match record.state {
        CaseState::Resolved(Verdict::Confirmed) => ("[CONFIRMADO]", GREEN),
        CaseState::Resolved(Verdict::Refuted) => ("[REFUTADO]", RED),
        CaseState::Resolved(Verdict::NotObservable) => ("[NO OBSERVABLE]", YELLOW),
        CaseState::Pending => ("[PENDIENTE]", GRAY),
    };
    let badge_formatted = format_badge(badge_text, color, use_color);
    let mut lines = vec![format!("{badge_formatted} {case}")];

    if let CaseState::Resolved(_) = record.state {
        if let Some(ref date) = record.date {
            let trimmed = date.trim();
            if !trimmed.is_empty() && trimmed != "-" {
                lines.push(format!("{PREFIX_FECHA}{trimmed}"));
            }
        }
    }

    if let Some(ref obs) = record.observation {
        let trimmed = obs.trim();
        if !trimmed.is_empty() && trimmed != "-" {
            lines.push(format!("{PREFIX_OBSERVACION}{trimmed}"));
        }
    }

    lines.join("\n")
}

pub(crate) fn format_protocol_card(id: &str, record: &ProtocolRecord, use_color: bool) -> String {
    let (badge_text, color) = match record.state {
        ProtocolState::Resolved(ProtocolVerdict::Compliant) => ("[CONFORME]", GREEN),
        ProtocolState::Resolved(ProtocolVerdict::Discrepant) => ("[DISCREPANCIA]", RED),
        ProtocolState::Resolved(ProtocolVerdict::NotObservable) => ("[NO OBSERVABLE]", YELLOW),
        ProtocolState::Pending => ("[PENDIENTE]", GRAY),
    };
    let badge_formatted = format_badge(badge_text, color, use_color);
    let chapter_tag = if record.chapter.starts_with('[') {
        record.chapter.clone()
    } else if record.chapter.starts_with("Cap.") {
        format!("[{}]", record.chapter)
    } else {
        format!("[Cap. {}]", record.chapter)
    };
    let mut lines = vec![format!("{badge_formatted} {chapter_tag} {id}")];

    if let ProtocolState::Resolved(_) = record.state {
        if let Some(ref date) = record.date {
            let trimmed = date.trim();
            if !trimmed.is_empty() && trimmed != "-" {
                lines.push(format!("{PREFIX_FECHA}{trimmed}"));
            }
        }
    }

    let statement = record.statement.trim();
    if !statement.is_empty() && statement != "-" {
        lines.push(format!("{PREFIX_ENUNCIADO}{statement}"));
    }

    let citation = record.citation.trim();
    if !citation.is_empty() && citation != "-" {
        lines.push(format!("{PREFIX_CITA}{citation}"));
    }

    if let Some(ref obs) = record.observation {
        let trimmed = obs.trim();
        if !trimmed.is_empty() && trimmed != "-" {
            lines.push(format!("{PREFIX_OBSERVACION}{trimmed}"));
        }
    }

    lines.join("\n")
}

pub(crate) fn format_list(dossier: &Dossier, use_color: bool) -> String {
    let header = dossier.header();
    let summary = DossierSummary::from_dossier(dossier);

    let mut out = String::new();
    out.push_str(&format!(
        "tanda del {}: {} {}, sujeto {}, transporte {}, almacén {}\n\n",
        header.date,
        header.os,
        header.os_version,
        header.subject_version,
        header.transport,
        header.store
    ));

    out.push_str(&format!(
        "Casos de divergencia: {} total, {} confirmados, {} refutados, {} no observables, {} pendientes\n",
        summary.divergence.total,
        summary.divergence.confirmed,
        summary.divergence.refuted,
        summary.divergence.not_observable,
        summary.divergence.pending
    ));
    out.push_str(&format!(
        "Condiciones de protocolo: {} total, {} conformes, {} discrepancias, {} no observables, {} pendientes\n\n",
        summary.protocol.total,
        summary.protocol.compliant,
        summary.protocol.discrepant,
        summary.protocol.not_observable,
        summary.protocol.pending
    ));

    out.push_str("── Casos de divergencia ──\n");
    let case_cards: Vec<String> = dossier
        .cases()
        .map(|(case, record)| format_case_card(case, record, use_color))
        .collect();
    if !case_cards.is_empty() {
        out.push('\n');
        out.push_str(&case_cards.join("\n\n"));
        out.push('\n');
    }

    out.push_str("\n── Condiciones de protocolo ──\n");
    let protocol_cards: Vec<String> = dossier
        .protocol_conditions()
        .map(|(id, record)| format_protocol_card(id, record, use_color))
        .collect();
    if !protocol_cards.is_empty() {
        out.push('\n');
        out.push_str(&protocol_cards.join("\n\n"));
        out.push('\n');
    }

    out
}

pub(crate) fn list(dossier: &Dossier) {
    use std::io::IsTerminal;
    let use_color = std::io::stdout().is_terminal();
    print!("{}", format_list(dossier, use_color));
}

/// El veredicto de una ficha que se juega a un código SAF concreto: sin código en el cable no
/// hay nada que afirmar, y el caso sale no observable en vez de refutado.
pub(crate) fn the_verdict_for_saf_code(outcome: ErrandOutcome, expected: &str) -> CaseOutcome {
    if !outcome.launched {
        return CaseOutcome::resolved(Verdict::NotObservable);
    }
    match outcome.error_code {
        Some(code) => CaseOutcome::Resolved {
            verdict: if code == expected {
                Verdict::Confirmed
            } else {
                Verdict::Refuted
            },
            observation: Some(code),
        },
        None => CaseOutcome::Resolved {
            verdict: Verdict::NotObservable,
            observation: outcome.error_type,
        },
    }
}

/// El veredicto del caso de divergencia que mide si `selectcert` exige clave privada: AutoFirma
/// cancela al requerir PIN sin respuesta (confirmado); rFirma devuelve el certificado sin sesión
/// ni comprobar clave privada (refutado).
pub(crate) fn the_verdict_for_private_key_check(outcome: ErrandOutcome) -> CaseOutcome {
    if !outcome.launched {
        return CaseOutcome::resolved(Verdict::NotObservable);
    }
    if outcome.data.is_some() || outcome.signature.is_some() {
        return CaseOutcome::Resolved {
            verdict: Verdict::Refuted,
            observation: Some("certificado devuelto sin comprobar clave privada".to_owned()),
        };
    }
    match outcome.error_type.as_deref() {
        Some(THE_CANCELLED_OPERATION_EXCEPTION) => CaseOutcome::Resolved {
            verdict: Verdict::Confirmed,
            observation: Some("operación cancelada al exigir clave privada".to_owned()),
        },
        Some(THE_DRIVER_CRASH | THE_EXHAUSTED_PATIENCE) | None => {
            CaseOutcome::resolved(Verdict::NotObservable)
        }
        Some(_) => CaseOutcome::Resolved {
            verdict: Verdict::NotObservable,
            observation: outcome.error_type,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dossier::{HeaderCoordinates, ProtocolConditionDefinition};

    #[test]
    fn a_missing_saf_code_is_not_observable_rather_than_refuted() {
        let outcome = ErrandOutcome {
            launched: true,
            error_type: Some(THE_DRIVER_CRASH.to_owned()),
            error_code: None,
            signature: None,
            data: None,
            protocol_conditions: Vec::new(),
            recent_subject_lines: Vec::new(),
        };
        let CaseOutcome::Resolved { verdict, .. } = the_verdict_for_saf_code(outcome, "SAF_47")
        else {
            panic!("el caso debería resolverse");
        };
        assert_eq!(verdict, Verdict::NotObservable);
    }

    #[test]
    fn the_expected_saf_code_confirms_the_case() {
        let outcome = ErrandOutcome {
            launched: true,
            error_type: Some("java.lang.Exception".to_owned()),
            error_code: Some("SAF_47".to_owned()),
            signature: None,
            data: None,
            protocol_conditions: Vec::new(),
            recent_subject_lines: Vec::new(),
        };
        let CaseOutcome::Resolved { verdict, .. } = the_verdict_for_saf_code(outcome, "SAF_47")
        else {
            panic!("el caso debería resolverse");
        };
        assert_eq!(verdict, Verdict::Confirmed);
    }

    #[test]
    fn another_saf_code_refutes_the_case() {
        let outcome = ErrandOutcome {
            launched: true,
            error_type: Some("java.lang.Exception".to_owned()),
            error_code: Some("SAF_03".to_owned()),
            signature: None,
            data: None,
            protocol_conditions: Vec::new(),
            recent_subject_lines: Vec::new(),
        };
        let CaseOutcome::Resolved { verdict, .. } = the_verdict_for_saf_code(outcome, "SAF_47")
        else {
            panic!("el caso debería resolverse");
        };
        assert_eq!(verdict, Verdict::Refuted);
    }

    #[test]
    fn private_key_check_confirmed_on_cancelled_operation() {
        let outcome = ErrandOutcome {
            launched: true,
            error_type: Some(THE_CANCELLED_OPERATION_EXCEPTION.to_owned()),
            error_code: None,
            signature: None,
            data: None,
            protocol_conditions: Vec::new(),
            recent_subject_lines: Vec::new(),
        };
        let CaseOutcome::Resolved {
            verdict,
            observation,
        } = the_verdict_for_private_key_check(outcome)
        else {
            panic!("el caso debería resolverse");
        };
        assert_eq!(verdict, Verdict::Confirmed);
        assert_eq!(
            observation.as_deref(),
            Some("operación cancelada al exigir clave privada")
        );
    }

    #[test]
    fn private_key_check_refuted_when_certificate_data_returned() {
        let outcome = ErrandOutcome {
            launched: true,
            error_type: None,
            error_code: None,
            signature: None,
            data: Some("MIID...".to_owned()),
            protocol_conditions: Vec::new(),
            recent_subject_lines: Vec::new(),
        };
        let CaseOutcome::Resolved {
            verdict,
            observation,
        } = the_verdict_for_private_key_check(outcome)
        else {
            panic!("el caso debería resolverse");
        };
        assert_eq!(verdict, Verdict::Refuted);
        assert_eq!(
            observation.as_deref(),
            Some("certificado devuelto sin comprobar clave privada")
        );
    }

    #[test]
    fn private_key_check_not_observable_when_unlaunched_or_crashed() {
        let unlaunched = ErrandOutcome {
            launched: false,
            error_type: None,
            error_code: None,
            signature: None,
            data: None,
            protocol_conditions: Vec::new(),
            recent_subject_lines: Vec::new(),
        };
        let CaseOutcome::Resolved { verdict, .. } = the_verdict_for_private_key_check(unlaunched)
        else {
            panic!("el caso debería resolverse");
        };
        assert_eq!(verdict, Verdict::NotObservable);

        let crashed = ErrandOutcome {
            launched: true,
            error_type: Some(THE_DRIVER_CRASH.to_owned()),
            error_code: None,
            signature: None,
            data: None,
            protocol_conditions: Vec::new(),
            recent_subject_lines: Vec::new(),
        };
        let CaseOutcome::Resolved { verdict, .. } = the_verdict_for_private_key_check(crashed)
        else {
            panic!("el caso debería resolverse");
        };
        assert_eq!(verdict, Verdict::NotObservable);
    }

    #[test]
    fn protocol_verdict_labels_match_spanish_vocabulary() {
        assert_eq!(
            protocol_verdict_label(ProtocolVerdict::Compliant),
            "conforme"
        );
        assert_eq!(
            protocol_verdict_label(ProtocolVerdict::Discrepant),
            "discrepancia"
        );
        assert_eq!(
            protocol_verdict_label(ProtocolVerdict::NotObservable),
            "no observable"
        );
    }

    #[test]
    fn summary_calculation_counts_all_states_accurately() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let coordinates = HeaderCoordinates {
            os: "Linux".to_owned(),
            os_version: "6.0".to_owned(),
            subject_version: "1.9.2".to_owned(),
            transport: "websocket".to_owned(),
            store: "softhsm2".to_owned(),
        };
        let cond1 = ProtocolConditionDefinition {
            id: "cond1",
            chapter: "05",
            citation: "file1.java:1",
            statement: "statement 1",
        };
        let cond2 = ProtocolConditionDefinition {
            id: "cond2",
            chapter: "14",
            citation: "file2.java:2",
            statement: "statement 2",
        };
        let cond3 = ProtocolConditionDefinition {
            id: "cond3",
            chapter: "15",
            citation: "file3.java:3",
            statement: "statement 3",
        };
        let cond4 = ProtocolConditionDefinition {
            id: "cond4",
            chapter: "15",
            citation: "file4.java:4",
            statement: "statement 4",
        };

        let mut dossier = Dossier::open(
            &path,
            "autofirma",
            &["case_a", "case_b", "case_c", "case_d"],
            &[cond1, cond2, cond3, cond4],
            Some(coordinates),
        )
        .unwrap();

        dossier
            .resolve("case_a", Verdict::Confirmed, Some("obs a".to_owned()))
            .unwrap();
        dossier.resolve("case_b", Verdict::Refuted, None).unwrap();
        dossier
            .resolve("case_c", Verdict::NotObservable, None)
            .unwrap();
        // case_d queda Pending

        dossier
            .resolve_protocol("cond1", ProtocolVerdict::Compliant, Some("ok".to_owned()))
            .unwrap();
        dossier
            .resolve_protocol("cond2", ProtocolVerdict::Discrepant, None)
            .unwrap();
        dossier
            .resolve_protocol("cond3", ProtocolVerdict::NotObservable, None)
            .unwrap();
        // cond4 queda Pending

        let summary = DossierSummary::from_dossier(&dossier);
        assert_eq!(summary.divergence.total, 4);
        assert_eq!(summary.divergence.confirmed, 1);
        assert_eq!(summary.divergence.refuted, 1);
        assert_eq!(summary.divergence.not_observable, 1);
        assert_eq!(summary.divergence.pending, 1);

        assert_eq!(summary.protocol.total, 4);
        assert_eq!(summary.protocol.compliant, 1);
        assert_eq!(summary.protocol.discrepant, 1);
        assert_eq!(summary.protocol.not_observable, 1);
        assert_eq!(summary.protocol.pending, 1);
    }

    #[test]
    fn format_divergence_card_confirmed_with_date_and_observation() {
        let record = CaseRecord {
            state: CaseState::Resolved(Verdict::Confirmed),
            date: Some("2026-09-17".to_string()),
            observation: Some("la firma salió sin sello de tiempo y sin error".to_string()),
        };
        let formatted = format_case_card(
            "a_broken_tsa_url_returns_an_unstamped_signature_without_a_warning",
            &record,
            false,
        );
        let expected = "[CONFIRMADO]    a_broken_tsa_url_returns_an_unstamped_signature_without_a_warning\n  Fecha:        2026-09-17\n  Observación:  la firma salió sin sello de tiempo y sin error";
        assert_eq!(formatted, expected);
    }

    #[test]
    fn format_divergence_card_not_observable_omits_observation() {
        let record = CaseRecord {
            state: CaseState::Resolved(Verdict::NotObservable),
            date: Some("2026-09-17".to_string()),
            observation: None,
        };
        let formatted = format_case_card(
            "obsolete_and_unsupported_protocol_share_error_code",
            &record,
            false,
        );
        let expected = "[NO OBSERVABLE] obsolete_and_unsupported_protocol_share_error_code\n  Fecha:        2026-09-17";
        assert_eq!(formatted, expected);
    }

    #[test]
    fn format_divergence_card_pending_omits_empty_fields_and_hyphens() {
        let record = CaseRecord {
            state: CaseState::Pending,
            date: None,
            observation: None,
        };
        let formatted = format_case_card("selectcert_checks_private_key", &record, false);
        assert_eq!(formatted, "[PENDIENTE]     selectcert_checks_private_key");
        assert!(!formatted.contains('-'));
        assert!(!formatted.contains("Fecha"));
        assert!(!formatted.contains("Observación"));
    }

    #[test]
    fn format_divergence_card_refuted() {
        let record = CaseRecord {
            state: CaseState::Resolved(Verdict::Refuted),
            date: Some("2026-09-17".to_string()),
            observation: Some("error refutado".to_string()),
        };
        let formatted = format_case_card("case_refuted", &record, false);
        assert!(formatted.starts_with("[REFUTADO]      case_refuted"));
    }

    #[test]
    fn format_protocol_card_compliant_with_all_fields() {
        let record = ProtocolRecord {
            chapter: "15".to_string(),
            citation: "ProtocolInvocationLauncher.java:741".to_string(),
            statement: "Parámetros de entrada con sintaxis inválida se rechazan con SAF_03."
                .to_string(),
            state: ProtocolState::Resolved(ProtocolVerdict::Compliant),
            date: Some("2026-09-17".to_string()),
            observation: Some("SAF_03".to_string()),
        };
        let formatted = format_protocol_card("invalid_parameters_syntax_rejected", &record, false);
        let expected = "[CONFORME]      [Cap. 15] invalid_parameters_syntax_rejected\n  Fecha:        2026-09-17\n  Enunciado:    Parámetros de entrada con sintaxis inválida se rechazan con SAF_03.\n  Cita:         ProtocolInvocationLauncher.java:741\n  Observación:  SAF_03";
        assert_eq!(formatted, expected);
    }

    #[test]
    fn format_protocol_card_pending_omits_date_and_observation_and_hyphens() {
        let record = ProtocolRecord {
            chapter: "14".to_string(),
            citation: "Version.java:120-169; ProtocolInvocationLauncherSign.java:143-150; ProtocolInvocationLauncherErrorManager.java:72".to_string(),
            statement: "Si mcv solicita una versión superior a la de la aplicación (p. ej. mcv=99.0.0), la operación es rechazada con SAF_41.".to_string(),
            state: ProtocolState::Pending,
            date: None,
            observation: None,
        };
        let formatted = format_protocol_card(
            "operation_minimum_client_version_unsatisfied_rejected",
            &record,
            false,
        );
        let expected = "[PENDIENTE]     [Cap. 14] operation_minimum_client_version_unsatisfied_rejected\n  Enunciado:    Si mcv solicita una versión superior a la de la aplicación (p. ej. mcv=99.0.0), la operación es rechazada con SAF_41.\n  Cita:         Version.java:120-169; ProtocolInvocationLauncherSign.java:143-150; ProtocolInvocationLauncherErrorManager.java:72";
        assert_eq!(formatted, expected);
        assert!(!formatted.contains("Fecha"));
        assert!(!formatted.contains("Observación"));
        assert!(!formatted.contains(" - "));
    }

    #[test]
    fn format_protocol_card_discrepant_and_not_observable() {
        let discrepant = ProtocolRecord {
            chapter: "05".to_string(),
            citation: "AfirmaWebSocketServer.java:10".to_string(),
            statement: "statement".to_string(),
            state: ProtocolState::Resolved(ProtocolVerdict::Discrepant),
            date: Some("2026-09-17".to_string()),
            observation: Some("discrepancia observada".to_string()),
        };
        let f1 = format_protocol_card("v5_test", &discrepant, false);
        assert!(f1.starts_with("[DISCREPANCIA]  [Cap. 05] v5_test"));

        let not_obs = ProtocolRecord {
            chapter: "Cap. 05".to_string(),
            citation: "AfirmaWebSocketServer.java:10".to_string(),
            statement: "statement".to_string(),
            state: ProtocolState::Resolved(ProtocolVerdict::NotObservable),
            date: Some("2026-09-17".to_string()),
            observation: None,
        };
        let f2 = format_protocol_card("v5_test2", &not_obs, false);
        assert!(f2.starts_with("[NO OBSERVABLE] [Cap. 05] v5_test2"));
    }

    #[test]
    fn observation_with_hyphen_or_empty_is_omitted() {
        let record = CaseRecord {
            state: CaseState::Resolved(Verdict::Confirmed),
            date: Some("2026-09-17".to_string()),
            observation: Some("-".to_string()),
        };
        let formatted = format_case_card("hyphen_test", &record, false);
        assert!(!formatted.contains("Observación"));

        let proto_record = ProtocolRecord {
            chapter: "05".to_string(),
            citation: "Afirma.java:1".to_string(),
            statement: "statement".to_string(),
            state: ProtocolState::Resolved(ProtocolVerdict::Compliant),
            date: Some("2026-09-17".to_string()),
            observation: Some("-".to_string()),
        };
        let proto_formatted = format_protocol_card("proto_hyphen_test", &proto_record, false);
        assert!(!proto_formatted.contains("Observación"));
    }

    #[test]
    fn badges_employ_ansi_colors_when_enabled_and_clean_when_disabled() {
        let confirmed_case = CaseRecord {
            state: CaseState::Resolved(Verdict::Confirmed),
            date: None,
            observation: None,
        };
        let refuted_case = CaseRecord {
            state: CaseState::Resolved(Verdict::Refuted),
            date: None,
            observation: None,
        };
        let not_obs_case = CaseRecord {
            state: CaseState::Resolved(Verdict::NotObservable),
            date: None,
            observation: None,
        };
        let pending_case = CaseRecord {
            state: CaseState::Pending,
            date: None,
            observation: None,
        };

        let c_colored = format_case_card("case1", &confirmed_case, true);
        assert!(c_colored.contains("[32m[CONFIRMADO][0m"));

        let r_colored = format_case_card("case2", &refuted_case, true);
        assert!(r_colored.contains("[31m[REFUTADO][0m"));

        let n_colored = format_case_card("case3", &not_obs_case, true);
        assert!(n_colored.contains("[33m[NO OBSERVABLE][0m"));

        let p_colored = format_case_card("case4", &pending_case, true);
        assert!(p_colored.contains("[90m[PENDIENTE][0m"));

        let compliant_proto = ProtocolRecord {
            chapter: "15".to_string(),
            citation: "cite".to_string(),
            statement: "stmt".to_string(),
            state: ProtocolState::Resolved(ProtocolVerdict::Compliant),
            date: None,
            observation: None,
        };
        let discrepant_proto = ProtocolRecord {
            chapter: "15".to_string(),
            citation: "cite".to_string(),
            statement: "stmt".to_string(),
            state: ProtocolState::Resolved(ProtocolVerdict::Discrepant),
            date: None,
            observation: None,
        };

        let comp_colored = format_protocol_card("p1", &compliant_proto, true);
        assert!(comp_colored.contains("[32m[CONFORME][0m"));

        let disc_colored = format_protocol_card("p2", &discrepant_proto, true);
        assert!(disc_colored.contains("[31m[DISCREPANCIA][0m"));

        // When use_color is false, no ANSI escapes anywhere
        let c_plain = format_case_card("case1", &confirmed_case, false);
        assert!(!c_plain.contains(""));

        let comp_plain = format_protocol_card("p1", &compliant_proto, false);
        assert!(!comp_plain.contains(""));
    }

    #[test]
    fn format_list_divides_output_into_three_differentiated_blocks() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let coordinates = HeaderCoordinates {
            os: "Linux".to_owned(),
            os_version: "6.6.0".to_owned(),
            subject_version: "1.8.2".to_owned(),
            transport: "websocket".to_owned(),
            store: "software".to_owned(),
        };
        let cond = ProtocolConditionDefinition {
            id: "v4_ports_negotiation",
            chapter: "05",
            citation: "ProtocolInvocationLauncher.java:233-236",
            statement: "El cliente JavaScript selecciona 3 puertos aleatorios únicos",
        };
        let dossier = Dossier::open(
            &path,
            "autofirma",
            &["selectcert_checks_private_key"],
            &[cond],
            Some(coordinates),
        )
        .unwrap();

        let output = format_list(&dossier, false);
        assert!(output.contains("tanda del "));
        assert!(output.contains("sujeto 1.8.2, transporte websocket, almacén software"));
        assert!(output.contains(
            "Casos de divergencia: 1 total, 0 confirmados, 0 refutados, 0 no observables, 1 pendientes"
        ));
        assert!(output.contains(
            "Condiciones de protocolo: 1 total, 0 conformes, 0 discrepancias, 0 no observables, 1 pendientes"
        ));
        assert!(output.contains("── Casos de divergencia ──"));
        assert!(output.contains("[PENDIENTE]     selectcert_checks_private_key"));
        assert!(output.contains("── Condiciones de protocolo ──"));
        assert!(output.contains("[PENDIENTE]     [Cap. 05] v4_ports_negotiation"));
        assert!(!output.contains(""));
    }
}
