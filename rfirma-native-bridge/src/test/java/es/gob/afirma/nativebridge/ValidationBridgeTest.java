package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.PrivateKey;
import java.security.Signature;
import java.security.cert.X509Certificate;
import java.util.Arrays;
import java.util.Base64;
import java.util.List;
import java.util.Properties;

import org.spongycastle.cms.CMSSignedData;
import org.spongycastle.cms.SignerInformation;
import org.junit.jupiter.api.Test;

import com.aowagie.text.pdf.AcroFields;
import com.aowagie.text.pdf.PdfReader;

import es.gob.afirma.signvalidation.SignValidity;
import es.gob.afirma.signvalidation.SignValidity.SIGN_DETAIL_TYPE;
import es.gob.afirma.signvalidation.SignValidity.VALIDITY_ERROR;

/** Las salidas del veredicto, sobre firmas hechas aqui mismo. */
class ValidationBridgeTest {

    private static final String ALGORITHM = "SHA256withRSA";

    @Test
    void a_document_without_signatures_is_unsigned() throws Exception {
        final ValidationBridge.Verdict verdict =
                ValidationBridge.validate(TestFixtures.samplePdf(), "PAdES", false);

        assertEquals(ValidationBridge.UNSIGNED, verdict.outcome());
    }

    @Test
    void a_freshly_signed_pdf_is_valid() throws Exception {
        final ValidationBridge.Verdict verdict =
                ValidationBridge.validate(signed(TestFixtures.samplePdf()), "PAdES", false);

        assertEquals(ValidationBridge.VALID, verdict.outcome(), "motivo: " + verdict.reason());
    }

    @Test
    void a_signature_that_no_longer_matches_the_document_is_invalid() throws Exception {
        final byte[] altered =
                TestFixtures.withOneByteChangedInsideTheSignedRange(signed(TestFixtures.samplePdf()));

        final ValidationBridge.Verdict verdict =
                ValidationBridge.validate(altered, "PAdES", false);

        assertEquals(ValidationBridge.INVALID, verdict.outcome());
        assertEquals("NO_MATCH_DATA", verdict.reason(),
                "una firma invalida cruza con el motivo del original");
    }

    @Test
    void a_document_modified_after_signing_asks_for_confirmation() throws Exception {
        final byte[] modified =
                TestFixtures.withThePageRepaintedAfterSigning(signed(TestFixtures.samplePdf()));

        final ValidationBridge.Verdict verdict =
                ValidationBridge.validate(modified, "PAdES", false);

        assertEquals(ValidationBridge.CONFIRMATION_NEEDED, verdict.outcome());
        assertEquals("allowShadowAttack", verdict.param(),
                "la clave de extraParams con la que se repite sin volver a preguntar");
        assertEquals("pdfShadowAttackSuspect", verdict.messageCode(),
                "y el codigo del mensaje con el que pregunta el original");
    }

    @Test
    void two_signatures_with_nothing_after_the_last_are_valid_with_the_oldest_listed_first()
            throws Exception {
        final byte[] pdf = TestFixtures.pdfWithVisibleCosignsApartFromTheFirstSignature(1);
        assertTheOldestSignatureIsListedFirst(pdf);

        final ValidationBridge.Verdict verdict = ValidationBridge.validate(pdf, "PAdES", false);

        assertEquals(ValidationBridge.VALID, verdict.outcome(),
                "motivo: " + verdict.reason() + ", pregunta: " + verdict.messageCode());
    }

    @Test
    void the_sample_signed_twice_by_the_original_lists_the_oldest_first_and_is_valid()
            throws Exception {
        final byte[] pdf = Files.readAllBytes(Path.of("..", "testdata", "previous-signatures",
                "pades-two-signatures-oldest-listed-first.pdf"));
        assertTheOldestSignatureIsListedFirst(pdf);

        final ValidationBridge.Verdict verdict = ValidationBridge.validate(pdf, "PAdES", false);

        assertEquals(ValidationBridge.VALID, verdict.outcome(),
                "motivo: " + verdict.reason() + ", pregunta: " + verdict.messageCode());
    }

    @Test
    void three_signatures_with_nothing_after_the_last_are_valid_whatever_order_they_are_listed_in()
            throws Exception {
        final byte[] pdf = TestFixtures.pdfWithVisibleCosignsApartFromTheFirstSignature(2);
        assertTheLatestSignatureIsNotListedFirst(pdf);

        final ValidationBridge.Verdict verdict = ValidationBridge.validate(pdf, "PAdES", false);

        assertEquals(ValidationBridge.VALID, verdict.outcome(),
                "motivo: " + verdict.reason() + ", pregunta: " + verdict.messageCode());
    }

