//! **La sesión de firma de un trámite de sede** (ID-263, ID-264, ID-286): la
//! prefirma que vuelve a pasar el filtro de la sede y la postfirma que **no
//! escribe nada**.
//!
//! Es el gemelo de sede de [`super`], y comparte con él el ciclo a medias
//! ([`SigningSession`]) y la firma en el token: la fase que toca la clave
//! privada no sabe de sedes (ADR-0001), así que el PIN entra por la misma orden
//! en los dos recorridos. Lo que cambia son las dos puntas: con qué se firma
//! —el certificado que la sede sigue aceptando (ID-259) y la política que
//! declaró (ID-266)— y qué se hace con el resultado —devolverlo, y nada más—.
//!
//! Cada negativa vuelve con **su** código del catálogo ([`SiteRefusal`]) y no
//! con uno solo para todas (ID-292): aquí la situación todavía tiene tipo, y
//! es el último sitio donde se puede decidir.

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

/// **Caso de uso.** Prefirma de un trámite de sede (ID-263).
///
/// Es [`begin`] con dos diferencias, y las dos son de la sede:
///
/// 1. **El filtro se vuelve a comprobar antes del PIN** (ID-259), y por eso el
///    certificado no lo resuelve [`plan_signature`] sino
///    [`filtering::usable_certificate_for_the_site`]: que estuviera en la lista
///    que la ventana enseñó no basta, porque la ventana no es quien hace
///    cumplir lo que pidió la sede.
/// 2. Los `extraParams` que la sede declaró viajan **debajo** de los seis
///    ajustes de rFirma (ID-266, [`crate::app::policies`]).
///
/// Y una tercera, que es de dónde sale lo que la sede recibe: cada negativa
/// vuelve con **su** código del catálogo y no con uno solo para todas
/// (ID-292). Aquí no se firma nada más que en el ciclo trifásico, pero se
/// llega a él por el token, por el filtro de la sede y por la política que
/// ella declaró, y esas tres son situaciones distintas que la sede sabe
/// contarle a la persona.
pub fn begin_for_the_site<E: FilterEngine>(
    site: &SiteSigning<'_, E>,
    order: &SigningOrder,
    stores: &[Store],
    listed: &ListedCertificates,
    opened: &OpenedDocuments,
    isolate: &Isolate,
    session: &SigningSession,
) -> Result<StoreSecret, SiteRefusal> {
    // El asa que se consintió tiene que seguir en el registro: si no está, no
    // hay documento que leer, y eso es lo que se le dice a la sede.
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
    // La sede ya no acepta el certificado que la ventana enseñó (ID-259): para
    // ella, ninguno que valga. Es el mismo código con el que
    // `errand::identity_handed_over` despacha esta misma situación.
    .map_err(|failure| SiteRefusal::new(SafCode::NoCertificatesInKeystore, failure))?;
    // Sin colocación nuestra esto no puede fallar hoy (ID-282), pero si algún
    // día fallara sería por el recuadro, que tiene código propio.
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

/// **Lo que la sede recibe cuando algo no sale**: el código que le toca a la
/// situación, y la situación entera para la ventana (ID-291).
///
/// Existe porque los dos destinos necesitan cosas distintas y ninguna sirve
/// para la otra. La ventana se arregla con [`Failure`], que lleva la situación
/// en texto; el cable necesita un código del catálogo publicado, y ese código
/// **lo manda la verdad de la situación, no el sitio donde se ha fallado**
/// (ID-292). Deducirlo del texto sería un `match` sobre cadenas con un
/// comodín, justo lo que la regla de [`frontier`] prohíbe: por eso el código
/// viaja decidido desde donde la situación todavía tenía tipo.
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

    /// El código que va al cable.
    pub fn code(&self) -> SafCode {
        self.code
    }

    /// La situación entera, para la ventana.
    pub fn failure(&self) -> &Failure {
        &self.failure
    }

    /// La situación entera, quedándosela.
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

/// Lo que una firma tiene de trámite de sede, y que en el recorrido local no
/// existe: el motor que hace cumplir el filtro y la política que ella declaró.
///
/// Van juntos porque llegan juntos —los dos salen de la misma operación— y
/// porque separarlos invita a pasar uno y olvidar el otro, que es firmar con la
/// política de la sede sin volver a comprobar su filtro, o al revés.
pub struct SiteSigning<'a, E: FilterEngine> {
    /// El motor de filtros, prestado del puente (ID-252).
    pub engine: &'a E,
    /// Lo que la sede pide del listado, que se comprueba otra vez (ID-259).
    pub filter: &'a SiteFilter,
    /// Los `extraParams` que declaró, ya expandidos (ID-266).
    pub from_the_site: &'a BTreeMap<String, String>,
}

