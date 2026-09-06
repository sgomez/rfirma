//! **La prefirma en seco**: el ciclo trifásico entero con un `PK1` inventado,
//! para que la ventana pinte dentro del recuadro lo que va a quedar de verdad
//! (ID-136).
//!
//! No es un dibujo del sello: es **el sello**. El PDF que sale de aquí lo ha
//! compuesto la misma postfirma que compone el firmado, con la misma
//! configuración que armará [`crate::app::signing::plan_signature`], así que el
//! recuadro que se ve tiene la tipografía, el recorte y el reparto que va a
//! tener. El sondeo del #115 lo midió: los bytes visibles son idénticos a los
//! del firmado real, y `pdf.js` de fábrica los pinta sin nada en medio. Por eso
//! aquí no hay código de dibujo.
//!
//! ```text
//!   1. prefirma   Java   una sola, para TODO el conjunto de páginas
//!   2. firma      ——     no la hay: el PK1 se inventa (no se pide el PIN)
//!   3. postfirma  Java   ensambla el PDF y se devuelve a la ventana
//! ```
//!
//! # Dos condiciones que no se negocian
//!
//! - **No pide PIN.** La fase 2 no ocurre: el `PK1` sale de
//!   [`TokenSignature::invented`] y el certificado se lee sin secreto, que es
//!   lo que ya hace el listado. Una vista previa que pidiera el PIN sería
//!   teclearlo dos veces por firma.
//! - **No toca el disco de destino.** Lo que devuelve son bytes, y aquí no se
//!   llama ni a [`crate::app::documents`] para entregar, ni a
//!   [`crate::app::recents`] para anotar: la insignia `Firmado` la escribe solo
//!   la postfirma de verdad (ID-76), y una vista previa que dejara un fichero
//!   dejaría un PDF con una firma inválida en la carpeta de la persona.
//!
//! # Una sola prefirma para todo el conjunto (ID-110)
//!
//! El widget replicado es idéntico en todas las páginas, y `signaturePages`
//! lleva el conjunto entero en un solo `extraParams`: veinte páginas son una
//! prefirma, no veinte. Aquí eso no se decide, se hereda —la configuración la
//! arma el mismo plan que la firma— y hay una guarda abajo que se pone roja si
//! alguien mete un bucle por páginas.
//!
//! # El coste es la razón de que esto se pida a mano
//!
//! Componer el PDF cuesta lo que cuesta el ciclo entero menos el token. En el
//! equipo de desarrollo, ≈0,15 s en un PDF normal y ≈1,9 s con 507 MB de RSS en
//! un escaneado de 37 MB. Ese número es lo que sostiene el ID-109: en un
//! documento grande la vista previa se pide, no se refresca sola.

use crate::app::cycle::{self, SigningRequest, TokenSignature};
use crate::app::documents;
use crate::app::signing::{admitted_bytes, on_the_bridge, plan_signature};
use crate::commands::orders::SigningOrder;
use crate::commands::views::Failure;
use crate::isolate::Isolate;
use crate::memory::{ListedCertificates, OpenedDocuments};
use crate::pkcs11::Store;
use crate::signing::AdmissibleDocument;

