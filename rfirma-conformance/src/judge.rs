//! El juez: lo observado en un trámite frente a la expectativa que declara el catálogo, a un
//! resultado; no lanza trámites, no prepara nada y no pregunta a nadie.

use std::fmt;

use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
use base64::Engine as _;
use serde::Deserialize;

use crate::errand::{
    ErrandOutcome, ProtocolConditionResult, THE_DRIVER_CRASH, THE_EXHAUSTED_PATIENCE,
};
use crate::outcome::Outcome;

const THE_CANCELLED_OPERATION_EXCEPTION: &str = "es.gob.afirma.core.AOCancelledOperationException";

const THE_OUT_OF_MEMORY_ERROR: &str = "es.gob.afirma.core.OutOfMemoryError";

/// Con lo que el cliente publicado se rinde cuando nadie le contesta en ningún puerto.
const APPLICATION_NOT_FOUND_EXCEPTION: &str =
    "es.gob.afirma.standalone.ApplicationNotFoundException";

/// Lo que una comprobación conducida espera de su trámite: una sola cosa del vocabulario cerrado.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Expectation {
    Code(Code),
    Completes(Completion),
    Silence,
}

impl Expectation {
    /// Las condiciones de la sede que juzgan la comprobación, con el nombre que les da su guion.
    pub(crate) fn conditions(&self) -> &[String] {
        match self {
            Self::Completes(completion) => &completion.conditions,
            Self::Code(_) | Self::Silence => &[],
        }
    }

    /// Lo que la expectativa nombra en códigos y condiciones, para cruzarlo con el manual.
    pub(crate) fn the_declared_text(&self) -> String {
        match self {
            Self::Code(code) => code.to_string(),
            Self::Completes(completion) => completion.conditions.join("\n"),
            Self::Silence => String::new(),
        }
    }

    /// Si espera lo mismo que `other`, sin mirar el orden de sus condiciones.
    pub(crate) fn is_the_same_as(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Completes(one), Self::Completes(another)) => {
                one.with_its_conditions_sorted() == another.with_its_conditions_sorted()
            }
            _ => self == other,
        }
    }
}

/// El código que tiene que recibir la sede: `SAF_NN`, `SAF_*` o una cadena reconocida.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub(crate) enum Code {
    Saf(String),
    AnySaf,
    Cancel,
    SaveOk,
    Ok,
    MemoryError,
}

impl TryFrom<String> for Code {
    type Error = String;

    fn try_from(name: String) -> Result<Self, String> {
        match name.as_str() {
            "SAF_*" => Ok(Self::AnySaf),
            "CANCEL" => Ok(Self::Cancel),
            "SAVE_OK" => Ok(Self::SaveOk),
            "OK" => Ok(Self::Ok),
            "MEMORY_ERROR" => Ok(Self::MemoryError),
            _ if is_a_saf_code(&name) => Ok(Self::Saf(name)),
            _ => Err(format!(
                "«{name}» no es un código: SAF_NN, SAF_*, CANCEL, SAVE_OK, OK o MEMORY_ERROR"
            )),
        }
    }
}

impl Code {
    fn admits(&self, received: &str) -> bool {
        match self {
            Self::AnySaf => is_a_saf_code(received),
            _ => self.to_string() == received,
        }
    }
}

impl fmt::Display for Code {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Saf(code) => code,
            Self::AnySaf => "un código SAF",
            Self::Cancel => "CANCEL",
            Self::SaveOk => "SAVE_OK",
            Self::Ok => "OK",
            Self::MemoryError => "MEMORY_ERROR",
        })
    }
}

fn is_a_saf_code(name: &str) -> bool {
    name.strip_prefix("SAF_")
        .is_some_and(|digits| digits.len() == 2 && digits.bytes().all(|byte| byte.is_ascii_digit()))
}

/// Lo que tiene que traer el trámite completo: las condiciones de la sede y lo que vuelve; sin nada
/// declarado, basta con que vuelva.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Completion {
    #[serde(default)]
    conditions: Vec<String>,
    #[serde(default)]
    starts_with: Option<String>,
    #[serde(default)]
    contains_oid: Option<Oid>,
    #[serde(default)]
    contains_bytes: Option<Base64Bytes>,
    #[serde(default)]
    byte_length: Option<usize>,
}