    @Test
    void a_visible_cosign_over_the_first_signature_with_nothing_after_it_is_valid()
            throws Exception {
        final byte[] pdf = TestFixtures.pdfWithAVisibleCosignOverTheFirstSignature();
        assertTheOldestSignatureIsListedFirst(pdf);

        final ValidationBridge.Verdict verdict = ValidationBridge.validate(pdf, "PAdES", false);

        assertEquals(ValidationBridge.VALID, verdict.outcome(),
                "motivo: " + verdict.reason() + ", pregunta: " + verdict.messageCode());
    }

    @Test
    void a_page_repainted_after_the_last_of_several_signatures_asks_for_confirmation()
            throws Exception {
        final byte[] pdf = TestFixtures.withThePageRepaintedAfterSigning(
                TestFixtures.pdfWithVisibleCosignsApartFromTheFirstSignature(1));

        final ValidationBridge.Verdict verdict = ValidationBridge.validate(pdf, "PAdES", false);

        assertEquals(ValidationBridge.CONFIRMATION_NEEDED, verdict.outcome());
        assertEquals("allowShadowAttack", verdict.param());
        assertEquals("pdfShadowAttackSuspect", verdict.messageCode());
    }

    @Test
    void a_form_signed_twice_is_valid_while_nothing_is_filled_in_after_signing() throws Exception {
        final ValidationBridge.Verdict verdict = ValidationBridge.validate(
                TestFixtures.pdfWithATextFieldSignedTwice(), "PAdES", false);

        assertEquals(ValidationBridge.VALID, verdict.outcome(),
                "motivo: " + verdict.reason() + ", pregunta: " + verdict.messageCode());
    }

    @Test
    void a_form_filled_in_after_the_last_of_several_signatures_asks_for_confirmation()
            throws Exception {
        final byte[] pdf = TestFixtures.withTheTextFieldFilledInAfterSigning(
                TestFixtures.pdfWithATextFieldSignedTwice());

        final ValidationBridge.Verdict verdict = ValidationBridge.validate(pdf, "PAdES", false);

        assertEquals(ValidationBridge.CONFIRMATION_NEEDED, verdict.outcome());
        assertEquals("allowModifiedForm", verdict.param());
        assertEquals("signingModifiedPdfForm", verdict.messageCode());
    }

    @Test
    void a_freshly_signed_cades_is_valid() throws Exception {
        final Properties implicitMode = new Properties();
        implicitMode.setProperty("mode", "implicit");
        final byte[] signature =
                CadesCycle.sign(TestFixtures.challenge(), implicitMode, "sign");

        final ValidationBridge.Verdict verdict =
                ValidationBridge.validate(signature, "CAdES", false);

        assertEquals(ValidationBridge.VALID, verdict.outcome(), "motivo: " + verdict.reason());
    }

    @Test
    void a_freshly_signed_xades_is_valid() throws Exception {
        final byte[] signed = XadesCycle.sign(XadesCycle.referenceXml(), new Properties());

        final ValidationBridge.Verdict verdict =
                ValidationBridge.validate(signed, "XAdES Enveloping", false);

        assertEquals(ValidationBridge.VALID, verdict.outcome(), "motivo: " + verdict.reason());
    }

    /** El validador que no ha podido comprobar la firma no la da por buena. */
    @Test
    void a_signature_the_original_could_not_check_is_not_valid() throws Exception {
        final byte[] unreadable = ("<r><ds:Signature xmlns:ds=\"http://www.w3.org/2000/09/xmldsig#\""
                + "/></r>").getBytes(StandardCharsets.UTF_8);

        final ValidationBridge.Verdict verdict =
                ValidationBridge.validate(unreadable, "XAdES Enveloped", false);

        assertEquals(ValidationBridge.INVALID, verdict.outcome());
        assertEquals("UNKOWN_ERROR", verdict.reason());
    }

