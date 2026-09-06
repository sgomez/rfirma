//! Sesión de firma de un trámite de sede: prefirma filtrada y postfirma en memoria (ADR-0001, ADR-0016).

use std::collections::BTreeMap;

use crate::app::filtering::{self, FilterEngine};
use crate::app::frontier;
use crate::app::in_hand::DocumentInHand;
use crate::commands::orders::SigningOrder;
use crate::commands::views::Failure;
use crate::isolate::Isolate;
use crate::memory::{ListedCertificates, OpenedDocuments};
use crate::pkcs11::{self, Store, StoreSecret, TokenError};
use crate::protocol::{SafCode, SiteFilter};

use super::{
    admitted_bytes_with_situation, config_for, on_the_bridge_with_situation, open_the_cycle,
    take_signed_cycle, CycleFailure, SignedCycle, SigningSession,
};

/// Prefirma de un trámite de sede aplicando los filtros solicitados.
pub fn begin_for_the_site<E: FilterEngine>(
    site: &SiteSigning<'_, E>,
    order: &SigningOrder,
    stores: &[Store],
    listed: &ListedCertificates,
    opened: &OpenedDocuments,
    isolate: &Isolate,
    session: &SigningSession,
) -> Result<StoreSecret, SiteRefusal> {
    let document = DocumentInHand::taken(opened, &order.document)
        .map_err(|failure| SiteRefusal::new(SafCode::CannotReadData, failure))?;
    let bytes = admitted_bytes_with_situation(document.document())?;
    let found = pkcs11::list_certificates_across(stores)?;
    let chosen = filtering::usable_certificate_for_the_site(
        site.engine,
        site.filter,
        &found,
        &order.certificate,
        listed,
    )
    .map_err(|failure| SiteRefusal::new(SafCode::NoCertificatesInKeystore, failure))?;
    let config = config_for(order, chosen)
        .map_err(|failure| SiteRefusal::new(SafCode::VisibleSignature, failure))?;
    let reference = chosen.reference().clone();
    let chain = vec![chosen.der().to_vec()];
    Ok(open_the_cycle(
        document,
        bytes,
        config,
        reference,
        chain,
        site.from_the_site,
        isolate,
        session,
    )?)
}

/// Resultado de rechazo de un trámite de sede con código de protocolo y detalle local.
#[derive(Debug)]
pub struct SiteRefusal {
    code: SafCode,
    failure: Failure,
}

impl SiteRefusal {
    /// Une el código del catálogo con la situación que lo decidió.
    pub fn new(code: SafCode, failure: Failure) -> Self {
        Self { code, failure }
    }

    /// Código que se enviará a la sede.
    pub fn code(&self) -> SafCode {
        self.code
    }

    /// Situación para la ventana.
    pub fn failure(&self) -> &Failure {
        &self.failure
    }

    /// Convierte el rechazo en el fallo para la ventana.
    pub fn into_failure(self) -> Failure {
        self.failure
    }
}

impl From<CycleFailure> for SiteRefusal {
    fn from(failure: CycleFailure) -> Self {
        Self::new(frontier::code_of_cycle(&failure), Failure::from(failure))
    }
}

impl From<TokenError> for SiteRefusal {
    fn from(error: TokenError) -> Self {
        Self::new(frontier::code_of_token(error.situation()), error.into())
    }
}

/// Contexto de firma requerido por un trámite de sede.
pub struct SiteSigning<'a, E: FilterEngine> {
    /// Motor de filtros sobre certificados.
    pub engine: &'a E,
    /// Filtro de certificados declarado por la sede.
    pub filter: &'a SiteFilter,
    /// Parámetros adicionales declarados por la sede.
    pub from_the_site: &'a BTreeMap<String, String>,
}

/// Firma de un trámite de sede lista para transmitir.
pub struct SiteSignature {
    /// Bytes del PDF firmado.
    pub signed: Vec<u8>,
    /// Certificado firmante en formato DER.
    pub signer_der: Vec<u8>,
}

