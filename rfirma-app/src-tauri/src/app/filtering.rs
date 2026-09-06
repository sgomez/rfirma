//! **El listado que la sede acepta**: los criterios de rFirma primero, y la
//! expresión de la sede después, aplicada por el motor prestado (ID-252,
//! ID-258).
//!
//! Es el segundo caso de uso que cruza la frontera nativa, y el único que la
//! cruza **sin firmar nada**: lo que viaja es el DER público de cada
//! certificado y lo que vuelve son los índices que pasan. No hay sello ni
//! sesión porque no hay dos fases que atar (ADR-0016).
//!
//! # El orden no es un detalle
//!
//! Primero los criterios de rFirma —clave privada y `CKA_LABEL`, los del ID-07
//! y el ADR-0010—, y **sólo entonces** la expresión de la sede. Así, un listado
//! vacío significa inequívocamente «la sede los excluyó» y no «este equipo no
//! tenía ninguno», que es lo que permite escribir esa pantalla (ID-258, ID-278).
//! Al revés, las dos causas serían la misma lista vacía.
//!
//! # Y se vuelve a mirar antes del PIN
//!
//! [`usable_certificate_for_the_site`] repite el filtro justo antes de pedir el
//! secreto (ID-259). No es porque el filtro caduque: es que la ventana no es
//! quien hace cumplir una restricción de la sede. Entre listar y firmar puede
//! haber pasado cualquier cosa, y el único sitio donde esa restricción se
//! sostiene es aquí.
//!
//! # Este módulo no interpreta ni un criterio
//!
//! La lista blanca —qué criterios se dejan cruzar— es de
//! [`crate::protocol::filters`], y quién decide qué certificado pasa es el
//! motor de `afirma-keystores-filters`. Aquí sólo se ordenan las dos cosas y se
//! traduce el resultado.

use base64::Engine as _;

use crate::commands::Failure;
use crate::ffi::BridgeError;
use crate::memory::ListedCertificates;
use crate::pkcs11::{self, Store, TokenCertificate};
use crate::protocol::SiteFilter;

/// Quien sabe acotar un listado con la expresión de la sede.
///
/// En producción es el puente; en las pruebas, un doble. La costura existe
/// porque el orden de las dos criba —la de rFirma y la de la sede— es una
/// decisión que hay que poder probar en grada A, sin `librfirma_crypto.so`
/// delante (TD-20).
pub trait FilterEngine {
    /// Los índices, sobre la lista que se le da y en su orden, que pasan.
    fn select(
        &self,
        filter_properties: &str,
        certificates_b64: &str,
    ) -> Result<Vec<usize>, BridgeError>;
}

/// **Caso de uso.** El listado que la sede acepta, de los tokens conectados.
///
/// Las dos cribas, en el orden del ID-258: los criterios de rFirma los aplica
/// [`pkcs11::list_certificates_across`] —firma con él y tiene etiqueta— y sólo
/// lo que sobrevive a eso se le enseña al motor.
pub fn listing_the_site_accepts<E: FilterEngine>(
    engine: &E,
    stores: &[Store],
    filter: &SiteFilter,
) -> Result<Vec<TokenCertificate>, Failure> {
    let ours = pkcs11::list_certificates_across(stores)?;
    keep_what_the_site_accepts(engine, filter, ours)
}

/// La expresión de la sede aplicada a un listado que **ya** pasó por los
/// criterios de rFirma.
///
/// Se puede llamar sola porque el listado y la firma llegan por caminos
/// distintos, pero nunca antes de la criba de rFirma: quien la llame con un
/// listado sin cribar convierte el `[]` del ID-258 en una respuesta ambigua.
pub fn keep_what_the_site_accepts<E: FilterEngine>(
    engine: &E,
    filter: &SiteFilter,
    certificates: Vec<TokenCertificate>,
) -> Result<Vec<TokenCertificate>, Failure> {
    let accepted = accepted_indexes(engine, filter, &certificates)?;

    Ok(certificates
        .into_iter()
        .enumerate()
        .filter(|(index, _)| accepted.contains(index))
        .map(|(_, certificate)| certificate)
        .collect())
}

