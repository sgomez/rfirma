//! La traducción de lo observado en cada comprobación a su veredicto, y el listado que las
//! muestra.

use std::io::IsTerminal;

use crate::baseline::{
    contrast_of, the_exit_code_of, the_expectation_of, the_surprises_of, verdict_name,
    BaselineTally, Contrast, Expectation,
};
use crate::catalogue::Check;
use crate::checks::the_reason_it_is_still_pending;
use crate::dossier::{CheckRecord, CheckState, Dossier, Verdict};
use crate::errand::{ErrandOutcome, THE_DRIVER_CRASH, THE_EXHAUSTED_PATIENCE};

/// La excepción con la que el cliente publicado reporta la cancelación de la operación.
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
    the_verdict_for_a_dialogue(
        outcome,
        answer,
        if outcome.data.is_some() {
            "se pidió el PIN del token y devolvió el certificado tras autenticación"
        } else {
            "se pidió el PIN del token y la operación se canceló"
        },
        "certificado devuelto sin pedir PIN ni comprobar clave privada",
    )
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

/// El veredicto del certificado fijado: quien está delante dice si se le pidió el certificado en
/// las selecciones en que el protocolo lo exige, y sólo en ellas.
pub(crate) fn the_verdict_for_a_pinned_certificate(
    outcome: &ErrandOutcome,
    answer: &str,
) -> CheckOutcome {
    the_verdict_for_a_dialogue(
        outcome,
        answer,
        "el certificado se pidió en la primera selección y tras soltarlo, y no mientras estuvo fijado",
        "el certificado fijado no se respetó o no se volvió a pedir al soltarlo",
    )
}

/// El veredicto del área de firma visible: quien está delante dice si se le pidió marcarla, que es
/// lo que `visibleSignature=want` exige antes de firmar.
pub(crate) fn the_verdict_for_a_visible_signature_area(
    outcome: &ErrandOutcome,
    answer: &str,
) -> CheckOutcome {
    the_verdict_for_a_dialogue(
        outcome,
        answer,
        "se pidió marcar el área de la firma visible",
        "la firma salió sin pedir el área que el protocolo exige marcar",
    )
}

/// El veredicto del guardado de una firma: quien está delante dice si se pidió destino, y el cable
/// dice si la firma volvió a la sede después de guardarla.
pub(crate) fn the_verdict_for_a_saved_signature(
    outcome: &ErrandOutcome,
    answer: &str,
) -> CheckOutcome {
    match the_answered_dialogue(outcome, answer, "no se pidió dónde guardar la firma") {
        Answered::Settled(outcome) => outcome,
        Answered::Asked => {
            if outcome.signature.is_some() {
                CheckOutcome::of(
                    Verdict::Compliant,
                    "se pidió destino y la firma volvió a la sede",
                )
            } else {
                CheckOutcome::of(
                    Verdict::Noncompliant,
                    "se pidió destino, pero la firma no volvió a la sede",
                )
            }
        }
    }
}

/// El veredicto del nombre de guardado: quien está delante dice si se le ofreció el que la petición
/// propuso.
pub(crate) fn the_verdict_for_a_proposed_save_name(
    outcome: &ErrandOutcome,
    answer: &str,
) -> CheckOutcome {
    the_verdict_for_a_dialogue(
        outcome,
        answer,
        "se ofreció para guardar el nombre que la petición propuso",
        "la firma se guardó con un nombre que la petición no propuso",
    )
}

/// El veredicto de la carga interactiva del documento: la petición llegó sin datos y quien está
/// delante dice si se le pidió el documento a firmar.
pub(crate) fn the_verdict_for_a_requested_input_document(
    outcome: &ErrandOutcome,
    answer: &str,
) -> CheckOutcome {
    the_verdict_for_a_dialogue(
        outcome,
        answer,
        "se pidió el documento a firmar antes de firmarlo",
        "la petición sin datos se resolvió sin pedir el documento a firmar",
    )
}