impl Completion {
    fn with_its_conditions_sorted(&self) -> Self {
        let mut sorted = self.clone();
        sorted.conditions.sort();
        sorted
    }

    fn declares_what_comes_back(&self) -> bool {
        Self {
            conditions: Vec::new(),
            ..self.clone()
        } != Self::default()
    }
}

/// Un OID en notación de puntos, con su codificación DER para buscarlo entre los bytes.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct Oid {
    dotted: String,
    der: Vec<u8>,
}

impl TryFrom<String> for Oid {
    type Error = String;

    fn try_from(dotted: String) -> Result<Self, String> {
        let malformed = || format!("«{dotted}» no es un OID");
        let arcs: Vec<u64> = dotted
            .split('.')
            .map(|arc| arc.parse().map_err(|_| malformed()))
            .collect::<Result<_, _>>()?;
        let [first, second, rest @ ..] = arcs.as_slice() else {
            return Err(malformed());
        };
        if *first > 2 || (*first < 2 && *second >= 40) {
            return Err(malformed());
        }
        let mut content = in_base_128(first * 40 + second);
        for arc in rest {
            content.extend(in_base_128(*arc));
        }
        let length = u8::try_from(content.len())
            .ok()
            .filter(|length| *length < 0x80)
            .ok_or_else(malformed)?;
        let mut der = vec![0x06, length];
        der.extend(content);
        Ok(Self { dotted, der })
    }
}

fn in_base_128(mut arc: u64) -> Vec<u8> {
    let mut septets = vec![(arc & 0x7F) as u8];
    arc >>= 7;
    while arc > 0 {
        septets.push((arc & 0x7F) as u8 | 0x80);
        arc >>= 7;
    }
    septets.reverse();
    septets
}

/// Unos bytes declarados en Base64.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct Base64Bytes(Vec<u8>);

impl TryFrom<String> for Base64Bytes {
    type Error = String;

    fn try_from(encoded: String) -> Result<Self, String> {
        decoded(&encoded)
            .map(Self)
            .ok_or_else(|| format!("«{encoded}» no es Base64"))
    }
}

/// Un resultado con su observación.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Verdict {
    pub(crate) outcome: Outcome,
    pub(crate) observation: Option<String>,
}

impl Verdict {
    pub(crate) fn of(outcome: Outcome, observation: impl Into<String>) -> Self {
        Self {
            outcome,
            observation: Some(observation.into()),
        }
    }

    fn compliant(observation: impl Into<String>) -> Self {
        Self::of(Outcome::Compliant, observation)
    }

    fn noncompliant(observation: impl Into<String>) -> Self {
        Self::of(Outcome::Noncompliant, observation)
    }

    fn unobservable(observation: impl Into<String>) -> Self {
        Self::of(Outcome::NotObservable, observation)
    }

    fn unobservable_as(observed: &ErrandOutcome) -> Self {
        Self {
            outcome: Outcome::NotObservable,
            observation: observed.error_type.clone(),
        }
    }
}

/// El resultado: lo que viajó frente a lo que se esperaba.
pub(crate) fn judge(observed: &ErrandOutcome, expectation: &Expectation) -> Verdict {
    if let Some(unobservable) = the_preamble(observed, expectation) {
        return unobservable;
    }
    match expectation {
        Expectation::Code(code) => against_the_code(observed, code),
        Expectation::Completes(completion) => completed(observed, completion),
        Expectation::Silence => unanswered(observed),
    }
}

/// Lo que no deja juzgar nada, salvo que una condición esperada llegara igualmente.
fn the_preamble(observed: &ErrandOutcome, expectation: &Expectation) -> Option<Verdict> {
    if any_emitted(observed, expectation.conditions()) {
        return None;
    }
    if !observed.launched {
        return Some(Verdict::unobservable("el cliente no llegó a arrancar"));
    }
    match observed.error_type.as_deref() {
        Some(THE_DRIVER_CRASH) => Some(Verdict::unobservable_as(observed)),
        Some(THE_EXHAUSTED_PATIENCE) if *expectation != Expectation::Silence => {
            Some(Verdict::unobservable_as(observed))
        }
        _ => None,
    }
}

fn any_emitted(observed: &ErrandOutcome, names: &[String]) -> bool {
    names
        .iter()
        .any(|name| the_condition(observed, name).is_some())
}