/// **Caso de uso.** El certificado que pide la orden, si sigue estando, sirve
/// para firmar **y la sede lo sigue aceptando** (ID-259).
///
/// Es la última comprobación antes del PIN, y las tres cosas se miran aquí: el
/// estado del token lo repasa [`super::certificates::usable_certificate`] y la
/// restricción de la sede, la vuelta al motor de abajo. Que el certificado
/// estuviera en la lista que la ventana enseñó no basta: la ventana no es quien
/// hace cumplir lo que pidió la sede.
pub fn usable_certificate_for_the_site<'a, E: FilterEngine>(
    engine: &E,
    filter: &SiteFilter,
    certificates: &'a [TokenCertificate],
    handle: &str,
    listed: &ListedCertificates,
) -> Result<&'a TokenCertificate, Failure> {
    let chosen = super::certificates::usable_certificate(certificates, handle, listed)?;

    let only_this_one = std::slice::from_ref(chosen).to_vec();
    if accepted_indexes(engine, filter, &only_this_one)?.is_empty() {
        return Err(Failure::new(
            "certificateNotFound",
            format!(
                "la sede excluye {}: su filtro ya no lo acepta",
                chosen.reference().label()
            ),
        ));
    }

    Ok(chosen)
}

/// Los índices que el motor acepta, con la expresión cruzando **literal**
/// (ID-256).
fn accepted_indexes<E: FilterEngine>(
    engine: &E,
    filter: &SiteFilter,
    certificates: &[TokenCertificate],
) -> Result<Vec<usize>, Failure> {
    let accepted = engine.select(&filter.as_java_properties(), &to_der_payload(certificates))?;

    if let Some(out_of_range) = accepted.iter().find(|index| **index >= certificates.len()) {
        return Err(Failure::new(
            "bridgeFailed",
            format!("el motor de filtros ha devuelto el indice {out_of_range}"),
        ));
    }
    Ok(accepted)
}