/// El veredicto de la confirmación de sobrescritura: quien está delante dice si se le preguntó
/// antes de escribir sobre un fichero que ya estaba.
pub(crate) fn the_verdict_for_an_overwrite_confirmation(
    outcome: &ErrandOutcome,
    answer: &str,
) -> CheckOutcome {
    the_verdict_for_a_dialogue(
        outcome,
        answer,
        "se pidió confirmación antes de escribir sobre el fichero que ya estaba",
        "se escribió sobre el fichero que ya estaba sin pedir confirmación",
    )
}

/// El veredicto de una cancelación: quien está delante dice si canceló, y el cable dice si la sede
/// recibió la cancelación como tal.
pub(crate) fn the_verdict_for_a_cancelled_dialogue(
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
    if !answered_yes(answer) {
        return CheckOutcome::of(Verdict::NotObservable, "no se llegó a cancelar el diálogo");
    }
    match outcome.error_type.as_deref() {
        Some(THE_CANCELLED_OPERATION_EXCEPTION) => CheckOutcome::of(
            Verdict::Compliant,
            "la sede recibió la cancelación como cancelación",
        ),
        Some(THE_DRIVER_CRASH | THE_EXHAUSTED_PATIENCE) => CheckOutcome::Resolved {
            verdict: Verdict::NotObservable,
            observation: outcome.error_type.clone(),
        },
        Some(other) => CheckOutcome::of(
            Verdict::Noncompliant,
            format!("se canceló y la sede recibió {other}"),
        ),
        None => CheckOutcome::of(
            Verdict::Noncompliant,
            "se canceló y la sede no recibió ninguna cancelación",
        ),
    }
}

/// El veredicto de una carga interactiva: quien está delante dice si se le pidieron los ficheros, y
/// el cable dice si la respuesta trajo cada nombre junto a su contenido.
pub(crate) fn the_verdict_for_an_interactive_load(
    check: &Check,
    outcome: &ErrandOutcome,
    answer: &str,
) -> CheckOutcome {
    match the_answered_dialogue(outcome, answer, "no se pidió elegir qué fichero cargar") {
        Answered::Settled(outcome) => outcome,
        Answered::Asked => the_verdict_of(check, outcome),
    }
}

/// El veredicto de la autoselección: con un único candidato, quien está delante dice que no se le
/// pidió elegir y el cable trae el certificado igualmente.
pub(crate) fn the_verdict_for_an_automatic_selection(
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
    if answered_yes(answer) {
        return CheckOutcome::of(
            Verdict::Noncompliant,
            "se pidió elegir certificado habiendo un único candidato",
        );
    }
    if outcome.data.is_some() {
        return CheckOutcome::of(
            Verdict::Compliant,
            "el único candidato se resolvió sin pedir que se eligiera",
        );
    }
    CheckOutcome::Resolved {
        verdict: Verdict::NotObservable,
        observation: outcome.error_type.clone(),
    }
}

/// Lo que queda de un diálogo antes de mirar el cable: o ya está resuelto, o quien está delante
/// contestó que sí y el cable tiene la última palabra.
enum Answered {
    Settled(CheckOutcome),
    Asked,
}

fn answered_yes(answer: &str) -> bool {
    answer.to_lowercase().starts_with('s')
}

fn the_answered_dialogue(outcome: &ErrandOutcome, answer: &str, unasked: &str) -> Answered {
    if !outcome.launched {
        return Answered::Settled(CheckOutcome::of(
            Verdict::NotObservable,
            "el sujeto no llegó a arrancar en esta tanda",
        ));
    }
    if let Some(THE_DRIVER_CRASH | THE_EXHAUSTED_PATIENCE) = outcome.error_type.as_deref() {
        return Answered::Settled(CheckOutcome::Resolved {
            verdict: Verdict::NotObservable,
            observation: outcome.error_type.clone(),
        });
    }
    if answer.is_empty() {
        return Answered::Settled(CheckOutcome::StillPending);
    }
    if !answered_yes(answer) {
        return Answered::Settled(CheckOutcome::of(Verdict::Noncompliant, unasked.to_owned()));
    }
    Answered::Asked
}