/// **Caso de uso.** El PDF compuesto con el sello que va a quedar, sin firmar.
///
/// Recorre lo mismo que [`crate::app::signing::begin`] hasta la prefirma —el documento sale
/// del registro por su identificador (ID-62), y se rechaza lo que no se puede
/// firmar antes de cruzar la frontera— y después se salta el token: el `PK1` se
/// inventa y la postfirma ensambla. Lo que vuelve son los bytes del PDF, que es
/// lo único que la ventana necesita para pintarlo.
pub fn compose(
    order: &SigningOrder,
    stores: &[Store],
    listed: &ListedCertificates,
    opened: &OpenedDocuments,
    isolate: &Isolate,
) -> Result<Vec<u8>, Failure> {
    let document = documents::opened_document(opened, &order.document)?;
    let bytes = admitted_bytes(&document)?;
    let (config, reference, chain) = plan_signature(stores, listed, order)?;

    on_the_bridge(isolate, move |bridge| {
        // La misma comprobación que en la firma, y por el mismo motivo: el tipo
        // que la garantiza presta los bytes, y los bytes han viajado al hilo
        // del isolate.
        let document = AdmissibleDocument::check(&bytes)?;
        let cycle = cycle::presign(
            bridge,
            SigningRequest {
                document,
                chain: &chain,
                config: &config,
                from_the_site: &crate::app::cycle::NOTHING_FROM_A_SITE,
                certificate: &reference,
            },
        )?;
        // El sello no se lee ni se reconstruye: se transporta y se devuelve
        // igual que en la firma (ADR-0016). Aquí el transporte es esta línea.
        let seal = cycle.seal_in_transit();
        cycle.postsign(bridge, &TokenSignature::invented(), &seal)
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crate::signing::{PadesRect, PageSet, Placement, SignatureConfig};

    /// **Grada A**: sin puente y sin token. Lo que se comprueba aquí es la
    /// forma del recorrido —cuántas veces cruza y qué no toca—; que el PDF
    /// compuesto se pinte igual que el firmado lo midió el sondeo del #115 y lo
    /// vigila la grada C con `pdfsig` (TD-33).
    ///
    /// La mitad de producción de este mismo fichero, cortada por `mod tests`
    /// para que las guardas no se lean a sí mismas.
    fn production_half() -> &'static str {
        include_str!("preview.rs")
            .split_once("\nmod tests {")
            .map(|(before, _)| before)
            .unwrap_or_default()
    }

    /// **ID-110**: con varias páginas se pide **una sola** prefirma.
    ///
    /// La mitad que se puede afirmar sin puente es que el conjunto entero cabe
    /// en un `extraParams`: veinte páginas son un `signaturePages`, no veinte
    /// peticiones. La otra mitad —que nadie itere— la fija
    /// `the_bridge_is_crossed_twice_and_never_once_per_page`.
    #[test]
    fn a_page_set_of_twenty_travels_as_a_single_presign_request() {
        let pages = PageSet::only(1..=20).expect("veinte paginas no es vacio");
        let config = SignatureConfig {
            placement: Some(Placement {
                rect: PadesRect {
                    lower_left_x: 48,
                    lower_left_y: 179,
                    upper_right_x: 250,
                    upper_right_y: 260,
                },
                pages,
            }),
            layer2_text: String::new(),
            rubric_image: None,
            sign_reason: None,
            allow_unregistered_signatures: false,
        };

        let params = config.extra_params();

        assert_eq!(
            params.get("signaturePages").map(String::as_str),
            Some("1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20"),
            "el conjunto entero viaja en un solo extraParams"
        );
    }

    /// El puente se cruza **dos veces** —prefirma y postfirma— y ninguna de
    /// ellas está dentro de un bucle.
    ///
    /// Se lee la fuente porque lo que se vigila no es el resultado de una
    /// llamada sino que no haya *n* llamadas: un `for` por páginas alrededor de
    /// la prefirma daría el mismo PDF y veinte veces el coste, y ninguna
    /// aserción sobre bytes lo vería.
    #[test]
    fn the_bridge_is_crossed_twice_and_never_once_per_page() {
        let source = production_half();

        assert_eq!(
            source.matches("cycle::presign(").count(),
            1,
            "la prefirma se pide una sola vez, sea cual sea el conjunto de paginas"
        );
        assert_eq!(
            source.matches(".postsign(").count(),
            1,
            "y la postfirma ensambla una sola vez"
        );
        for loop_keyword in ["for ", "while ", ".iter()", ".map("] {
            assert!(
                !source.contains(loop_keyword),
                "«{loop_keyword}» en la vista previa: una prefirma por pagina es el ID-110 roto"
            );
        }
    }

    /// **No pide PIN y no toca el disco de destino.**
    ///
    /// Las dos condiciones del ID-136 se ven mirando lo que esta fuente **no**
    /// nombra: sin `pin` no hay fase 2, y sin el entregador ni la bandeja no
    /// hay fichero que quede escrito ni fila que gane la insignia `Firmado`
    /// (ID-76).
    #[test]
    fn the_dry_run_neither_asks_for_the_pin_nor_writes_anything() {
        let source = production_half();

        // Identificadores, no subcadenas: «pinta» lleva un «pin» dentro y una
        // guarda que se pusiera roja por la prosa no diria nada de la fase 2.
        let words: BTreeSet<&str> = source
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .filter(|word| !word.is_empty())
            .collect();
        for forbidden in ["pin", "sign_on_token", "deliver", "note_signed", "Signed"] {
            assert!(
                !words.contains(forbidden),
                "«{forbidden}» en la vista previa: deja de ser en seco"
            );
        }
    }

    /// El `PK1` de la vista previa es el inventado, y no hay otra forma de
    /// llegar a la postfirma sin token.
    #[test]
    fn the_pkcs1_of_the_dry_run_is_the_invented_one() {
        assert!(production_half().contains("TokenSignature::invented()"));

        let invented = super::TokenSignature::invented();
        assert_eq!(invented.raw().len(), 256, "una firma RSA de 2048 bits");
        assert!(
            invented.raw().iter().all(|byte| *byte == 0),
            "no lo ha calculado ningun token, y se nota"
        );
    }

    /// Nadie más se inventa un `PK1`: la vista previa es el único sitio.
    ///
    /// Si `TokenSignature::invented` apareciera en el recorrido de la firma, un
    /// PDF sin firmar saldría por la puerta con cara de firmado.
    #[test]
    fn only_the_dry_run_invents_a_pkcs1() {
        let signing = include_str!("signing.rs")
            .split_once("\nmod tests {")
            .map(|(before, _)| before)
            .unwrap_or_default();

        assert!(
            !signing.contains("TokenSignature::invented"),
            "el recorrido de la firma se esta inventando el PK1"
        );
    }
}
