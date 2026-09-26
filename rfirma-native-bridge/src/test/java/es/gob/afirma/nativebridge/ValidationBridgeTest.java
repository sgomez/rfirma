package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.charset.StandardCharsets;
import java.security.Signature;
import java.security.cert.X509Certificate;
import java.util.Base64;
import java.util.Properties;

import org.junit.jupiter.api.Test;

/** Las salidas del veredicto, sobre firmas hechas aqui mismo. */
class ValidationBridgeTest {

    private static final String ALGORITHM = "SHA256withRSA";

    @Test
    void a_document_without_signatures_is_unsigned() throws Exception {
        final ValidationBridge.Verdict verdict =
                ValidationBridge.validate(TestFixtures.samplePdf(), "PAdES");

        assertEquals(ValidationBridge.UNSIGNED, verdict.outcome());
    }

    @Test
    void a_freshly_signed_pdf_is_valid() throws Exception {
        final ValidationBridge.Verdict verdict =
                ValidationBridge.validate(signed(TestFixtures.samplePdf()), "PAdES");

        assertEquals(ValidationBridge.VALID, verdict.outcome(), "motivo: " + verdict.reason());
    }

    @Test
    void a_signature_that_no_longer_matches_the_document_is_invalid() throws Exception {
        final byte[] altered =
                TestFixtures.withOneByteChangedInsideTheSignedRange(signed(TestFixtures.samplePdf()));

        final ValidationBridge.Verdict verdict = ValidationBridge.validate(altered, "PAdES");

        assertEquals(ValidationBridge.INVALID, verdict.outcome());
        assertEquals("NO_MATCH_DATA", verdict.reason(),
                "una firma invalida cruza con el motivo del original");
    }

    @Test
    void a_document_modified_after_signing_asks_for_confirmation() throws Exception {
        final byte[] modified =
                TestFixtures.withThePageRepaintedAfterSigning(signed(TestFixtures.samplePdf()));

        final ValidationBridge.Verdict verdict = ValidationBridge.validate(modified, "PAdES");

        assertEquals(ValidationBridge.CONFIRMATION_NEEDED, verdict.outcome());
        assertEquals("allowShadowAttack", verdict.param(),
                "la clave de extraParams con la que se repite sin volver a preguntar");
        assertEquals("pdfShadowAttackSuspect", verdict.messageCode(),
                "y el codigo del mensaje con el que pregunta el original");
    }

    @Test
    void a_freshly_signed_cades_is_valid() throws Exception {
        final Properties implicitMode = new Properties();
        implicitMode.setProperty("mode", "implicit");
        final byte[] signature =
                CadesCycle.sign(TestFixtures.challenge(), implicitMode, "sign");

        final ValidationBridge.Verdict verdict = ValidationBridge.validate(signature, "CAdES");

        assertEquals(ValidationBridge.VALID, verdict.outcome(), "motivo: " + verdict.reason());
    }

    @Test
    void a_freshly_signed_xades_is_valid() throws Exception {
        final byte[] signed = XadesCycle.sign(XadesCycle.referenceXml(), new Properties());

        final ValidationBridge.Verdict verdict =
                ValidationBridge.validate(signed, "XAdES Enveloping");

        assertEquals(ValidationBridge.VALID, verdict.outcome(), "motivo: " + verdict.reason());
    }

    /** El validador que no ha podido comprobar la firma no la da por buena. */
    @Test
    void a_signature_the_original_could_not_check_is_not_valid() throws Exception {
        final byte[] unreadable = ("<r><ds:Signature xmlns:ds=\"http://www.w3.org/2000/09/xmldsig#\""
                + "/></r>").getBytes(StandardCharsets.UTF_8);

        final ValidationBridge.Verdict verdict =
                ValidationBridge.validate(unreadable, "XAdES Enveloped");

        assertEquals(ValidationBridge.INVALID, verdict.outcome());
        assertEquals("UNKOWN_ERROR", verdict.reason());
    }

    @Test
    void an_explicit_cades_cannot_be_checked_without_its_content() throws Exception {
        final byte[] signature =
                CadesCycle.sign(TestFixtures.challenge(), new Properties(), "sign");

        final ValidationBridge.Verdict verdict = ValidationBridge.validate(signature, "CAdES");

        assertEquals(ValidationBridge.INVALID, verdict.outcome(),
                "sin los datos firmados el original no puede comprobar nada");
        assertEquals("NO_DATA", verdict.reason());
    }

    @Test
    void a_format_without_a_validator_of_its_own_is_refused() {
        assertThrows(IllegalArgumentException.class,
                () -> ValidationBridge.validerFor("CAdES-ASiC-S"));
    }

    @Test
    void the_binary_and_the_xml_validators_answer_for_their_formats() {
        assertTrue(ValidationBridge.validerFor("CAdES").getClass().getName()
                .endsWith("ValidateBinarySignature"));
        assertTrue(ValidationBridge.validerFor("FacturaE").getClass().getName()
                .endsWith("ValidateXMLSignature"));
    }

    private static byte[] signed(final byte[] pdf) throws Exception {
        final X509Certificate[] chain = TestFixtures.certificateChain();
        final PadesBridge.PreSignResult pre =
                PadesBridge.preSign(pdf, ALGORITHM, chain, new Properties());

        final Signature signature = Signature.getInstance(ALGORITHM);
        signature.initSign(TestFixtures.privateKey());
        signature.update(Base64.getDecoder().decode(pre.preSignB64()));

        return PadesBridge.postSign(pdf, chain, pre.stamp(), pre.session(),
                Base64.getEncoder().encodeToString(signature.sign()));
    }

}