    @Test
    void an_explicit_cades_cannot_be_checked_without_its_content() throws Exception {
        final byte[] signature =
                CadesCycle.sign(TestFixtures.challenge(), new Properties(), "sign");

        final ValidationBridge.Verdict verdict =
                ValidationBridge.validate(signature, "CAdES", false);

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

    @Test
    void a_pdf_signed_with_an_expired_certificate_is_invalid_when_certificates_are_checked()
            throws Exception {
        final byte[] pdf = signedWithTheExpiredCertificate(TestFixtures.samplePdf());

        assertExpiredOnlyWhenChecked(pdf, "PAdES");
    }

    @Test
    void a_cades_signed_with_an_expired_certificate_is_invalid_when_certificates_are_checked()
            throws Exception {
        final Properties implicitMode = new Properties();
        implicitMode.setProperty("mode", "implicit");
        final byte[] signature = CadesCycle.signedBy(TestFixtures.challenge(), implicitMode,
                "sign", TestFixtures.expiredCertificateChain(), TestFixtures.expiredPrivateKey());

        assertExpiredOnlyWhenChecked(signature, "CAdES");
    }

    @Test
    void a_xades_signed_with_an_expired_certificate_is_invalid_when_certificates_are_checked()
            throws Exception {
        final byte[] signed = XadesCycle.signedBy(XadesCycle.referenceXml(), new Properties(),
                "sign", TestFixtures.expiredCertificateChain(), TestFixtures.expiredPrivateKey());

        assertExpiredOnlyWhenChecked(signed, "XAdES Enveloping");
    }

    @Test
    void a_cades_with_a_byte_of_its_content_changed_is_invalid() throws Exception {
        final byte[] altered = withOneByteChanged(implicitCades(), TestFixtures.challenge());

        final ValidationBridge.Verdict verdict = ValidationBridge.validate(altered, "CAdES", false);

        assertEquals(ValidationBridge.INVALID, verdict.outcome());
        assertEquals("NO_MATCH_DATA", verdict.reason());
    }

    @Test
    void a_cades_with_its_signature_value_altered_is_invalid() throws Exception {
        final byte[] signature = implicitCades();
        final byte[] value =
                ((SignerInformation) new CMSSignedData(signature).getSignerInfos().getSigners()
                        .iterator().next()).getSignature();

        final ValidationBridge.Verdict verdict =
                ValidationBridge.validate(withOneByteChanged(signature, value), "CAdES", false);

        assertEquals(ValidationBridge.INVALID, verdict.outcome());
    }

    @Test
    void a_cades_with_a_byte_of_its_content_changed_is_invalid_with_an_expired_certificate()
            throws Exception {
        final byte[] altered = withOneByteChanged(expiredImplicitCades(), TestFixtures.challenge());

        final ValidationBridge.Verdict verdict = ValidationBridge.validate(altered, "CAdES", false);

        assertEquals(ValidationBridge.INVALID, verdict.outcome());
        assertEquals("NO_MATCH_DATA", verdict.reason());
    }

    @Test
    void an_intact_cades_with_an_expired_certificate_is_valid_when_certificates_are_not_checked()
            throws Exception {
        final ValidationBridge.Verdict verdict =
                ValidationBridge.validate(expiredImplicitCades(), "CAdES", false);

        assertEquals(ValidationBridge.VALID, verdict.outcome(), "motivo: " + verdict.reason());
    }

    private static byte[] implicitCades() throws Exception {
        final Properties implicitMode = new Properties();
        implicitMode.setProperty("mode", "implicit");
        return CadesCycle.sign(TestFixtures.challenge(), implicitMode, "sign");
    }

    private static byte[] expiredImplicitCades() throws Exception {
        final Properties implicitMode = new Properties();
        implicitMode.setProperty("mode", "implicit");
        return CadesCycle.signedBy(TestFixtures.challenge(), implicitMode,
                "sign", TestFixtures.expiredCertificateChain(), TestFixtures.expiredPrivateKey());
    }

    private static byte[] withOneByteChanged(final byte[] document, final byte[] inside) {
        final byte[] altered = document.clone();
        for (int at = 0; at + inside.length <= altered.length; at++) {
            if (Arrays.equals(altered, at, at + inside.length, inside, 0, inside.length)) {
                altered[at + inside.length / 2] ^= 0x01;
                return altered;
            }
        }
        throw new IllegalStateException("el contenido no esta en la firma");
    }

    @Test
    void verify_says_of_a_freshly_signed_pdf_what_the_original_command_line_says() throws Exception {
        assertEquals(List.of("Firma valida"),
                ValidationBridge.results(signed(TestFixtures.samplePdf()), "PAdES"));
    }

    @Test
    void verify_says_of_a_freshly_signed_implicit_cades_that_it_is_valid() throws Exception {
        final Properties implicitMode = new Properties();
        implicitMode.setProperty("mode", "implicit");
        final byte[] signature = CadesCycle.sign(TestFixtures.challenge(), implicitMode, "sign");

        assertEquals(List.of("Firma valida"), ValidationBridge.results(signature, "CAdES"));
    }

    @Test
    void verify_checks_the_expiry_of_the_signing_certificate() throws Exception {
        final List<String> results =
                ValidationBridge.results(signedWithTheExpiredCertificate(TestFixtures.samplePdf()),
                        "PAdES");

        assertTrue(results.contains("Firma no valida: existe un certificado de firma caducado"),
                "resultados: " + results);
    }

    @Test
    void verify_says_a_document_without_signatures_has_none() throws Exception {
        assertEquals(List.of("Firma no valida: no se encuentra la firma dentro del documento"),
                ValidationBridge.results(TestFixtures.samplePdf(), "PAdES"));
    }

    @Test
    void verify_reports_a_pdf_repainted_after_signing_instead_of_asking_for_confirmation()
            throws Exception {
        final byte[] modified =
                TestFixtures.withThePageRepaintedAfterSigning(signed(TestFixtures.samplePdf()));

        final List<String> results = ValidationBridge.results(modified, "PAdES");

        assertTrue(results.stream().anyMatch(result -> result.startsWith("Firma no valida")),
                "resultados: " + results);
    }

    @Test
    void a_pdf_whose_byte_range_does_not_start_at_zero_is_invalid() throws Exception {
        final byte[] pdf = ByteRangeSamples.resignedOver(signed(TestFixtures.samplePdf()),
                range -> new long[] {1, range[1] - 1, range[2], range[3]});

        final ValidationBridge.Verdict verdict = ValidationBridge.validate(pdf, "PAdES", false);

        assertEquals(ValidationBridge.INVALID, verdict.outcome());
        assertEquals("CORRUPTED_SIGN", verdict.reason());
    }

    @Test
    void a_pdf_whose_byte_range_gap_holds_unsigned_bytes_besides_the_contents_is_invalid()
            throws Exception {
        final byte[] pdf = ByteRangeSamples.withUnsignedBytesInsideTheContentsGap(
                signed(TestFixtures.samplePdf()));

        final ValidationBridge.Verdict verdict = ValidationBridge.validate(pdf, "PAdES", false);

        assertEquals(ValidationBridge.INVALID, verdict.outcome());
        assertEquals("CORRUPTED_SIGN", verdict.reason());
    }

    @Test
    void a_pdf_whose_byte_range_has_more_than_four_numbers_is_invalid() throws Exception {
        final byte[] signed = signed(TestFixtures.samplePdf());
        final long[] range = ByteRangeSamples.lastRange(signed);
        final byte[] pdf = ByteRangeSamples.withTheByteRangeWritten(signed,
                "[0 " + range[1] + " " + range[2] + " " + range[3] + " 0]");

        final ValidationBridge.Verdict verdict = ValidationBridge.validate(pdf, "PAdES", false);

        assertEquals(ValidationBridge.INVALID, verdict.outcome());
        assertEquals("CORRUPTED_SIGN", verdict.reason());
    }

    @Test
    void a_pdf_whose_byte_range_reaches_beyond_every_offset_is_invalid() throws Exception {
        final byte[] pdf =
                ByteRangeSamples.withAByteRangeBeyondEveryOffset(signed(TestFixtures.samplePdf()));

        final ValidationBridge.Verdict verdict = ValidationBridge.validate(pdf, "PAdES", false);

        assertEquals(ValidationBridge.INVALID, verdict.outcome());
        assertEquals("CORRUPTED_SIGN", verdict.reason());
    }

    @Test
    void the_site_accepts_a_pdf_whose_byte_range_ends_inside_its_revision() throws Exception {
        final byte[] pdf = ByteRangeSamples.resignedOver(signed(TestFixtures.samplePdf()),
                range -> new long[] {0, range[1], range[2], range[3] - "%%EOF\n".length()});

        final ValidationBridge.Verdict verdict = ValidationBridge.validate(pdf, "PAdES", false);

        assertEquals(ValidationBridge.VALID, verdict.outcome(), "motivo: " + verdict.reason());
    }

    @Test
    void the_site_accepts_a_pdf_with_spare_bytes_after_its_last_eof() throws Exception {
        final byte[] pdf =
                ByteRangeSamples.withSpareBytesAfterTheLastEof(signed(TestFixtures.samplePdf()));

        final ValidationBridge.Verdict verdict = ValidationBridge.validate(pdf, "PAdES", false);

        assertEquals(ValidationBridge.VALID, verdict.outcome(), "motivo: " + verdict.reason());
    }

    @Test
    void the_site_accepts_a_pdf_with_two_signatures_in_incremental_revisions() throws Exception {
        final ValidationBridge.Verdict verdict =
                ValidationBridge.validate(TestFixtures.pdfWithTwoOrdinarySignatures(), "PAdES", false);

        assertEquals(ValidationBridge.VALID, verdict.outcome(), "motivo: " + verdict.reason());
    }

    @Test
    void the_site_accepts_a_pdf_with_a_document_timestamp_after_its_signature() throws Exception {
        final byte[] pdf = ByteRangeSamples.withADocTimeStampAfter(signed(TestFixtures.samplePdf()));

        final ValidationBridge.Verdict verdict = ValidationBridge.validate(pdf, "PAdES", false);

        assertEquals(ValidationBridge.VALID, verdict.outcome(), "motivo: " + verdict.reason());
    }

    @Test
    void a_pdf_whose_document_timestamp_gap_holds_unsigned_bytes_is_invalid() throws Exception {
        final byte[] pdf = ByteRangeSamples.withUnsignedBytesInsideTheContentsGap(
                ByteRangeSamples.withADocTimeStampAfter(signed(TestFixtures.samplePdf())));

        final ValidationBridge.Verdict verdict = ValidationBridge.validate(pdf, "PAdES", false);

        assertEquals(ValidationBridge.INVALID, verdict.outcome());
        assertEquals("CORRUPTED_SIGN", verdict.reason());
    }

    @Test
    void verify_reports_a_signature_whose_byte_range_ends_inside_its_revision() throws Exception {
        final byte[] pdf = ByteRangeSamples.resignedOver(signed(TestFixtures.samplePdf()),
                range -> new long[] {0, range[1], range[2], range[3] - "%%EOF\n".length()});

        final List<String> results = ValidationBridge.results(pdf, "PAdES");

        assertTrue(results.contains(ValidationBridge.plainText(new SignValidity(
                SIGN_DETAIL_TYPE.KO, VALIDITY_ERROR.CORRUPTED_SIGN).toString())),
                "resultados: " + results);
    }

    @Test
    void verify_says_of_a_pdf_with_spare_bytes_after_its_last_eof_that_it_is_valid()
            throws Exception {
        final byte[] pdf =
                ByteRangeSamples.withSpareBytesAfterTheLastEof(signed(TestFixtures.samplePdf()));

        assertEquals(List.of("Firma valida"), ValidationBridge.results(pdf, "PAdES"));
    }

    @Test
    void the_html_entities_of_the_original_messages_become_plain_letters() {
        assertEquals("la información no es válida",
                ValidationBridge.plainText("la informaci&oacute;n no es v&aacute;lida"));
    }

    private static void assertExpiredOnlyWhenChecked(final byte[] document, final String format)
            throws Exception {
        final ValidationBridge.Verdict checked = ValidationBridge.validate(document, format, true);
        final ValidationBridge.Verdict unchecked =
                ValidationBridge.validate(document, format, false);

        assertEquals(ValidationBridge.INVALID, checked.outcome());
        assertEquals("CERTIFICATE_EXPIRED", checked.reason(),
                "el motivo con el que el original da por caducado el certificado");
        assertEquals(ValidationBridge.VALID, unchecked.outcome(),
                "sin comprobar certificados la caducidad no cuenta: motivo " + unchecked.reason());
    }

    private static void assertTheOldestSignatureIsListedFirst(final byte[] pdf) throws Exception {
        assertEquals(TestFixtures.FIRST_SIGNATURE_FIELD,
                new PdfReader(pdf).getAcroFields().getSignatureNames().get(0),
                "iText lista primero la firma antigua");
    }

    private static void assertTheLatestSignatureIsNotListedFirst(final byte[] pdf)
            throws Exception {
        final AcroFields fields = new PdfReader(pdf).getAcroFields();
        assertNotEquals(fields.getTotalRevisions(),
                fields.getRevision(fields.getSignatureNames().get(0)),
                "iText no lista primero la firma de la ultima revision: "
                        + fields.getSignatureNames());
    }

    private static byte[] signed(final byte[] pdf) throws Exception {
        return signedBy(pdf, TestFixtures.certificateChain(), TestFixtures.privateKey());
    }

    private static byte[] signedWithTheExpiredCertificate(final byte[] pdf) throws Exception {
        return signedBy(pdf, TestFixtures.expiredCertificateChain(),
                TestFixtures.expiredPrivateKey());
    }

    private static byte[] signedBy(final byte[] pdf, final X509Certificate[] chain,
            final PrivateKey key) throws Exception {
        final PadesBridge.PreSignResult pre =
                PadesBridge.preSign(pdf, ALGORITHM, chain, new Properties());

        final Signature signature = Signature.getInstance(ALGORITHM);
        signature.initSign(key);
        signature.update(Base64.getDecoder().decode(pre.preSignB64()));

        return PadesBridge.postSign(pdf, chain, pre.stamp(), pre.session(),
                Base64.getEncoder().encodeToString(signature.sign()));
    }

}
