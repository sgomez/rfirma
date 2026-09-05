//! **La política de firma que declara la sede**: `expPolicy` expandido por el
//! motor del original (ID-266).
//!
//! Es el tercer caso de uso que cruza la frontera nativa, y el segundo que la
//! cruza **sin firmar nada** (el otro es [`super::filtering`]). Lo que viaja es
//! el bloque de `extraParams` tal y como lo mandó la sede, y lo que vuelve es
//! el mismo bloque con `expPolicy` convertido en las claves que le
//! corresponden. No hay sello ni sesión porque no hay dos fases que atar
//! (ADR-0016).
//!
//! # Por qué esto no se reimplementa
//!
//! `expPolicy=FirmaAGE` se expande en el identificador de la política, su
//! huella, el algoritmo de la huella y el calificador. Escribir esos cuatro
//! valores a mano en Rust es una copia que envejece sola, y **expandirlos mal
//! es firmar con una política distinta de la declarada**: la firma sale, nada
//! falla, y lo que la sede recibe no es lo que pidió. `ExtraParamsProcessor` ya
//! vive dentro de `afirma-core`, así que se usa el del original (ID-266).
//!
//! # Quién manda cuando las dos partes dicen algo de la misma clave
//!
//! Los `extraParams` de la sede son **la base**, y los seis ajustes de rFirma
//! ([`crate::signing::SignatureConfig`]) se escriben **encima**. La sede decide
//! la política; rFirma decide el recuadro visible, que es lo que la persona ve
//! y consiente. La única clave que las dos tocan es `signatureSubFilter`, y las
//! dos escriben el mismo valor —`ETSI.CAdES.detached`—, porque es el que la
//! política de la AGE exige y el que rFirma envía siempre; un subfiltro
//! distinto declarado por la sede ni siquiera llega hasta aquí: el expansor lo
//! rechaza.

use std::collections::BTreeMap;

use crate::ffi::{BridgeError, ExpandRequest, NativeBridge};
use crate::protocol::{pairs_of, PADES};
use crate::signing::to_java_properties;

/// Quien sabe expandir la política de firma que declara la sede.
///
/// En producción es el puente; en las pruebas, un doble. La costura existe por
/// lo mismo que la de [`super::filtering::FilterEngine`]: el orden en el que se
/// mezclan los `extraParams` de la sede y los ajustes de rFirma es una decisión
/// que hay que poder probar en grada A, sin `librfirma_crypto.so` delante
/// (TD-20).
pub trait PolicyEngine {
    /// El bloque `java.util.Properties` expandido, a partir del de la sede.
    fn expand(&self, extra_params: &str, format: &str) -> Result<String, BridgeError>;
}

impl PolicyEngine for NativeBridge {
    fn expand(&self, extra_params: &str, format: &str) -> Result<String, BridgeError> {
        self.expand_extra_params(ExpandRequest {
            extra_params,
            format,
        })
    }
}

/// **Caso de uso.** Los `extraParams` que la sede declaró, con su política ya
/// expandida.
///
/// Sin `expPolicy` el motor devuelve lo mismo que entró, así que **se llama
/// igual**: preguntar aquí si la clave está sería reimplementar en Rust la
/// primera línea de la decisión que se ha ido a buscar a Java, y la respuesta
/// dejaría de valer en cuanto el original expandiera una clave más.
///
/// Devuelve la situación del puente **sin traducir**: quien la convierte en un
/// código para la sede es [`super::frontier`], y ese es el único sitio (ID-288).
pub fn expanded_for_the_site<E: PolicyEngine>(
    engine: &E,
    declared: &[(String, String)],
) -> Result<BTreeMap<String, String>, BridgeError> {
    let block = to_java_properties(&declared.iter().cloned().collect());
    let expanded = engine.expand(&block, PADES)?;
    Ok(pairs_of(&expanded).into_iter().collect())
}

/// Los `extraParams` que se le envían al puente: los de la sede debajo y los
/// seis ajustes de rFirma encima.
///
/// El orden es la decisión, y va escrita en una sola línea a propósito: si
/// alguien la invierte, la sede podría reescribir el recuadro que la persona
/// acaba de ver y consentir.
pub fn merged_with(
    from_the_site: BTreeMap<String, String>,
    ours: BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut merged = from_the_site;
    merged.extend(ours);
    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// **Grada A**: ni token, ni librería nativa, ni socket (TD-51, TD-52).
    ///
    /// El doble no expande nada: apunta el bloque que se le dio y devuelve el
    /// que se le programó. Lo que se prueba aquí es el orden y el trasiego, no
    /// la expansión, que es del original.
    struct AnEngine {
        asked: RefCell<Vec<(String, String)>>,
        answer: Result<String, ()>,
    }

    impl AnEngine {
        fn answering(block: &str) -> Self {
            Self {
                asked: RefCell::new(Vec::new()),
                answer: Ok(block.to_owned()),
            }
        }

        fn that_refuses_the_policy() -> Self {
            Self {
                asked: RefCell::new(Vec::new()),
                answer: Err(()),
            }
        }
    }

    impl PolicyEngine for AnEngine {
        fn expand(&self, extra_params: &str, format: &str) -> Result<String, BridgeError> {
            self.asked
                .borrow_mut()
                .push((extra_params.to_owned(), format.to_owned()));
            self.answer.clone().map_err(|()| {
                BridgeError::IncompatiblePolicy("politica que no se puede aplicar".to_owned())
            })
        }
    }

    fn params(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect()
    }

    fn declared(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect()
    }

    /// Lo que se le entrega al motor es el bloque de la sede tal cual, y el
    /// formato es siempre PAdES: es el único que rFirma atiende (ID-263).
    #[test]
    fn the_declared_block_reaches_the_engine_as_a_pades_expansion() {
        let engine = AnEngine::answering("policyIdentifier=urn:oid:2.16.724.1.3.1.1.2.1.9\n");

        let expanded =
            expanded_for_the_site(&engine, &declared(&[("expPolicy", "FirmaAGE")])).expect("ok");

        assert_eq!(
            engine.asked.borrow().as_slice(),
            [("expPolicy=FirmaAGE\n".to_owned(), "pades".to_owned())]
        );
        assert_eq!(
            expanded,
            params(&[("policyIdentifier", "urn:oid:2.16.724.1.3.1.1.2.1.9")])
        );
    }

    /// **ID-266**: la política que no se puede aplicar no se ignora, y lo que
    /// sube es la situación del puente con nombre propio, no un fallo de firma.
    #[test]
    fn a_policy_that_cannot_be_applied_is_not_signed_around() {
        let engine = AnEngine::that_refuses_the_policy();

        let refused = expanded_for_the_site(&engine, &declared(&[("expPolicy", "Inventada")]));

        assert!(refused.is_err());
    }

    /// El orden de la mezcla: la sede pone la política y rFirma el recuadro.
    #[test]
    fn what_rfirma_decides_is_written_over_what_the_site_declared() {
        let merged = merged_with(
            params(&[
                ("layer2Text", "lo que la sede quisiera"),
                ("policyIdentifier", "urn:oid:1"),
            ]),
            params(&[("layer2Text", "Firmado por: Ada Lovelace Byron")]),
        );

        assert_eq!(
            merged,
            params(&[
                ("layer2Text", "Firmado por: Ada Lovelace Byron"),
                ("policyIdentifier", "urn:oid:1"),
            ])
        );
    }
}