fn the_condition<'a>(
    observed: &'a ErrandOutcome,
    name: &str,
) -> Option<&'a ProtocolConditionResult> {
    observed
        .protocol_conditions
        .iter()
        .find(|condition| condition.name == name)
}

/// Todas las condiciones a la vez: una no conforme pesa más que una sin medir.
fn the_conditions(observed: &ErrandOutcome, names: &[String]) -> Verdict {
    let verdicts: Vec<Verdict> = names
        .iter()
        .map(|name| match the_condition(observed, name) {
            Some(condition) => Verdict {
                outcome: condition.outcome,
                observation: condition.observation.clone(),
            },
            None => Verdict::unobservable(format!("la sede no emitió «{name}»")),
        })
        .collect();
    let outcome = [Outcome::Noncompliant, Outcome::NotObservable]
        .into_iter()
        .find(|worse| verdicts.iter().any(|verdict| verdict.outcome == *worse))
        .unwrap_or(Outcome::Compliant);
    let observations: Vec<String> = verdicts
        .into_iter()
        .filter(|verdict| verdict.outcome == outcome)
        .filter_map(|verdict| verdict.observation)
        .collect();
    Verdict {
        outcome,
        observation: (!observations.is_empty()).then(|| observations.join("; ")),
    }
}

/// Lo que recibió la sede, visto como respuesta del protocolo.
enum Received<'a> {
    Code(&'a str),
    Uncoded(&'a str),
    AResult,
    Nobody,
    Nothing,
}

fn what_the_site_received(observed: &ErrandOutcome) -> Received<'_> {
    if let Some(code) = observed.error_code.as_deref() {
        return Received::Code(code);
    }
    match observed.error_type.as_deref() {
        Some(THE_CANCELLED_OPERATION_EXCEPTION) => Received::Code("CANCEL"),
        Some(THE_OUT_OF_MEMORY_ERROR) => Received::Code("MEMORY_ERROR"),
        Some(APPLICATION_NOT_FOUND_EXCEPTION | THE_EXHAUSTED_PATIENCE) => Received::Nobody,
        Some(other) => Received::Uncoded(other),
        None => match observed.data.as_deref() {
            Some(said @ ("SAVE_OK" | "OK")) => Received::Code(said),
            _ if observed.signature.is_some() || observed.data.is_some() => Received::AResult,
            _ => Received::Nothing,
        },
    }
}

fn against_the_code(observed: &ErrandOutcome, expected: &Code) -> Verdict {
    match what_the_site_received(observed) {
        Received::Code(code) if expected.admits(code) => Verdict::compliant(code),
        Received::Code(said) | Received::Uncoded(said) => {
            Verdict::noncompliant(format!("{said} donde el protocolo exige {expected}"))
        }
        Received::AResult => {
            Verdict::noncompliant(format!("un resultado donde el protocolo exige {expected}"))
        }
        Received::Nobody => Verdict::noncompliant(format!(
            "nadie respondió donde el protocolo exige {expected}"
        )),
        Received::Nothing => Verdict::unobservable_as(observed),
    }
}

/// Lo que midió la sede decide; si no midió nada, un SAF donde se esperaba el trámite completo es
/// un fallo del cliente, no algo que no se vio.
fn completed(observed: &ErrandOutcome, completion: &Completion) -> Verdict {
    let conditions = &completion.conditions;
    if any_emitted(observed, conditions) {
        let verdict = the_conditions(observed, conditions);
        if verdict.outcome != Outcome::Compliant || !completion.declares_what_comes_back() {
            return verdict;
        }
        return what_came_back(observed, completion);
    }
    if let Some(code) = observed.error_code.as_deref() {
        return Verdict::noncompliant(format!(
            "{code} donde el protocolo exige que el trámite se complete"
        ));
    }
    if conditions.is_empty() {
        what_came_back(observed, completion)
    } else {
        the_conditions(observed, conditions)
    }
}

fn what_came_back(observed: &ErrandOutcome, completion: &Completion) -> Verdict {
    match observed.signature.as_deref().or(observed.data.as_deref()) {
        Some(result) => the_contents_of(result, completion),
        None => Verdict::unobservable_as(observed),
    }
}