/// La firma de un trámite de sede: lo que va al cable, y nada más.
pub struct SiteSignature {
    /// El PDF firmado, en bytes.
    pub signed: Vec<u8>,
    /// El DER del certificado firmante, que la sede recibe delante de la firma.
    pub signer_der: Vec<u8>,
}

/// **Caso de uso.** Postfirma de un trámite de sede: ensambla y devuelve, y
/// **no escribe nada** (ID-286, ID-264).
///
/// Tres cosas que la postfirma local hace y ésta **no**, y las tres son la
/// misma decisión leída de tres sitios:
///
/// - no deja caer el documento en la carpeta de destino: que una sede escriba
///   ficheros en el equipo está fuera del alcance por seguridad (ID-264), y el
///   documento que ella mandó no deja rastro (ID-286);
/// - no anota fila en la bandeja, ni «último documento»;
/// - no recuerda el certificado. El del trámite lo acotó el filtro de la sede,
///   y dejar que eso cambie el certificado por omisión de la persona sería
///   dejar que la sede elija por ella.
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
        // Llegar aquí sin ciclo abierto es que el trámite se ha descolocado:
        // no es ninguna situación que la sede sepa contar, es la firma que no
        // ha salido.
        .map_err(|failure| SiteRefusal::new(SafCode::SignatureFailed, failure))?;

    // Y aquí sí: el sello que no cuadra (ADR-0016) y la política que la sede
    // declaró y no se puede aplicar tienen código propio, y salen con él.
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

    /// **Grada A**: lo que se comprueba leyendo esta fuente son ausencias —lo
    /// que la postfirma de la sede **no** hace— y una ausencia no la vigila
    /// ninguna prueba de comportamiento.
    const SOURCE: &str = include_str!("site.rs");

    /// La mitad de producción, sin las pruebas: si no, esta comprobación se
    /// leería a sí misma y encontraría siempre sus propios literales.
    fn production_half() -> &'static str {
        SOURCE
            .split_once("\nmod tests {")
            .map(|(before, _)| before)
            .unwrap_or(SOURCE)
    }

    /// **ID-286 / ID-264**: la postfirma de un trámite de sede **no escribe
    /// nada**.
    ///
    /// Se lee la fuente y no el resultado por lo mismo que sus hermanas: el
    /// recorrido entero exige el puente, y lo que se vigila es una ausencia. Y
    /// una ausencia sólo se comprueba mirando: si mañana alguien añade ahí la
    /// entrega del documento «para que el usuario también tenga su copia»,
    /// ninguna prueba de comportamiento se pondría roja.
    #[test]
    fn the_postsign_of_a_site_errand_writes_nothing_anywhere() {
        // Es la última función del fichero: lo que sigue es el módulo de
        // pruebas, y la mitad de producción ya lo ha cortado.
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
                 deja rastro (ID-286) y rFirma no guarda ficheros por orden suya (ID-264)"
            );
        }
    }

    /// **ID-259**: la prefirma de un trámite de sede vuelve a pasar el filtro
    /// antes del PIN, y por eso no resuelve el certificado con
    /// `plan_signature`, que no sabe nada de la sede.
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

    /// **ID-282**: una firma de sede llega **sin colocación**, y de ahí no sale
    /// ninguna clave de geometría. Es así como los `signaturePositionOnPage*`
    /// que mandó la sede cruzan al puente crudos: no hay nada que los pise.
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

    /// Y la de la sede tampoco: el documento se pide igual, por su
    /// identificador, aunque quien lo abriera fuera el trámite (ID-62).
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
            "y la sede recibe el codigo de lo que ha pasado, no uno para todo (ID-292)"
        );
    }

    /// Un motor que nunca llega a que le pregunten: la prefirma de la sede se
    /// para antes, en el documento.
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
