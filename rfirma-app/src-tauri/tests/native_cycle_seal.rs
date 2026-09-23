//! Pruebas de integración del sello del ciclo trifásico contra el token: rechazo de un campo alterado (ADR-0001, ADR-0014, ADR-0016).

#[path = "native_cycle/support.rs"]
mod support;

mod full_cycle {
    use base64::Engine;
    use rfirma_lib::identity::adapters::pkcs11;
    use rfirma_lib::signing::adapters::ffi::NativeBridge;
    use rfirma_lib::signing::application::cycle::{self, SigningRequest};
    use rfirma_lib::signing::domain::bridge::{Format, SignatureOperation};
    use rfirma_lib::signing::domain::{AdmissibleDocument, SessionSeal, TokenSignatures};

    use super::support::{
        a_config_of, a_one_page_pdf, bridge, reference, signing_certificate, PIN,
    };

    /// Decodifica el bloque de texto del sello (ADR-0016).
    fn inside_the_seal(seal: &SessionSeal) -> String {
        let raw = base64::engine::general_purpose::STANDARD
            .decode(seal.as_bridge_payload())
            .expect("el sello del puente viene en Base64");
        String::from_utf8(raw).expect("el bloque del sello es UTF-8")
    }

    /// Re-codifica las líneas del sello preservando el salto final.
    fn re_encoded(lines: &[String]) -> String {
        base64::engine::general_purpose::STANDARD.encode(format!("{}\n", lines.join("\n")))
    }

    /// Altera el primer campo del sello que empiece por prefijo.
    fn seal_with_field_altered(seal: &SessionSeal, prefix: &str) -> SessionSeal {
        let block = inside_the_seal(seal);

        let untouched: Vec<String> = block.lines().map(str::to_owned).collect();
        assert_eq!(
            re_encoded(&untouched),
            seal.as_bridge_payload(),
            "reconstruir el sello sin mutar nada tiene que dar el mismo payload byte a \
             byte, o esta prueba pasaria por la reconstruccion y no por el campo \
             '{prefix}':\n{block}"
        );

        let mut altered = false;
        let lines: Vec<String> = block
            .lines()
            .map(|line| {
                if !altered && line.starts_with(prefix) {
                    altered = true;
                    format!("{line}-alterado")
                } else {
                    line.to_owned()
                }
            })
            .collect();
        assert!(
            altered,
            "el sello del puente ya no lleva ningun campo '{prefix}'. \
             Sin ese campo la invariante del ADR-0016 no esta sellada, \
             y esta prueba habria pasado sin comprobar nada:\n{block}"
        );
        SessionSeal::from_bridge(re_encoded(&lines))
    }

    /// Prepara un ciclo firmado en token listo para postfirma.
    fn a_cycle_ready_to_postsign() -> (NativeBridge, cycle::OpenCycle, TokenSignatures) {
        let bridge = bridge();
        let pdf = a_one_page_pdf();
        let certificate = signing_certificate();
        let chain = vec![certificate.der().to_vec()];
        let reference = reference();
        let config = a_config_of("Firmado por: PRUEBAS FNMT", None);

        let cycle = cycle::presign(
            &bridge,
            SigningRequest {
                format: Format::Pades,
                algorithm: cycle::ALGORITHM,
                operation: SignatureOperation::Sign,
                document: AdmissibleDocument::check(&pdf).expect("es admisible"),
                chain: &chain,
                config: &config,
                from_the_site: &cycle::NOTHING_FROM_A_SITE,
                certificate: &reference,
            },
        )
        .expect("la prefirma deberia salir");
        let signature = cycle
            .sign_on_token(&pkcs11::RealToken, PIN)
            .expect("el token deberia firmar");

        (bridge, cycle, signature)
    }

    /// Comprueba que postsign rechaza un sello con un campo alterado (ADR-0016).
    fn postsign_refuses_a_seal_altered_in(prefix: &str) {
        let (bridge, cycle, signature) = a_cycle_ready_to_postsign();

        let tampered = seal_with_field_altered(&cycle.seal_in_transit(), prefix);

        let outcome = cycle
            .postsign(&bridge, signature, &tampered)
            .map(|completed| format!("un PDF de {} bytes", completed.signed_document().len()));

        assert!(
            matches!(outcome, Err(cycle::CycleError::Seal(_))),
            "con el campo '{prefix}' alterado la postfirma tenia que abortar por el \
             sello, y ha contestado {outcome:?}. Un Ok aqui es un PDF con Digest \
             Mismatch que nadie sabe por que no vale"
        );
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so y el token (just test-native)"]
    fn an_altered_signing_instant_makes_the_postsign_fail() {
        postsign_refuses_a_seal_altered_in("TIME=");
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so y el token (just test-native)"]
    fn an_altered_time_zone_makes_the_postsign_fail() {
        postsign_refuses_a_seal_altered_in("TZ=");
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so y el token (just test-native)"]
    fn altered_effective_extra_params_make_the_postsign_fail() {
        postsign_refuses_a_seal_altered_in("P.");
    }
}