fn the_contents_of(result: &str, contents: &Completion) -> Verdict {
    const COMPLETED: &str = "el trámite se completó";
    if !contents.declares_what_comes_back() {
        return Verdict::compliant(COMPLETED);
    }
    let Some(bytes) = decoded(result) else {
        return Verdict::noncompliant("lo que volvió no es Base64");
    };
    if let Some(prefix) = &contents.starts_with {
        if !bytes.starts_with(prefix.as_bytes()) {
            return Verdict::noncompliant(format!("lo que volvió no empieza por «{prefix}»"));
        }
    }
    if let Some(oid) = &contents.contains_oid {
        if !contains(&bytes, &oid.der) {
            return Verdict::noncompliant(format!("lo que volvió no lleva el OID {}", oid.dotted));
        }
    }
    if let Some(Base64Bytes(expected)) = &contents.contains_bytes {
        if !contains(&bytes, expected) {
            return Verdict::noncompliant("lo que volvió no contiene los bytes declarados");
        }
    }
    if let Some(length) = contents.byte_length {
        if bytes.len() != length {
            return Verdict::noncompliant(format!(
                "lo que volvió mide {} bytes y no {length}",
                bytes.len()
            ));
        }
    }
    Verdict::compliant(COMPLETED)
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    needle.is_empty()
        || haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

pub(crate) fn decoded(encoded: &str) -> Option<Vec<u8>> {
    let compact: String = encoded.split_whitespace().collect();
    [STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD]
        .iter()
        .find_map(|engine| engine.decode(&compact).ok())
}

fn unanswered(observed: &ErrandOutcome) -> Verdict {
    match what_the_site_received(observed) {
        Received::Nobody | Received::Nothing => Verdict::compliant("nadie respondió"),
        Received::Code(said) | Received::Uncoded(said) => {
            Verdict::noncompliant(format!("{said} donde no debía responder nadie"))
        }
        Received::AResult => Verdict::noncompliant("un resultado donde no debía responder nadie"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogue::the_catalogue_in;
    use crate::errand::ProtocolConditionResult;

    fn observed() -> ErrandOutcome {
        ErrandOutcome {
            launched: true,
            error_type: None,
            error_code: None,
            signature: None,
            data: None,
            protocol_conditions: Vec::new(),
        }
    }

    fn unlaunched() -> ErrandOutcome {
        ErrandOutcome {
            launched: false,
            ..observed()
        }
    }

    fn with_code(code: &str) -> ErrandOutcome {
        ErrandOutcome {
            error_type: Some("java.lang.Exception".to_owned()),
            error_code: Some(code.to_owned()),
            ..observed()
        }
    }

    fn with_error(kind: &str) -> ErrandOutcome {
        ErrandOutcome {
            error_type: Some(kind.to_owned()),
            ..observed()
        }
    }

    fn with_signature(bytes: &[u8]) -> ErrandOutcome {
        ErrandOutcome {
            signature: Some(STANDARD.encode(bytes)),
            ..observed()
        }
    }

    fn with_data(data: &str) -> ErrandOutcome {
        ErrandOutcome {
            data: Some(data.to_owned()),
            ..observed()
        }
    }

    fn with_condition(name: &str, outcome: Outcome) -> ErrandOutcome {
        with_conditions(&[(name, outcome)])
    }

    fn with_conditions(conditions: &[(&str, Outcome)]) -> ErrandOutcome {
        ErrandOutcome {
            protocol_conditions: conditions
                .iter()
                .map(|(name, outcome)| ProtocolConditionResult {
                    name: (*name).to_owned(),
                    outcome: *outcome,
                    observation: Some("lo que midió la sede".to_owned()),
                })
                .collect(),
            ..observed()
        }
    }
    const CRASH: &str = THE_DRIVER_CRASH;
    const TIMEOUT: &str = THE_EXHAUSTED_PATIENCE;
    const NOBODY: &str = APPLICATION_NOT_FOUND_EXCEPTION;
    const CANCELLED: &str = THE_CANCELLED_OPERATION_EXCEPTION;
    const TIMESTAMP_OID: &str = "1.2.840.113549.1.9.16.2.14";
    const TIMESTAMP_DER: [u8; 13] = [
        0x06, 0x0B, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x10, 0x02, 0x0E,
    ];

    use Outcome::{Compliant as C, Noncompliant as NC, NotObservable as NO};

    struct Case {
        name: &'static str,
        declares: String,
        observed: ErrandOutcome,
        expected: (Outcome, Option<&'static str>),
    }

    fn case(
        name: &'static str,
        declares: impl Into<String>,
        observed: ErrandOutcome,
        expected: (Outcome, Option<&'static str>),
    ) -> Case {
        Case {
            name,
            declares: declares.into(),
            observed,
            expected,
        }
    }

    fn the_table() -> Vec<Case> {
        let mut pdf = b"%PDF-1.7 ".to_vec();
        pdf.extend_from_slice(&TIMESTAMP_DER);
        vec![
            case(
                "the expected saf code is compliant",
                "expects.code = \"SAF_06\"",
                with_code("SAF_06"),
                (C, Some("SAF_06")),
            ),
            case(
                "another saf code says which was expected",
                "expects.code = \"SAF_06\"",
                with_code("SAF_03"),
                (NC, Some("SAF_03 donde el protocolo exige SAF_06")),
            ),
            case(
                "a result where a saf was due is noncompliant",
                "expects.code = \"SAF_06\"",
                with_signature(b"firma"),
                (NC, Some("un resultado donde el protocolo exige SAF_06")),
            ),
            case(
                "an uncoded error where a saf was due is noncompliant",
                "expects.code = \"SAF_06\"",
                with_error("java.lang.Exception"),
                (
                    NC,
                    Some("java.lang.Exception donde el protocolo exige SAF_06"),
                ),
            ),
            case(
                "nobody answering where a saf was due is noncompliant",
                "expects.code = \"SAF_47\"",
                with_error(NOBODY),
                (NC, Some("nadie respondió donde el protocolo exige SAF_47")),
            ),
            case(
                "a crashed driver is not observable",
                "expects.code = \"SAF_47\"",
                with_error(CRASH),
                (NO, Some(CRASH)),
            ),
            case(
                "an exhausted patience is not observable",
                "expects.code = \"SAF_06\"",
                with_error(TIMEOUT),
                (NO, Some(TIMEOUT)),
            ),
            case(
                "a client that never launched is not observable",
                "expects.code = \"SAF_06\"",
                unlaunched(),
                (NO, Some("el cliente no llegó a arrancar")),
            ),
            case(
                "an errand that brought nothing is not observable",
                "expects.code = \"SAF_06\"",
                observed(),
                (NO, None),
            ),
            case(
                "any saf code is admitted when any is expected",
                "expects.code = \"SAF_*\"",
                with_code("SAF_09"),
                (C, Some("SAF_09")),
            ),
            case(
                "a signature where any saf was due is noncompliant",
                "expects.code = \"SAF_*\"",
                with_signature(b"firma"),
                (
                    NC,
                    Some("un resultado donde el protocolo exige un código SAF"),
                ),
            ),
            case(
                "a cancellation the site received as such is compliant",
                "expects.code = \"CANCEL\"",
                with_error(CANCELLED),
                (C, Some("CANCEL")),
            ),
            case(
                "a signature where a cancellation was due is noncompliant",
                "expects.code = \"CANCEL\"",
                with_signature(b"firma"),
                (NC, Some("un resultado donde el protocolo exige CANCEL")),
            ),
            case(
                "an exhausted memory is recognised",
                "expects.code = \"MEMORY_ERROR\"",
                with_error("es.gob.afirma.core.OutOfMemoryError"),
                (C, Some("MEMORY_ERROR")),
            ),
            case(
                "a save confirmation is recognised",
                "expects.code = \"SAVE_OK\"",
                with_data("SAVE_OK"),
                (C, Some("SAVE_OK")),
            ),
            case(
                "ok where save_ok was due is noncompliant",
                "expects.code = \"SAVE_OK\"",
                with_data("OK"),
                (NC, Some("OK donde el protocolo exige SAVE_OK")),
            ),
            case(
                "a completed errand is compliant",
                "expects.completes = {}",
                with_data("MIIC"),
                (C, Some("el trámite se completó")),
            ),
            case(
                "a rejected valid request is never painted green",
                "expects.completes = {}",
                with_code("SAF_03"),
                (
                    NC,
                    Some("SAF_03 donde el protocolo exige que el trámite se complete"),
                ),
            ),
            case(
                "a completion that brought nothing is not observable",
                "expects.completes = {}",
                with_error(CANCELLED),
                (NO, Some(CANCELLED)),
            ),
            case(
                "a result that starts as declared is compliant",
                "expects.completes = { starts_with = \"%PDF\" }",
                with_signature(&pdf),
                (C, Some("el trámite se completó")),
            ),
            case(
                "a result that starts otherwise is noncompliant",
                "expects.completes = { starts_with = \"%PDF\" }",
                with_signature(b"PK\x03\x04"),
                (NC, Some("lo que volvió no empieza por «%PDF»")),
            ),
            case(
                "a result carrying the declared oid is compliant",
                format!("expects.completes = {{ contains_oid = \"{TIMESTAMP_OID}\" }}"),
                with_signature(&pdf),
                (C, Some("el trámite se completó")),
            ),
            case(
                "a result without the declared oid is noncompliant",
                format!("expects.completes = {{ contains_oid = \"{TIMESTAMP_OID}\" }}"),
                with_signature(b"CMS sin sello"),
                (
                    NC,
                    Some("lo que volvió no lleva el OID 1.2.840.113549.1.9.16.2.14"),
                ),
            ),
            case(
                "a result carrying the declared bytes is compliant",
                format!(
                    "expects.completes = {{ contains_bytes = \"{}\" }}",
                    STANDARD.encode(b"reto")
                ),
                with_signature(b"antes reto despues"),
                (C, Some("el trámite se completó")),
            ),
            case(
                "a result without the declared bytes is noncompliant",
                format!(
                    "expects.completes = {{ contains_bytes = \"{}\" }}",
                    STANDARD.encode(b"reto")
                ),
                with_signature(b"otra cosa"),
                (NC, Some("lo que volvió no contiene los bytes declarados")),
            ),
            case(
                "a result as long as the key is compliant",
                "expects.completes = { byte_length = 4 }",
                with_signature(b"1234"),
                (C, Some("el trámite se completó")),
            ),
            case(
                "a result of another length is noncompliant",
                "expects.completes = { byte_length = 256 }",
                with_signature(b"1234"),
                (NC, Some("lo que volvió mide 4 bytes y no 256")),
            ),
            case(
                "a result that is not base64 fails its byte checks",
                "expects.completes = { byte_length = 4 }",
                with_data("no es base64: %%%"),
                (NC, Some("lo que volvió no es Base64")),
            ),
            case(
                "the expected condition is the outcome",
                "expects.completes.conditions = [\"a-certificate-alone\"]",
                with_condition("a-certificate-alone", NC),
                (NC, Some("lo que midió la sede")),
            ),
            case(
                "a condition nobody expects does not judge",
                "expects.completes.conditions = [\"a-certificate-alone\"]",
                with_condition("another", C),
                (NO, Some("la sede no emitió «a-certificate-alone»")),
            ),
            case(
                "every condition of a list holding is compliant",
                "expects.completes.conditions = [\"a-certificate-alone\", \"another\"]",
                with_conditions(&[("a-certificate-alone", C), ("another", C)]),
                (C, Some("lo que midió la sede; lo que midió la sede")),
            ),
            case(
                "one condition of a list failing is noncompliant",
                "expects.completes.conditions = [\"a-certificate-alone\", \"another\"]",
                with_conditions(&[("a-certificate-alone", C), ("another", NC)]),
                (NC, Some("lo que midió la sede")),
            ),
            case(
                "a condition of a list left unsent is not observable",
                "expects.completes.conditions = [\"a-certificate-alone\", \"another\"]",
                with_condition("a-certificate-alone", C),
                (NO, Some("la sede no emitió «another»")),
            ),
            case(
                "a failing condition outweighs one left unsent",
                "expects.completes.conditions = [\"a-certificate-alone\", \"another\"]",
                ErrandOutcome {
                    error_type: Some(TIMEOUT.to_owned()),
                    ..with_condition("another", NC)
                },
                (NC, Some("lo que midió la sede")),
            ),
            case(
                "an emitted condition is judged even without a launch",
                "expects.completes.conditions = [\"no-channel-opens\"]",
                ErrandOutcome {
                    launched: false,
                    ..with_condition("no-channel-opens", C)
                },
                (C, Some("lo que midió la sede")),
            ),
            case(
                "a saf where the conditions of a completion were due is noncompliant",
                "expects.completes.conditions = [\"a-certificate-alone\"]",
                with_code("SAF_03"),
                (
                    NC,
                    Some("SAF_03 donde el protocolo exige que el trámite se complete"),
                ),
            ),
            case(
                "an emitted condition decides over a saf that came with it",
                "expects.completes.conditions = [\"the-refused-upload-attempted\"]",
                ErrandOutcome {
                    error_code: Some("SAF_11".to_owned()),
                    ..with_condition("the-refused-upload-attempted", C)
                },
                (C, Some("lo que midió la sede")),
            ),
            case(
                "a completion with conditions also weighs what came back",
                "expects.completes = { starts_with = \"%PDF\", conditions = [\"another\"] }",
                ErrandOutcome {
                    signature: Some(STANDARD.encode(b"PK\x03\x04")),
                    ..with_condition("another", C)
                },
                (NC, Some("lo que volvió no empieza por «%PDF»")),
            ),
            case(
                "nobody answering where nobody should is compliant",
                "expects = \"silence\"",
                with_error(NOBODY),
                (C, Some("nadie respondió")),
            ),
            case(
                "an exhausted patience where nobody should answer is compliant",
                "expects = \"silence\"",
                with_error(TIMEOUT),
                (C, Some("nadie respondió")),
            ),
            case(
                "an answer where nobody should answer is noncompliant",
                "expects = \"silence\"",
                with_code("SAF_06"),
                (NC, Some("SAF_06 donde no debía responder nadie")),
            ),
        ]
    }

    fn a_check_declaring(declares: &str) -> crate::catalogue::Check {
        the_catalogue_in(&format!(
            "[[check]]\nid = \"an_id\"\nset = \"errores\"\nchapter = \"15\"\n\
             citation = \"A.java:1\"\nstatement = \"Algo.\"\n\n\
             [check.drive]\nmode = \"v4\"\nscript = \"selectcert\"\n{declares}\n"
        ))
        .unwrap_or_else(|complaint| panic!("{declares}: {complaint}"))
        .remove(0)
    }

    #[test]
    fn every_row_of_the_table_is_judged_as_it_says() {
        let failures: Vec<String> = the_table()
            .into_iter()
            .filter_map(|case| {
                let check = a_check_declaring(&case.declares);
                let judged = judge(&case.observed, &check.trial().unwrap().expects);
                let (outcome, said) = case.expected;
                let agrees = judged.outcome == outcome && judged.observation.as_deref() == said;
                (!agrees).then(|| {
                    format!(
                        "{}: {:?} {:?}",
                        case.name, judged.outcome, judged.observation
                    )
                })
            })
            .collect();
        assert!(failures.is_empty(), "{failures:#?}");
    }

    #[test]
    fn two_expectations_are_the_same_whatever_the_order_of_their_conditions() {
        let one = a_check_declaring("expects.completes.conditions = [\"a\", \"b\"]");
        let other = a_check_declaring("expects.completes.conditions = [\"b\", \"a\"]");
        let another = a_check_declaring("expects.completes.conditions = [\"a\"]");

        let expects = |check: &crate::catalogue::Check| check.trial().unwrap().expects.clone();
        assert!(expects(&one).is_the_same_as(&expects(&other)));
        assert!(!expects(&one).is_the_same_as(&expects(&another)));
    }

    #[test]
    fn an_oid_is_encoded_as_its_der() {
        let oid = Oid::try_from(TIMESTAMP_OID.to_owned()).unwrap();
        assert_eq!(oid.der, TIMESTAMP_DER);
    }

    #[test]
    fn a_word_outside_the_vocabulary_is_rejected_when_read() {
        for declares in [
            "expects.code = \"SAF_6\"",
            "expects.code = \"ERROR\"",
            "expects.completes = { contains_oid = \"uno.dos\" }",
            "expects.completes = { contains_bytes = \"%%%\" }",
            "expects.completes = { ends_with = \"%%EOF\" }",
            "expects = \"nothing\"",
            "expects.condition = [\"a\"]",
        ] {
            assert!(
                the_catalogue_in(&format!(
                    "[[check]]\nid = \"a\"\nset = \"errores\"\nchapter = \"1\"\n\
                     citation = \"A\"\nstatement = \"B\"\n\n\
                     [check.drive]\nmode = \"v4\"\nscript = \"selectcert\"\n{declares}\n"
                ))
                .is_err(),
                "{declares}"
            );
        }
    }
}