/// Postfirma de un trámite de sede que devuelve el resultado sin persistir en disco (ADR-0011).
pub fn finish_for_the_site(
    isolate: &Isolate,
    session: &SigningSession,
) -> Result<SiteSignature, SiteRefusal> {
    let SignedCycle {
        cycle,
        signature,
        seal,
        signer_der,
        ..
    } = take_signed_cycle(session)
        .map_err(|failure| SiteRefusal::new(SafCode::SignatureFailed, failure))?;

    let signed = on_the_bridge_with_situation(isolate, move |bridge| {
        cycle.postsign(bridge, &signature, &seal)
    })?;

    Ok(SiteSignature { signed, signer_der })
}

#[cfg(test)]
mod tests {
    use super::{begin_for_the_site, SiteSigning};
    use crate::app::filtering::FilterEngine;
    use crate::app::fixtures::{a_certificate, an_order};
    use crate::app::signing::{config_for, SigningSession};
    use crate::commands::orders::SigningOrder;
    use crate::isolate::Isolate;
    use crate::memory::{ListedCertificates, OpenedDocuments};
    use crate::protocol::{SafCode, SiteFilter};
    use std::collections::BTreeMap;

    const SOURCE: &str = include_str!("site.rs");

    fn production_half() -> &'static str {
        SOURCE
            .split_once("\nmod tests {")
            .map(|(before, _)| before)
            .unwrap_or(SOURCE)
    }

    #[test]
    fn the_postsign_of_a_site_errand_writes_nothing_anywhere() {
        let site_postsign = production_half()
            .split_once("pub fn finish_for_the_site(")
            .expect("la postfirma de la sede sigue aqui")
            .1;

        for forbidden in [
            "documents::deliver",
            "recents::",
            "session.delivered",
            "remember_the_certificate",
        ] {
            assert!(
                !site_postsign.contains(forbidden),
                "la postfirma de la sede llama a «{forbidden}»: el documento que manda una sede no \
                 deja rastro y rFirma no guarda ficheros por orden suya"
            );
        }
    }

    #[test]
    fn the_presign_of_a_site_errand_checks_the_filter_again_before_the_pin() {
        let site_presign = production_half()
            .split_once("pub fn begin_for_the_site<")
            .expect("la prefirma de la sede sigue aqui")
            .1
            .split_once("\n/// ")
            .expect("y termina donde empieza la siguiente")
            .0;

        assert!(
            site_presign.contains("filtering::usable_certificate_for_the_site("),
            "el filtro de la sede se vuelve a comprobar antes de pedir el secreto"
        );
        assert!(
            !site_presign.contains("plan_signature("),
            "y no por el camino local, que no sabe nada de la sede"
        );
    }

    #[test]
    fn a_signature_the_site_placed_carries_no_geometry_of_our_own() {
        let order = SigningOrder {
            placement: None,
            ..an_order()
        };

        let config = config_for(&order, &a_certificate("FIRMA", &[])).expect("no hay que colocar");

        assert_eq!(config.placement, None);
        for key in crate::signing::Setting::Geometry.keys() {
            assert!(!config.extra_params().contains_key(*key), "'{key}' es suya");
        }
    }

    #[test]
    fn a_site_signature_cannot_begin_on_a_document_that_is_not_open() {
        let order = SigningOrder {
            document: "00000000000000000000000000000000".to_owned(),
            ..an_order()
        };
        let engine = NoEngine;

        let failure = begin_for_the_site(
            &SiteSigning {
                engine: &engine,
                filter: &SiteFilter::default(),
                from_the_site: &BTreeMap::new(),
            },
            &order,
            &[],
            &ListedCertificates::new(),
            &OpenedDocuments::new(),
            &Isolate::start(),
            &SigningSession::default(),
        )
        .expect_err("ese documento no esta abierto");

        assert_eq!(failure.failure().situation, "documentUnreadable");
        assert_eq!(
            failure.code(),
            SafCode::CannotReadData,
            "y la sede recibe el codigo de lo que ha pasado, no uno para todo"
        );
    }

    struct NoEngine;

    impl FilterEngine for NoEngine {
        fn select(
            &self,
            _properties: &str,
            _certificates: &str,
        ) -> Result<Vec<usize>, crate::ffi::BridgeError> {
            unreachable!("no se llega a filtrar nada")
        }
    }
}
