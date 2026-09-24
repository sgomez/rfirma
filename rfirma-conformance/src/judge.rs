//! El juez: lo observado en un trámite, la expectativa que declara el catálogo y la respuesta de la
//! persona, a un resultado; no lanza trámites ni prepara nada.

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

/// Un resultado con su observación, o pendiente si la persona no respondió.
pub(crate) enum CheckOutcome {
    Resolved {
        outcome: Outcome,
        observation: Option<String>,
    },
    StillPending,
}

impl CheckOutcome {
    pub(crate) fn of(outcome: Outcome, observation: impl Into<String>) -> Self {
        Self::Resolved {
            outcome,
            observation: Some(observation.into()),
        }
    }
}

/// Lo que respondió la persona a la pregunta de la comprobación.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Answer {
    Yes,
    No,
    Unanswered,
}

impl Answer {
    /// La respuesta escrita en la consola: sí si empieza por «s», y vacía si no contestó.
    pub(crate) fn read(reply: &str) -> Self {
        let reply = reply.trim().to_lowercase();
        if reply.is_empty() {
            Self::Unanswered
        } else if reply.starts_with('s') {
            Self::Yes
        } else {
            Self::No
        }
    }
}

/// Lo que una comprobación espera del trámite, tal y como la declara el catálogo.
pub(crate) struct Expectation<'a> {
    pub(crate) on_the_wire: Option<OnTheWire<'a>>,
    pub(crate) person: Option<&'a Person>,
}

