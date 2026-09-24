//! Pruebas de integración de grada C con la biblioteca nativa: apertura y filtrado (ADR-0014).

use std::path::{Path, PathBuf};

use rfirma_lib::signing::adapters::ffi::{locate, parse_presign, NativeBridge};
use rfirma_lib::signing::domain::bridge::{
    BridgeError, Format, PostSignRequest, PreSignRequest, SignatureOperation, LIBRARY_FILE,
};

/// Un PDF mínimo en Base64 no válido para firmar.
const NOT_A_PDF_B64: &str = "bm8gc295IHVuIFBERg==";

/// Un certificado sintético no válido.
const NOT_A_CERTIFICATE_B64: &str = "bm8gc295IHVuIGNlcnRpZmljYWRv";

/// Una prefirma sintética no válida, tal como la contaría el puente.
const NOT_A_PRESIGN_JSON: &str =
    r#"{"ok":true,"session":"<xml/>","pre":"MTIz","stamp":"bm8gc295IHVuIHNlbGxv"}"#;

fn library() -> PathBuf {
    let executable = std::env::current_exe().expect("debería haber ejecutable");
    let directory = executable.parent().unwrap_or(Path::new(".")).to_path_buf();
    locate(&|name| std::env::var_os(name), &directory).unwrap_or_else(|error| {
        panic!("{error}\n\nejecuta 'just test-native', que exporta RFIRMA_LIB_DIR")
    })
}

fn bridge() -> NativeBridge {
    NativeBridge::open_at(&library()).expect("la librería debería cargarse")
}

fn presign_of_something_invalid(bridge: &NativeBridge) -> Result<(), BridgeError> {
    presign_of_something_invalid_in(bridge, Format::Pades)
}

fn presign_of_something_invalid_in(
    bridge: &NativeBridge,
    format: Format,
) -> Result<(), BridgeError> {
    bridge
        .presign(PreSignRequest {
            format,
            operation: SignatureOperation::Sign,
            document_b64: NOT_A_PDF_B64,
            algorithm: "SHA256withRSA",
            certificate_chain_b64: NOT_A_CERTIFICATE_B64,
            extra_params: "signaturePage=1\n",
        })
        .map(|_| ())
}

