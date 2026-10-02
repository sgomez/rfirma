//! Las pruebas de grada C de `sign -config` con el puente real.

use super::*;

const A_POLICY_AND_A_BOX: &str = "# la política de la AGE y un recuadro en la primera página\\n\
     expPolicy=FirmaAGE\\n\
     signaturePositionOnPageLowerLeftX=100\\n\
     signaturePositionOnPageLowerLeftY=100\\n\
     signaturePositionOnPageUpperRightX=300\\n\
     signaturePositionOnPageUpperRightY=200\\n\
     signaturePage=1";

/// El OID 2.16.724.1.3.1.1.2.1.9 de la política de la AGE, en DER y en el hexadecimal de `/Contents`.
const AGE_POLICY_IN_THE_CONTENTS: &str = "060a60855401030101020109";

fn signed_with_config(config: &str) -> (tempfile::TempDir, Roots, Outcome, PathBuf) {
    let (home, installed_alias) = a_home_with_an_installed_certificate();
    let roots = the_roots_under(home.path());
    let input = home.path().join("documento.pdf");
    std::fs::write(&input, a_one_page_pdf()).expect("el PDF deberia escribirse");
    let output = home.path().join("firmado.pdf");
    let outcome = attended_with_the_roots(
        &[
            "sign",
            "-i",
            input.to_str().expect("ruta UTF-8"),
            "-o",
            output.to_str().expect("ruta UTF-8"),
            "-alias",
            &installed_alias,
            "-config",
            config,
        ],
        &roots,
        &ScriptedTerminal::without_a_tty(),
    );
    (home, roots, outcome, output)
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn sign_with_config_puts_its_policy_and_its_visible_box_in_the_pades() {
    let (_home, roots, outcome, output) = signed_with_config(A_POLICY_AND_A_BOX);

    assert_eq!(outcome.exit_code, SUCCEEDED, "{:?}", outcome.stderr);
    let signed = std::fs::read(&output).expect("la firma deberia estar en -o");
    assert_eq!(verdict_of(&roots, &signed), SignatureVerdict::Valid);
    let text = String::from_utf8_lossy(&signed).to_lowercase();
    assert!(
        text.contains(AGE_POLICY_IN_THE_CONTENTS),
        "la firma deberia llevar la política de la AGE"
    );
    assert!(
        text.contains("/bbox[0 0 200 100]"),
        "la apariencia de la firma deberia medir el recuadro de -config"
    );
}

#[test]
fn sign_with_a_config_a_site_could_not_declare_fails_with_its_refusal_and_writes_nothing() {
    let (_home, roots, outcome, output) = signed_with_config(
        "signaturePositionOnPageLowerLeftX=100\\n\
         signaturePositionOnPageLowerLeftY=100\\n\
         signaturePositionOnPageUpperRightX=300\\n\
         signaturePositionOnPageUpperRightY=200\\n\
         signaturePage=append",
    );

    assert_eq!(outcome.exit_code, FAILED, "{:?}", outcome.stderr);
    assert!(
        outcome.stderr.concat().contains("signaturePage=append"),
        "{:?}",
        outcome.stderr
    );
    assert!(!output.exists());
    assert_eq!(roots.identity.remembered_certificate(), None);
}
