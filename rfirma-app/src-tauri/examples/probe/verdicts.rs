//! La traducción de lo observado en cada caso a un veredicto, y el listado que las muestra.

use crate::dossier::{CaseState, Dossier, ProtocolState, ProtocolVerdict, Verdict};
use crate::errand::ErrandOutcome;

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
fn verdict_label(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Confirmed => "confirmado",
        Verdict::Refuted => "refutado",
        Verdict::NotObservable => "no observable",
    }
}

/// La etiqueta en castellano del resultado de una condición del protocolo.
pub(crate) fn protocol_verdict_label(verdict: ProtocolVerdict) -> &'static str {
    match verdict {
        ProtocolVerdict::Compliant => "conforme",
        ProtocolVerdict::Discrepant => "discrepancia",
        ProtocolVerdict::NotObservable => "no observable",
    }
}

pub(crate) fn list(dossier: &Dossier) {
    let header = dossier.header();
    println!(
        "tanda del {}: {} {}, sujeto {}, transporte {}, almacén {}",
        header.date,
        header.os,
        header.os_version,
        header.subject_version,
        header.transport,
        header.store
    );
    for (case, record) in dossier.cases() {
        let state = match record.state {
            CaseState::Pending => "pendiente",
            CaseState::Resolved(verdict) => verdict_label(verdict),
        };
        let date = record.date.as_deref().unwrap_or("-");
        let observation = record.observation.as_deref().unwrap_or("-");
        println!("{case}\t{state}\t{date}\t{observation}");
    }
    for (id, record) in dossier.protocol_conditions() {
        let state = match record.state {
            ProtocolState::Pending => "pendiente",
            ProtocolState::Resolved(verdict) => protocol_verdict_label(verdict),
        };
        let date = record.date.as_deref().unwrap_or("-");
        let observation = record.observation.as_deref().unwrap_or("-");
        println!(
            "[{}] {}\t{}\t{}\t{}\t{}\t{}",
            record.chapter, id, state, date, record.citation, record.statement, observation
        );
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errand::THE_DRIVER_CRASH;

    #[test]
    fn a_missing_saf_code_is_not_observable_rather_than_refuted() {
        let outcome = ErrandOutcome {
            launched: true,
            error_type: Some(THE_DRIVER_CRASH.to_owned()),
            error_code: None,
            signature: None,
            protocol_conditions: Vec::new(),
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
            protocol_conditions: Vec::new(),
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
            protocol_conditions: Vec::new(),
        };
        let CaseOutcome::Resolved { verdict, .. } = the_verdict_for_saf_code(outcome, "SAF_47")
        else {
            panic!("el caso debería resolverse");
        };
        assert_eq!(verdict, Verdict::Refuted);
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
}