fn postsign_of_something_invalid(bridge: &NativeBridge) -> Result<(), BridgeError> {
    let presigned = parse_presign(NOT_A_PRESIGN_JSON)?;
    let sealed = presigned
        .sealed_with(presigned.invented_signatures(), presigned.stamp())
        .expect("el sello es el mismo");
    bridge
        .postsign(PostSignRequest {
            format: Format::Pades,
            document_b64: NOT_A_PDF_B64,
            certificate_chain_b64: NOT_A_CERTIFICATE_B64,
            sealed: &sealed,
        })
        .map(|_| ())
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn the_library_loads_from_where_the_adr_says_and_creates_its_isolate() {
    let bridge = bridge();

    assert!(bridge.path().ends_with(LIBRARY_FILE));
    assert!(bridge.path().is_file());
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn a_bridge_failure_comes_back_as_json_and_not_as_a_crash() {
    let bridge = bridge();

    let error = presign_of_something_invalid(&bridge).expect_err("eso no es un PDF firmable");

    match error {
        BridgeError::Failed(detail) => assert!(
            detail.contains("Exception") || detail.contains("Error"),
            "el detalle crudo de Java tiene que llegar entero: {detail}"
        ),
        other => panic!("se esperaba un fallo del puente, no {other}"),
    }
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn the_postsign_crosses_the_border_and_comes_back_as_json_too() {
    let bridge = bridge();

    let error = postsign_of_something_invalid(&bridge).expect_err("eso no es una postfirma");

    match error {
        BridgeError::Failed(detail) => assert!(
            detail.contains("Exception") || detail.contains("Error"),
            "el detalle crudo de Java tiene que llegar entero: {detail}"
        ),
        other => panic!("se esperaba un fallo del puente, no {other}"),
    }
}

#[path = "native_cycle/support.rs"]
mod support;

/// Filtrado y expansión de políticas contra el token (ADR-0001, ADR-0014).
mod full_cycle {
    use rfirma_lib::signing::domain::bridge::{BridgeError, ExpandRequest, FilterRequest};
    use rfirma_lib::site::application::filtering;
    use rfirma_lib::site::domain::protocol::site_filter;

    use std::path::PathBuf;

    use rfirma_lib::identity::domain::certificate::{ListedCertificate, TokenCertificate};
    use rfirma_lib::identity::domain::error::TokenError;
    use rfirma_lib::site::ports::Certificates;

    use base64::Engine;

    use super::support::{bridge, signing_certificate, ACTIVE};

    struct NoDiscoveredModules;

    impl Certificates for NoDiscoveredModules {
        fn listed(&self) -> Result<Vec<TokenCertificate>, TokenError> {
            unreachable!("el filtrado recibe el listado hecho")
        }

        fn rows_of(&self, _found: Vec<TokenCertificate>) -> Vec<ListedCertificate> {
            unreachable!("el filtrado no pinta filas")
        }

        fn discovered_module(&self, _library: &str) -> Option<PathBuf> {
            None
        }

        fn usable<'a>(
            &self,
            _found: &'a [TokenCertificate],
            _handle: &str,
        ) -> Result<&'a TokenCertificate, TokenError> {
            unreachable!("el filtrado no elige certificado")
        }
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
    fn the_filter_engine_survives_inside_the_native_image() {
        let bridge = bridge();
        let certificate = signing_certificate();
        let der = base64::engine::general_purpose::STANDARD.encode(certificate.der());

        let inside = bridge
            .filter_certificates(FilterRequest {
                filter_properties: "filters=subject.contains:EIDAS CERTIFICADO PRUEBAS\n",
                certificates_b64: &der,
            })
            .expect("el motor tiene que contestar desde dentro de la imagen");
        assert_eq!(inside, vec![0]);

        let outside = bridge
            .filter_certificates(FilterRequest {
                filter_properties: "filters=subject.contains:NO ESTA EN EL SUBJECT\n",
                certificates_b64: &der,
            })
            .expect("el motor tiene que contestar desde dentro de la imagen");
        assert!(
            outside.is_empty(),
            "la sede lo excluye y el listado sale vacio, no completo"
        );
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
    fn the_policy_expander_survives_inside_the_native_image() {
        let bridge = bridge();

        let expanded = bridge
            .expand_extra_params(ExpandRequest {
                extra_params: "expPolicy=FirmaAGE\n",
                format: "PAdES",
                signed_data_length: 0,
            })
            .expect("el expansor tiene que contestar desde dentro de la imagen");

        assert!(
            !expanded.contains("expPolicy="),
            "la clave expandible se consume: {expanded}"
        );
        assert!(
            expanded.contains("policyIdentifier=urn:oid:"),
            "y el identificador sale del policy.properties de afirma-core: {expanded}"
        );
        assert!(
            expanded.contains("signatureSubFilter=ETSI.CAdES.detached"),
            "que es ademas el mismo subfiltro que rFirma envia siempre: {expanded}"
        );
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
    fn a_policy_that_does_not_fit_the_format_comes_back_named() {
        let bridge = bridge();

        let refused = bridge
            .expand_extra_params(ExpandRequest {
                extra_params: "expPolicy=PoliticaQueNoExiste\n",
                format: "PAdES",
                signed_data_length: 0,
            })
            .expect_err("esa politica no se puede aplicar");

        assert!(
            matches!(refused, BridgeError::IncompatiblePolicy(_)),
            "tenia que llegar con nombre propio: {refused:?}"
        );
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
    fn the_use_case_bounds_the_listing_through_the_real_bridge() {
        let bridge = bridge();
        let listing = vec![signing_certificate()];
        let filter = site_filter(&[(
            "filters".to_owned(),
            "subject.contains:EIDAS CERTIFICADO PRUEBAS".to_owned(),
        )]);

        let kept =
            filtering::keep_what_the_site_accepts(&bridge, &filter, listing, &NoDiscoveredModules)
                .expect("el motor contesta");

        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].reference().label(), ACTIVE);
    }
}
