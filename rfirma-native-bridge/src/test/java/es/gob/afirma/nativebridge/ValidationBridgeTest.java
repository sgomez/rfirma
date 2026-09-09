package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;
import java.security.Signature;
import java.security.cert.X509Certificate;
import java.util.Base64;
import java.util.Properties;

import com.aowagie.text.pdf.PRIndirectReference;
import com.aowagie.text.pdf.PdfName;
import com.aowagie.text.pdf.PdfReader;

import org.junit.jupiter.api.Test;

/** Las tres salidas del veredicto, sobre firmas hechas aqui mismo. */
class ValidationBridgeTest {

    private static final String ALGORITHM = "SHA256withRSA";

    @Test
    void a_document_without_signatures_is_valid() throws Exception {
        final ValidationBridge.Verdict verdict =
                ValidationBridge.validate(TestFixtures.samplePdf(), "PAdES");

        assertEquals(ValidationBridge.VALID, verdict.outcome());
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
                withOneByteChangedInsideTheSignedRange(signed(TestFixtures.samplePdf()));

        final ValidationBridge.Verdict verdict = ValidationBridge.validate(altered, "PAdES");

        assertEquals(ValidationBridge.INVALID, verdict.outcome());
        assertEquals("NO_MATCH_DATA", verdict.reason(),
                "una firma invalida cruza con el motivo del original");
    }

    @Test
    void a_document_modified_after_signing_asks_for_confirmation() throws Exception {
        final byte[] modified = withThePageRepaintedAfterSigning(signed(TestFixtures.samplePdf()));

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

    /** La version del encabezado entra en el {@code /ByteRange}: el resumen deja de cuadrar. */
    private static byte[] withOneByteChangedInsideTheSignedRange(final byte[] pdf) {
        final int header = new String(pdf, StandardCharsets.ISO_8859_1).indexOf("%PDF-1.");
        assertTrue(header >= 0, "el encabezado tiene que estar");
        final byte[] altered = pdf.clone();
        final int version = header + "%PDF-1.".length();
        altered[version] = (byte) (altered[version] == '7' ? '4' : '7');
        return altered;
    }

    /**
     * Repinta la pagina en una revision incremental posterior a la firma, que es
     * el ataque que el original llama PDF Shadow Attack. La revision se escribe a
     * mano porque el PDF firmado cierra con un flujo de referencias cruzadas y una
     * tabla clasica encadenada a el no la lee ni iText.
     */
    private static byte[] withThePageRepaintedAfterSigning(final byte[] pdf) throws Exception {
        final PdfReader reader = new PdfReader(pdf);
        final int page =
                ((PRIndirectReference) reader.getPageN(1).get(PdfName.CONTENTS)).getNumber();
        final int root = ((PRIndirectReference) reader.getTrailer().get(PdfName.ROOT)).getNumber();
        final int table = reader.getXrefSize();

        final String painting = "1 0 0 RG 1 0 0 rg 50 50 400 300 re f\n";
        final String repainted = "\n" + page + " 0 obj\n<< /Length " + painting.length()
                + " >>\nstream\n" + painting + "endstream\nendobj\n";
        final int pageOffset = pdf.length + 1;
        final int tableOffset = pdf.length + repainted.length();

        final ByteArrayOutputStream rows = new ByteArrayOutputStream();
        rows.write(new byte[] {0, 0, 0, 0, 0, (byte) 0xff, (byte) 0xff});
        rows.write(inUse(pageOffset));
        rows.write(inUse(tableOffset));

        final String opening = table + " 0 obj\n<< /Type /XRef /Size " + (table + 1)
                + " /Index [0 1 " + page + " 1 " + table + " 1] /W [1 4 2] /Root " + root
                + " 0 R /Prev " + reader.getLastXref() + " /Length " + rows.size()
                + " >>\nstream\n";
        final String closing = "\nendstream\nendobj\nstartxref\n" + tableOffset + "\n%%EOF\n";

        final ByteArrayOutputStream out = new ByteArrayOutputStream();
        out.write(pdf);
        out.write(repainted.getBytes(StandardCharsets.ISO_8859_1));
        out.write(opening.getBytes(StandardCharsets.ISO_8859_1));
        out.write(rows.toByteArray());
        out.write(closing.getBytes(StandardCharsets.ISO_8859_1));
        return out.toByteArray();
    }

    private static byte[] inUse(final int offset) {
        return new byte[] {1, (byte) (offset >>> 24), (byte) (offset >>> 16),
                (byte) (offset >>> 8), (byte) offset, 0, 0};
    }
}