/// El listado tal y como lo recibe el puente: Base64 del DER separado por `;`.
fn to_der_payload(certificates: &[TokenCertificate]) -> String {
    certificates
        .iter()
        .map(|certificate| base64::engine::general_purpose::STANDARD.encode(certificate.der()))
        .collect::<Vec<_>>()
        .join(";")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::fixtures::{a_certificate, listed_from};
    use crate::protocol::site_filter;
    use std::cell::RefCell;

    /// **Grada A**: ni librería nativa ni token. El motor es un doble que apunta
    /// lo que se le pidió y contesta lo que se le diga.
    struct AnEngine {
        answer: Vec<usize>,
        asked: RefCell<Vec<(String, String)>>,
    }

    impl AnEngine {
        fn answering(answer: &[usize]) -> Self {
            Self {
                answer: answer.to_vec(),
                asked: RefCell::new(Vec::new()),
            }
        }
    }

    impl FilterEngine for AnEngine {
        fn select(
            &self,
            filter_properties: &str,
            certificates_b64: &str,
        ) -> Result<Vec<usize>, BridgeError> {
            self.asked
                .borrow_mut()
                .push((filter_properties.to_owned(), certificates_b64.to_owned()));
            Ok(self.answer.clone())
        }
    }

    fn a_filter(expression: &str) -> SiteFilter {
        site_filter(&[("filters".to_owned(), expression.to_owned())]).expect("es aceptable")
    }

    /// La expresión llega al motor **tal cual la escribió la sede**, y los
    /// certificados con ella en el mismo orden en que se listaron (ID-256).
    #[test]
    fn the_expression_and_the_listing_reach_the_engine_untouched() {
        let engine = AnEngine::answering(&[0]);
        let certificates = vec![a_certificate("UNO", &[0x01]), a_certificate("DOS", &[0x02])];

        keep_what_the_site_accepts(&engine, &a_filter("subject.contains:PEREZ"), certificates)
            .expect("el motor contesta");

        let asked = engine.asked.borrow();
        assert_eq!(asked.len(), 1);
        assert_eq!(asked[0].0, "filters=subject.contains:PEREZ\n");
        assert_eq!(asked[0].1, "AQ==;Ag==");
    }

    /// Lo que vuelve son índices, y lo que sale son **esos** certificados.
    #[test]
    fn only_the_certificates_the_engine_picked_come_back() {
        let engine = AnEngine::answering(&[1]);
        let certificates = vec![a_certificate("UNO", &[0x01]), a_certificate("DOS", &[0x02])];

        let kept = keep_what_the_site_accepts(&engine, &a_filter("ssl:true"), certificates)
            .expect("el motor contesta");

        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].reference().label(), "DOS");
    }

    /// El caso del ID-258: cuando la sede los excluye a todos, el listado sale
    /// **vacío y sin fallo**. Es una respuesta, no un error: la pantalla que la
    /// cuenta es la del ID-278.
    #[test]
    fn a_site_that_excludes_them_all_gives_an_empty_listing_and_not_a_failure() {
        let engine = AnEngine::answering(&[]);
        let certificates = vec![a_certificate("UNO", &[0x01])];

        let kept = keep_what_the_site_accepts(&engine, &a_filter("dnie:true"), certificates)
            .expect("excluirlos todos no es un fallo");

        assert!(kept.is_empty());
    }

    /// Sin filtro declarado **se llama igual**: el motor añade el `nonexpired`
    /// implícito de la ETSI, y eso sólo pasa en este camino (ID-254).
    #[test]
    fn a_site_that_declares_nothing_still_reaches_the_engine() {
        let engine = AnEngine::answering(&[0]);
        let certificates = vec![a_certificate("UNO", &[0x01])];

        keep_what_the_site_accepts(&engine, &SiteFilter::default(), certificates)
            .expect("el motor contesta");

        assert_eq!(engine.asked.borrow()[0].0, "");
    }

    /// ID-259: el filtro se vuelve a comprobar antes del PIN. Un certificado que
    /// está en el token, sirve para firmar y **la sede ya no acepta** no llega a
    /// pedir el secreto.
    #[test]
    fn a_certificate_the_site_no_longer_accepts_is_refused_before_the_pin() {
        let engine = AnEngine::answering(&[]);
        let certificates = [a_certificate("FIRMA", &[])];
        let (listed, handles) = listed_from(&certificates);

        let failure = usable_certificate_for_the_site(
            &engine,
            &a_filter("subject.contains:OTRO"),
            &certificates,
            &handles[0],
            &listed,
        )
        .expect_err("la sede lo excluye");

        assert_eq!(failure.situation, "certificateNotFound");
        assert!(failure.detail.contains("FIRMA"), "{}", failure.detail);
    }

    /// Y el estado se sigue mirando **antes** que el filtro: un certificado
    /// ilegible no llega ni a cruzar la frontera.
    #[test]
    fn an_unusable_certificate_never_reaches_the_engine() {
        let engine = AnEngine::answering(&[0]);
        let certificates = [a_certificate("FIRMA", &[0x00, 0x01, 0x02])];
        let (listed, handles) = listed_from(&certificates);

        let failure = usable_certificate_for_the_site(
            &engine,
            &a_filter("ssl:true"),
            &certificates,
            &handles[0],
            &listed,
        )
        .expect_err("no es legible");

        assert!(failure.detail.contains("Unreadable"), "{}", failure.detail);
        assert!(
            engine.asked.borrow().is_empty(),
            "un certificado que ya no sirve no tiene por que cruzar la frontera"
        );
    }

    /// Un índice que no señala a ninguna fila es una respuesta imposible, y se
    /// dice: quedarse callado sería devolver un listado más corto sin más.
    #[test]
    fn an_index_outside_the_listing_is_a_failure_and_not_a_silent_shorter_list() {
        let engine = AnEngine::answering(&[7]);
        let certificates = vec![a_certificate("UNO", &[0x01])];

        let failure = keep_what_the_site_accepts(&engine, &a_filter("ssl:true"), certificates)
            .expect_err("7 no es una fila");

        assert!(failure.detail.contains('7'), "{}", failure.detail);
    }

    /// El orden del ID-258, leído del código: los criterios de rFirma se aplican
    /// **antes** de que la expresión de la sede llegue al motor. Al revés, un
    /// listado vacío no distinguiría «la sede los excluyó» de «aquí no había
    /// ninguno».
    #[test]
    fn the_rfirma_criteria_run_before_the_expression_of_the_site() {
        let source = include_str!("filtering.rs");
        let body = source
            .split_once("pub fn listing_the_site_accepts")
            .expect("el caso de uso sigue aqui")
            .1;
        let ours = body
            .find("pkcs11::list_certificates_across")
            .expect("los criterios de rFirma");
        let theirs = body
            .find("keep_what_the_site_accepts")
            .expect("y despues los de la sede");

        assert!(
            ours < theirs,
            "la expresion de la sede se estaria aplicando antes que los criterios de rFirma"
        );
    }
}