/// El veredicto de una exigencia que se juega a un diálogo: sin respuesta de quien está delante no
/// hay nada que afirmar, y sin trámite completado tampoco hay incumplimiento que declarar.
fn the_verdict_for_a_dialogue(
    outcome: &ErrandOutcome,
    answer: &str,
    seen: &str,
    unseen: &str,
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
        return CheckOutcome::of(Verdict::Compliant, seen);
    }
    if outcome.signature.is_some() || outcome.data.is_some() {
        return CheckOutcome::of(Verdict::Noncompliant, unseen);
    }
    CheckOutcome::Resolved {
        verdict: Verdict::NotObservable,
        observation: outcome.error_type.clone(),
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
    use crate::baseline::Profile;
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
    fn a_signature_saved_after_asking_for_a_destination_is_compliant() {
        let signed = ErrandOutcome {
            signature: Some("MIIB...".to_owned()),
            ..an_outcome()
        };
        assert_eq!(
            the_verdict(the_verdict_for_a_saved_signature(&signed, "s")),
            Verdict::Compliant
        );
    }

    #[test]
    fn a_signature_saved_that_never_came_back_to_the_site_is_noncompliant() {
        assert_eq!(
            the_verdict(the_verdict_for_a_saved_signature(&an_outcome(), "s")),
            Verdict::Noncompliant
        );
    }

    #[test]
    fn a_saved_signature_nobody_answered_stays_pending() {
        assert!(matches!(
            the_verdict_for_a_saved_signature(&an_outcome(), ""),
            CheckOutcome::StillPending
        ));
    }

    #[test]
    fn a_cancellation_the_site_received_as_such_is_compliant() {
        let cancelled = ErrandOutcome {
            error_type: Some(THE_CANCELLED_OPERATION_EXCEPTION.to_owned()),
            ..an_outcome()
        };
        assert_eq!(
            the_verdict(the_verdict_for_a_cancelled_dialogue(&cancelled, "s")),
            Verdict::Compliant
        );
    }

    #[test]
    fn a_cancellation_the_site_never_received_is_noncompliant() {
        assert_eq!(
            the_verdict(the_verdict_for_a_cancelled_dialogue(&an_outcome(), "s")),
            Verdict::Noncompliant
        );
    }

    #[test]
    fn a_dialogue_nobody_cancelled_is_not_observable() {
        assert_eq!(
            the_verdict(the_verdict_for_a_cancelled_dialogue(&an_outcome(), "n")),
            Verdict::NotObservable
        );
    }

    #[test]
    fn the_only_candidate_resolved_without_asking_is_compliant() {
        let returned = ErrandOutcome {
            data: Some("MIID...".to_owned()),
            ..an_outcome()
        };
        assert_eq!(
            the_verdict(the_verdict_for_an_automatic_selection(&returned, "n")),
            Verdict::Compliant
        );
    }

    #[test]
    fn asking_to_choose_with_a_single_candidate_is_noncompliant() {
        let returned = ErrandOutcome {
            data: Some("MIID...".to_owned()),
            ..an_outcome()
        };
        assert_eq!(
            the_verdict(the_verdict_for_an_automatic_selection(&returned, "s")),
            Verdict::Noncompliant
        );
    }

    #[test]
    fn an_interactive_load_that_was_never_asked_for_is_noncompliant() {
        let loaded = ErrandOutcome {
            data: Some("Y29udHJhdG8=".to_owned()),
            ..an_outcome()
        };
        assert_eq!(
            the_verdict(the_verdict_for_an_interactive_load(
                &a_check_expecting(None),
                &loaded,
                "n"
            )),
            Verdict::Noncompliant
        );
    }

    #[test]
    fn an_interactive_load_takes_the_condition_the_driver_measured() {
        let loaded = ErrandOutcome {
            protocol_conditions: vec![crate::errand::ProtocolConditionResult {
                id: "an_id".to_owned(),
                verdict: Verdict::Noncompliant,
                observation: Some("la respuesta no trajo el nombre junto al contenido".to_owned()),
            }],
            data: Some("Y29udHJhdG8=".to_owned()),
            ..an_outcome()
        };
        assert_eq!(
            the_verdict(the_verdict_for_an_interactive_load(
                &a_check_expecting(None),
                &loaded,
                "s"
            )),
            Verdict::Noncompliant
        );
    }

    #[test]
    fn an_interactive_load_nobody_answered_stays_pending() {
        assert!(matches!(
            the_verdict_for_an_interactive_load(&a_check_expecting(None), &an_outcome(), ""),
            CheckOutcome::StillPending
        ));
    }

    #[test]
    fn a_save_name_that_was_the_proposed_one_is_compliant() {
        let signed = ErrandOutcome {
            signature: Some("MIIB...".to_owned()),
            ..an_outcome()
        };
        assert_eq!(
            the_verdict(the_verdict_for_a_proposed_save_name(&signed, "s")),
            Verdict::Compliant
        );
    }

    #[test]
    fn a_document_never_asked_for_before_signing_is_noncompliant() {
        let signed = ErrandOutcome {
            signature: Some("MIIB...".to_owned()),
            ..an_outcome()
        };
        assert_eq!(
            the_verdict(the_verdict_for_a_requested_input_document(&signed, "n")),
            Verdict::Noncompliant
        );
    }

    #[test]
    fn an_overwrite_that_was_confirmed_first_is_compliant() {
        assert_eq!(
            the_verdict(the_verdict_for_an_overwrite_confirmation(
                &an_outcome(),
                "s"
            )),
            Verdict::Compliant
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

    #[test]
    fn a_certificate_asked_for_where_the_protocol_says_is_compliant() {
        let signed = ErrandOutcome {
            signature: Some("MIIF...".to_owned()),
            ..an_outcome()
        };
        assert_eq!(
            the_verdict(the_verdict_for_a_pinned_certificate(&signed, "s")),
            Verdict::Compliant
        );
    }

    #[test]
    fn a_pinned_certificate_that_was_asked_for_again_is_noncompliant() {
        let signed = ErrandOutcome {
            signature: Some("MIIF...".to_owned()),
            ..an_outcome()
        };
        assert_eq!(
            the_verdict(the_verdict_for_a_pinned_certificate(&signed, "n")),
            Verdict::Noncompliant
        );
    }

    #[test]
    fn a_pinned_certificate_nobody_answered_stays_pending() {
        assert!(matches!(
            the_verdict_for_a_pinned_certificate(&an_outcome(), ""),
            CheckOutcome::StillPending
        ));
    }

    #[test]
    fn a_dialogue_whose_errand_never_came_back_is_not_observable() {
        assert_eq!(
            the_verdict(the_verdict_for_a_pinned_certificate(&an_outcome(), "n")),
            Verdict::NotObservable
        );
    }

    #[test]
    fn a_visible_area_that_was_asked_for_is_compliant() {
        let signed = ErrandOutcome {
            signature: Some("JVBERi0...".to_owned()),
            ..an_outcome()
        };
        assert_eq!(
            the_verdict(the_verdict_for_a_visible_signature_area(&signed, "s")),
            Verdict::Compliant
        );
    }

    #[test]
    fn a_signature_that_skipped_the_visible_area_is_noncompliant() {
        let signed = ErrandOutcome {
            signature: Some("JVBERi0...".to_owned()),
            ..an_outcome()
        };
        let CheckOutcome::Resolved {
            verdict,
            observation,
        } = the_verdict_for_a_visible_signature_area(&signed, "n")
        else {
            panic!("la comprobación debería resolverse");
        };
        assert_eq!(verdict, Verdict::Noncompliant);
        assert_eq!(
            observation.as_deref(),
            Some("la firma salió sin pedir el área que el protocolo exige marcar")
        );
    }

    #[test]
    fn a_crashed_visible_area_check_is_not_observable() {
        let crashed = ErrandOutcome {
            error_type: Some(THE_DRIVER_CRASH.to_owned()),
            ..an_outcome()
        };
        assert_eq!(
            the_verdict(the_verdict_for_a_visible_signature_area(&crashed, "s")),
            Verdict::NotObservable
        );
    }
}