/// Lo que se espera que viaje por el cable: una de las cuatro formas del vocabulario.
pub(crate) enum OnTheWire<'a> {
    Saf(&'a Code),
    Completes(&'a Contents),
    Conditions(&'a [String]),
    NoAnswer,
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

/// Lo que tiene que traer el resultado; sin nada declarado, basta con que vuelva.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Contents {
    #[serde(default)]
    starts_with: Option<String>,
    #[serde(default)]
    contains_oid: Option<Oid>,
    #[serde(default)]
    contains_bytes: Option<Base64Bytes>,
    #[serde(default)]
    byte_length: Option<usize>,
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

/// Lo que la persona dice de lo que vio: una frase por respuesta, y qué resultado sostiene el sí.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Person {
    yes: String,
    no: String,
    yes_means: Meaning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
enum Meaning {
    #[serde(rename = "conforme")]
    Compliant,
    #[serde(rename = "no-conforme")]
    Noncompliant,
}

/// El resultado: lo que viajó, frente a lo que se esperaba y lo que dijo la persona.
pub(crate) fn judge(
    observed: &ErrandOutcome,
    expectation: &Expectation<'_>,
    answer: Answer,
) -> CheckOutcome {
    if let Some(unobservable) = the_preamble(observed, expectation.on_the_wire.as_ref()) {
        return unobservable.into();
    }
    let on_the_wire = expectation
        .on_the_wire
        .as_ref()
        .map(|wire| on_the_wire(observed, wire));
    match expectation.person {
        Some(person) => by_the_person(person, answer, on_the_wire, observed),
        None => on_the_wire
            .unwrap_or_else(|| Verdict::unobservable("la comprobación no declara qué espera"))
            .into(),
    }
}

struct Verdict {
    outcome: Outcome,
    observation: Option<String>,
}

impl Verdict {
    fn of(outcome: Outcome, observation: impl Into<String>) -> Self {
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

impl From<Verdict> for CheckOutcome {
    fn from(verdict: Verdict) -> Self {
        Self::Resolved {
            outcome: verdict.outcome,
            observation: verdict.observation,
        }
    }
}

/// Lo que no deja juzgar nada, salvo que la condición esperada llegara igualmente.
fn the_preamble(observed: &ErrandOutcome, wire: Option<&OnTheWire<'_>>) -> Option<Verdict> {
    if let Some(OnTheWire::Conditions(names)) = wire {
        if names
            .iter()
            .any(|name| the_condition(observed, name).is_some())
        {
            return None;
        }
    }
    if !observed.launched {
        return Some(Verdict::unobservable("el cliente no llegó a arrancar"));
    }
    match observed.error_type.as_deref() {
        Some(THE_DRIVER_CRASH) => Some(Verdict::unobservable_as(observed)),
        Some(THE_EXHAUSTED_PATIENCE) if !matches!(wire, Some(OnTheWire::NoAnswer)) => {
            Some(Verdict::unobservable_as(observed))
        }
        _ => None,
    }
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

fn on_the_wire(observed: &ErrandOutcome, wire: &OnTheWire<'_>) -> Verdict {
    match wire {
        OnTheWire::Saf(code) => against_the_code(observed, code),
        OnTheWire::Completes(contents) => completed(observed, contents),
        OnTheWire::Conditions(names) => the_conditions(observed, names),
        OnTheWire::NoAnswer => unanswered(observed),
    }
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

fn completed(observed: &ErrandOutcome, contents: &Contents) -> Verdict {
    if let Some(code) = observed.error_code.as_deref() {
        return Verdict::noncompliant(format!(
            "{code} donde el protocolo exige que el trámite se complete"
        ));
    }
    match observed.signature.as_deref().or(observed.data.as_deref()) {
        Some(result) => the_contents_of(result, contents),
        None => Verdict::unobservable_as(observed),
    }
}

fn the_contents_of(result: &str, contents: &Contents) -> Verdict {
    const COMPLETED: &str = "el trámite se completó";
    if *contents == Contents::default() {
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

/// La respuesta esperada deja decidir al cable; un «no» inesperado pesa si el trámite llegó.
fn by_the_person(
    person: &Person,
    answer: Answer,
    on_the_wire: Option<Verdict>,
    observed: &ErrandOutcome,
) -> CheckOutcome {
    let said_yes = match answer {
        Answer::Unanswered => return CheckOutcome::StillPending,
        Answer::Yes => true,
        Answer::No => false,
    };
    let phrase = if said_yes { &person.yes } else { &person.no };
    let as_expected = said_yes == (person.yes_means == Meaning::Compliant);
    let verdict = if as_expected {
        match on_the_wire {
            Some(verdict) if verdict.outcome != Outcome::Compliant => verdict,
            _ => Verdict::compliant(phrase),
        }
    } else if said_yes {
        Verdict::noncompliant(phrase)
    } else {
        let this_far = on_the_wire.unwrap_or_else(|| completed(observed, &Contents::default()));
        if this_far.outcome == Outcome::NotObservable {
            this_far
        } else {
            Verdict::noncompliant(phrase)
        }
    };
    verdict.into()
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

    const DIALOGUE: &str = "question = \"¿se pidió? [s/n]\"\n\
         person = { yes = \"se pidió\", no = \"no se pidió\", yes_means = \"conforme\" }";
    const UNWANTED: &str = "question = \"¿se pidió? [s/n]\"\n\
         person = { yes = \"se pidió\", no = \"no se pidió\", yes_means = \"no-conforme\" }";

    enum Expected {
        Resolved(Outcome, Option<&'static str>),
        Pending,
    }

    use Expected::{Pending, Resolved};
    use Outcome::{Compliant as C, Noncompliant as NC, NotObservable as NO};

    struct Case {
        name: &'static str,
        declares: String,
        observed: ErrandOutcome,
        answer: Answer,
        expected: Expected,
    }

    fn case(
        name: &'static str,
        declares: impl Into<String>,
        observed: ErrandOutcome,
        answer: Answer,
        expected: Expected,
    ) -> Case {
        Case {
            name,
            declares: declares.into(),
            observed,
            answer,
            expected,
        }
    }

    fn with_dialogue(wire: &str) -> String {
        format!("{wire}\n{DIALOGUE}")
    }

    fn with_unwanted(wire: &str) -> String {
        format!("{wire}\n{UNWANTED}")
    }

    fn the_table() -> Vec<Case> {
        use Answer::{No, Unanswered as Silent, Yes};
        let mut pdf = b"%PDF-1.7 ".to_vec();
        pdf.extend_from_slice(&TIMESTAMP_DER);
        vec![
            case(
                "the expected saf code is compliant",
                "saf = \"SAF_06\"",
                with_code("SAF_06"),
                Silent,
                Resolved(C, Some("SAF_06")),
            ),
            case(
                "another saf code says which was expected",
                "saf = \"SAF_06\"",
                with_code("SAF_03"),
                Silent,
                Resolved(NC, Some("SAF_03 donde el protocolo exige SAF_06")),
            ),
            case(
                "a result where a saf was due is noncompliant",
                "saf = \"SAF_06\"",
                with_signature(b"firma"),
                Silent,
                Resolved(NC, Some("un resultado donde el protocolo exige SAF_06")),
            ),
            case(
                "an uncoded error where a saf was due is noncompliant",
                "saf = \"SAF_06\"",
                with_error("java.lang.Exception"),
                Silent,
                Resolved(
                    NC,
                    Some("java.lang.Exception donde el protocolo exige SAF_06"),
                ),
            ),
            case(
                "nobody answering where a saf was due is noncompliant",
                "saf = \"SAF_47\"",
                with_error(NOBODY),
                Silent,
                Resolved(NC, Some("nadie respondió donde el protocolo exige SAF_47")),
            ),
            case(
                "a crashed driver is not observable",
                "saf = \"SAF_47\"",
                with_error(CRASH),
                Silent,
                Resolved(NO, Some(CRASH)),
            ),
            case(
                "an exhausted patience is not observable",
                "saf = \"SAF_06\"",
                with_error(TIMEOUT),
                Silent,
                Resolved(NO, Some(TIMEOUT)),
            ),
            case(
                "a client that never launched is not observable",
                "saf = \"SAF_06\"",
                unlaunched(),
                Silent,
                Resolved(NO, Some("el cliente no llegó a arrancar")),
            ),
            case(
                "an errand that brought nothing is not observable",
                "saf = \"SAF_06\"",
                observed(),
                Silent,
                Resolved(NO, None),
            ),
            case(
                "any saf code is admitted when any is expected",
                "saf = \"SAF_*\"",
                with_code("SAF_09"),
                Silent,
                Resolved(C, Some("SAF_09")),
            ),
            case(
                "a signature where any saf was due is noncompliant",
                "saf = \"SAF_*\"",
                with_signature(b"firma"),
                Silent,
                Resolved(
                    NC,
                    Some("un resultado donde el protocolo exige un código SAF"),
                ),
            ),
            case(
                "a cancellation the site received as such is compliant",
                "saf = \"CANCEL\"",
                with_error(CANCELLED),
                Silent,
                Resolved(C, Some("CANCEL")),
            ),
            case(
                "a signature where a cancellation was due is noncompliant",
                "saf = \"CANCEL\"",
                with_signature(b"firma"),
                Silent,
                Resolved(NC, Some("un resultado donde el protocolo exige CANCEL")),
            ),
            case(
                "an exhausted memory is recognised",
                "saf = \"MEMORY_ERROR\"",
                with_error("es.gob.afirma.core.OutOfMemoryError"),
                Silent,
                Resolved(C, Some("MEMORY_ERROR")),
            ),
            case(
                "a save confirmation is recognised",
                "saf = \"SAVE_OK\"",
                with_data("SAVE_OK"),
                Silent,
                Resolved(C, Some("SAVE_OK")),
            ),
            case(
                "ok where save_ok was due is noncompliant",
                "saf = \"SAVE_OK\"",
                with_data("OK"),
                Silent,
                Resolved(NC, Some("OK donde el protocolo exige SAVE_OK")),
            ),
            case(
                "a completed errand is compliant",
                "completes = {}",
                with_data("MIIC"),
                Silent,
                Resolved(C, Some("el trámite se completó")),
            ),
            case(
                "a rejected valid request is never painted green",
                "completes = {}",
                with_code("SAF_03"),
                Silent,
                Resolved(
                    NC,
                    Some("SAF_03 donde el protocolo exige que el trámite se complete"),
                ),
            ),
            case(
                "a completion that brought nothing is not observable",
                "completes = {}",
                with_error(CANCELLED),
                Silent,
                Resolved(NO, Some(CANCELLED)),
            ),
            case(
                "a result that starts as declared is compliant",
                "completes = { starts_with = \"%PDF\" }",
                with_signature(&pdf),
                Silent,
                Resolved(C, Some("el trámite se completó")),
            ),
            case(
                "a result that starts otherwise is noncompliant",
                "completes = { starts_with = \"%PDF\" }",
                with_signature(b"PK\x03\x04"),
                Silent,
                Resolved(NC, Some("lo que volvió no empieza por «%PDF»")),
            ),
            case(
                "a result carrying the declared oid is compliant",
                format!("completes = {{ contains_oid = \"{TIMESTAMP_OID}\" }}"),
                with_signature(&pdf),
                Silent,
                Resolved(C, Some("el trámite se completó")),
            ),
            case(
                "a result without the declared oid is noncompliant",
                format!("completes = {{ contains_oid = \"{TIMESTAMP_OID}\" }}"),
                with_signature(b"CMS sin sello"),
                Silent,
                Resolved(
                    NC,
                    Some("lo que volvió no lleva el OID 1.2.840.113549.1.9.16.2.14"),
                ),
            ),
            case(
                "a result carrying the declared bytes is compliant",
                format!(
                    "completes = {{ contains_bytes = \"{}\" }}",
                    STANDARD.encode(b"reto")
                ),
                with_signature(b"antes reto despues"),
                Silent,
                Resolved(C, Some("el trámite se completó")),
            ),
            case(
                "a result without the declared bytes is noncompliant",
                format!(
                    "completes = {{ contains_bytes = \"{}\" }}",
                    STANDARD.encode(b"reto")
                ),
                with_signature(b"otra cosa"),
                Silent,
                Resolved(NC, Some("lo que volvió no contiene los bytes declarados")),
            ),
            case(
                "a result as long as the key is compliant",
                "completes = { byte_length = 4 }",
                with_signature(b"1234"),
                Silent,
                Resolved(C, Some("el trámite se completó")),
            ),
            case(
                "a result of another length is noncompliant",
                "completes = { byte_length = 256 }",
                with_signature(b"1234"),
                Silent,
                Resolved(NC, Some("lo que volvió mide 4 bytes y no 256")),
            ),
            case(
                "a result that is not base64 fails its byte checks",
                "completes = { byte_length = 4 }",
                with_data("no es base64: %%%"),
                Silent,
                Resolved(NC, Some("lo que volvió no es Base64")),
            ),
            case(
                "the expected condition is the outcome",
                "condition = \"a-certificate-alone\"",
                with_condition("a-certificate-alone", NC),
                Silent,
                Resolved(NC, Some("lo que midió la sede")),
            ),
            case(
                "a condition nobody expects does not judge",
                "condition = \"a-certificate-alone\"",
                with_condition("another", C),
                Silent,
                Resolved(NO, Some("la sede no emitió «a-certificate-alone»")),
            ),
            case(
                "every condition of a list holding is compliant",
                "condition = [\"a-certificate-alone\", \"another\"]",
                with_conditions(&[("a-certificate-alone", C), ("another", C)]),
                Silent,
                Resolved(C, Some("lo que midió la sede; lo que midió la sede")),
            ),
            case(
                "one condition of a list failing is noncompliant",
                "condition = [\"a-certificate-alone\", \"another\"]",
                with_conditions(&[("a-certificate-alone", C), ("another", NC)]),
                Silent,
                Resolved(NC, Some("lo que midió la sede")),
            ),
            case(
                "a condition of a list left unsent is not observable",
                "condition = [\"a-certificate-alone\", \"another\"]",
                with_condition("a-certificate-alone", C),
                Silent,
                Resolved(NO, Some("la sede no emitió «another»")),
            ),
            case(
                "a failing condition outweighs one left unsent",
                "condition = [\"a-certificate-alone\", \"another\"]",
                ErrandOutcome {
                    error_type: Some(TIMEOUT.to_owned()),
                    ..with_condition("another", NC)
                },
                Silent,
                Resolved(NC, Some("lo que midió la sede")),
            ),
            case(
                "an emitted condition is judged even without a launch",
                "condition = \"no-channel-opens\"",
                ErrandOutcome {
                    launched: false,
                    ..with_condition("no-channel-opens", C)
                },
                Silent,
                Resolved(C, Some("lo que midió la sede")),
            ),
            case(
                "nobody answering where nobody should is compliant",
                "no_answer = true",
                with_error(NOBODY),
                Silent,
                Resolved(C, Some("nadie respondió")),
            ),
            case(
                "an exhausted patience where nobody should answer is compliant",
                "no_answer = true",
                with_error(TIMEOUT),
                Silent,
                Resolved(C, Some("nadie respondió")),
            ),
            case(
                "an answer where nobody should answer is noncompliant",
                "no_answer = true",
                with_code("SAF_06"),
                Silent,
                Resolved(NC, Some("SAF_06 donde no debía responder nadie")),
            ),
            case(
                "a dialogue nobody answered stays pending",
                DIALOGUE,
                with_data("MIIC"),
                Silent,
                Pending,
            ),
            case(
                "a dialogue that was seen is compliant",
                DIALOGUE,
                with_data("MIIC"),
                Yes,
                Resolved(C, Some("se pidió")),
            ),
            case(
                "a dialogue that was seen is compliant even if nothing came back",
                DIALOGUE,
                with_error(CANCELLED),
                Yes,
                Resolved(C, Some("se pidió")),
            ),
            case(
                "a dialogue unseen on an errand that came back is noncompliant",
                DIALOGUE,
                with_signature(b"firma"),
                No,
                Resolved(NC, Some("no se pidió")),
            ),
            case(
                "a dialogue unseen on an errand that never came back is not observable",
                DIALOGUE,
                with_error(CANCELLED),
                No,
                Resolved(NO, Some(CANCELLED)),
            ),
            case(
                "a dialogue whose client never launched is not observable before asking",
                DIALOGUE,
                unlaunched(),
                Silent,
                Resolved(NO, Some("el cliente no llegó a arrancar")),
            ),
            case(
                "a crashed dialogue is not observable before asking",
                DIALOGUE,
                with_error(CRASH),
                Silent,
                Resolved(NO, Some(CRASH)),
            ),
            case(
                "a seen dialogue lets the wire decide",
                with_dialogue("saf = \"SAVE_OK\""),
                with_data("SAVE_OK"),
                Yes,
                Resolved(C, Some("se pidió")),
            ),
            case(
                "a seen dialogue with the wrong code is noncompliant",
                with_dialogue("saf = \"SAVE_OK\""),
                with_error("java.lang.Exception"),
                Yes,
                Resolved(
                    NC,
                    Some("java.lang.Exception donde el protocolo exige SAVE_OK"),
                ),
            ),
            case(
                "a seen dialogue with an unfinished errand is not observable",
                with_dialogue("completes = {}"),
                observed(),
                Yes,
                Resolved(NO, None),
            ),
            case(
                "a seen dialogue takes the condition the site measured",
                with_dialogue("condition = \"every-file-apart\""),
                with_condition("every-file-apart", NC),
                Yes,
                Resolved(NC, Some("lo que midió la sede")),
            ),
            case(
                "an unseen dialogue with its condition is noncompliant",
                with_dialogue("condition = \"every-file-apart\""),
                with_condition("every-file-apart", C),
                No,
                Resolved(NC, Some("no se pidió")),
            ),
            case(
                "an unwanted dialogue that was seen is noncompliant",
                with_unwanted("completes = {}"),
                with_data("MIIC"),
                Yes,
                Resolved(NC, Some("se pidió")),
            ),
            case(
                "an unwanted dialogue seen on an errand that never came back is noncompliant",
                with_unwanted("completes = {}"),
                with_error(CANCELLED),
                Yes,
                Resolved(NC, Some("se pidió")),
            ),
            case(
                "an unwanted dialogue not seen lets the wire decide",
                with_unwanted("completes = {}"),
                with_data("MIIC"),
                No,
                Resolved(C, Some("no se pidió")),
            ),
            case(
                "an unwanted dialogue not seen on an errand that never came back is not observable",
                with_unwanted("completes = {}"),
                observed(),
                No,
                Resolved(NO, None),
            ),
            case(
                "a silence the person was told about is compliant",
                with_dialogue("no_answer = true"),
                with_error(NOBODY),
                Yes,
                Resolved(C, Some("se pidió")),
            ),
            case(
                "a silence nobody was told about is noncompliant",
                with_dialogue("no_answer = true"),
                with_error(NOBODY),
                No,
                Resolved(NC, Some("no se pidió")),
            ),
            case(
                "a channel that opened after all is noncompliant whatever was said",
                with_dialogue("no_answer = true"),
                with_code("SAF_06"),
                Yes,
                Resolved(NC, Some("SAF_06 donde no debía responder nadie")),
            ),
        ]
    }

    fn a_check_declaring(declares: &str) -> crate::catalogue::Check {
        the_catalogue_in(&format!(
            "[[check]]\nid = \"an_id\"\nset = \"errores\"\nchapter = \"15\"\n\
             citation = \"A.java:1\"\nstatement = \"Algo.\"\n\
             drive = {{ mode = \"v4\", script = \"selectcert\" }}\nassistance = \"person\"\n\
             {declares}\n"
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
                let judged = judge(&case.observed, &check.expectation(), case.answer);
                let agrees = match (&judged, &case.expected) {
                    (CheckOutcome::StillPending, Pending) => true,
                    (
                        CheckOutcome::Resolved {
                            outcome,
                            observation,
                        },
                        Resolved(expected, said),
                    ) => outcome == expected && observation.as_deref() == *said,
                    _ => false,
                };
                (!agrees).then(|| {
                    let got = match judged {
                        CheckOutcome::StillPending => "pendiente".to_owned(),
                        CheckOutcome::Resolved {
                            outcome,
                            observation,
                        } => format!("{outcome:?} {observation:?}"),
                    };
                    format!("{}: {got}", case.name)
                })
            })
            .collect();
        assert!(failures.is_empty(), "{failures:#?}");
    }

    #[test]
    fn an_answer_is_read_from_what_was_written() {
        assert_eq!(Answer::read("Sí"), Answer::Yes);
        assert_eq!(Answer::read("n"), Answer::No);
        assert_eq!(Answer::read("  "), Answer::Unanswered);
    }

    #[test]
    fn an_oid_is_encoded_as_its_der() {
        let oid = Oid::try_from(TIMESTAMP_OID.to_owned()).unwrap();
        assert_eq!(oid.der, TIMESTAMP_DER);
    }

    #[test]
    fn a_word_outside_the_vocabulary_is_rejected_when_read() {
        for declares in [
            "saf = \"SAF_6\"",
            "saf = \"ERROR\"",
            "completes = { contains_oid = \"uno.dos\" }",
            "completes = { contains_bytes = \"%%%\" }",
            "completes = { ends_with = \"%%EOF\" }",
            "person = { yes = \"sí\", no = \"no\", yes_means = \"no-observable\" }",
        ] {
            assert!(
                the_catalogue_in(&format!(
                    "[[check]]\nid = \"a\"\nset = \"errores\"\nchapter = \"1\"\n\
                     citation = \"A\"\nstatement = \"B\"\n{declares}\n"
                ))
                .is_err(),
                "{declares}"
            );
        }
    }
}
