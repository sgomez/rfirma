//! Las etiquetas de una comprobación, lo que puede explicar su NO CONFORME igual en cualquier
//! informe, cada una con un origen que se comprueba; no cambian el resultado.

use serde::{Deserialize, Deserializer};

use crate::known_bug::{the_known_bug, InMaster, KnownBug};

/// Una etiqueta de la lista cerrada, con su nombre en pantalla y el porqué que la sostiene.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Label {
    AutofirmaBug(&'static KnownBug),
    AutofirmaBugInMaster(&'static KnownBug),
    RfirmaAdr(String),
    ManualDeprecated,
}

impl Label {
    pub fn name(&self) -> String {
        match self {
            Self::AutofirmaBug(_) => "autofirma:bug:1.9.2".to_owned(),
            Self::AutofirmaBugInMaster(_) => "autofirma:bug:master".to_owned(),
            Self::RfirmaAdr(adr) => format!("rfirma:{}", adr.to_lowercase()),
            Self::ManualDeprecated => "manual:deprecated".to_owned(),
        }
    }

    pub fn reason(&self) -> String {
        match self {
            Self::AutofirmaBug(bug) => {
                format!("{}: {} ({})", bug.id, bug.title, in_master_name(bug.master))
            }
            Self::AutofirmaBugInMaster(bug) => {
                format!("{} sigue presente en la rama master de AutoFirma", bug.id)
            }
            Self::RfirmaAdr(adr) => format!("Desviación deliberada de rFirma: la decide su {adr}"),
            Self::ManualDeprecated => "El manual de AutoFirma lo desaconseja (MCF, §8, pág. 97): \
                                       AutoFirma lo soporta y rFirma no."
                .to_owned(),
        }
    }

    pub fn known_bug(&self) -> Option<&'static KnownBug> {
        match self {
            Self::AutofirmaBug(bug) => Some(bug),
            _ => None,
        }
    }

    pub fn adr(&self) -> Option<&str> {
        match self {
            Self::RfirmaAdr(adr) => Some(adr),
            _ => None,
        }
    }
}

fn in_master_name(state: InMaster) -> &'static str {
    match state {
        InMaster::Present => "sigue en master",
        InMaster::Fixed => "corregido en master",
        InMaster::Partial => "corregido a medias en master",
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Explanations {
    autofirma: Option<String>,
    rfirma: Option<String>,
    manual: Option<ManualNote>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ManualNote {
    Deprecated,
}

/// Las etiquetas que declara el `explained_by` de una comprobación, con la de `master` deducida
/// del registro.
pub(crate) fn the_declared_labels<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<Label>, D::Error> {
    let explanations = Explanations::deserialize(deserializer)?;
    let mut labels = Vec::new();
    if let Some(id) = explanations.autofirma {
        let bug = the_known_bug(&id).ok_or_else(|| {
            serde::de::Error::custom(format!("{id} no está en el registro de bugs"))
        })?;
        labels.push(Label::AutofirmaBug(bug));
        if bug.master == InMaster::Present {
            labels.push(Label::AutofirmaBugInMaster(bug));
        }
    }
    if let Some(adr) = explanations.rfirma {
        if !is_an_adr_id(&adr) {
            return Err(serde::de::Error::custom(format!(
                "«{adr}» no es un ADR: se escribe ADR-NNNN"
            )));
        }
        labels.push(Label::RfirmaAdr(adr));
    }
    if let Some(ManualNote::Deprecated) = explanations.manual {
        labels.push(Label::ManualDeprecated);
    }
    Ok(labels)
}

fn is_an_adr_id(text: &str) -> bool {
    text.strip_prefix("ADR-")
        .is_some_and(|number| number.len() == 4 && number.bytes().all(|b| b.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_label_is_named_by_its_origin() {
        let bug = the_known_bug("BUG-15").unwrap();

        assert_eq!(Label::AutofirmaBug(bug).name(), "autofirma:bug:1.9.2");
        assert_eq!(
            Label::AutofirmaBugInMaster(bug).name(),
            "autofirma:bug:master"
        );
        assert_eq!(
            Label::RfirmaAdr("ADR-0010".to_owned()).name(),
            "rfirma:adr-0010"
        );
        assert_eq!(Label::ManualDeprecated.name(), "manual:deprecated");
    }

    #[test]
    fn the_reason_of_a_bug_carries_its_card_its_title_and_its_state_in_master() {
        let bug = the_known_bug("BUG-18").unwrap();

        assert_eq!(
            Label::AutofirmaBug(bug).reason(),
            format!("BUG-18: {} (corregido en master)", bug.title)
        );
    }

    #[test]
    fn the_reason_of_an_adr_label_names_the_adr() {
        assert!(Label::RfirmaAdr("ADR-0023".to_owned())
            .reason()
            .contains("ADR-0023"));
    }
}
