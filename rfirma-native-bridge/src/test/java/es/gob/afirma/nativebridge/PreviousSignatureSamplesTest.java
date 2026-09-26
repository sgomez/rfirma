package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.file.Files;
import java.nio.file.Path;
import java.security.cert.CertificateExpiredException;

import org.junit.jupiter.api.Test;

import com.aowagie.text.pdf.AcroFields;
import com.aowagie.text.pdf.PdfPKCS7;
import com.aowagie.text.pdf.PdfReader;

import es.gob.afirma.signvalidation.ISignatureFormatDetector;
import es.gob.afirma.signvalidation.SignatureFormatDetectorPadesCades;

/** Las muestras versionadas de {@code testdata/previous-signatures/} son lo que dice su nombre. */
class PreviousSignatureSamplesTest {

    private static final Path LONG_TERM_EXPIRED =
            Path.of("..", "testdata", "previous-signatures", "pades-long-term-expired.pdf");

    @Test
    void the_long_term_expired_sample_carries_a_signature_timestamp() throws Exception {
        assertNotNull(onlySignature(LONG_TERM_EXPIRED).getTimeStampDate());
    }

    @Test
    void the_long_term_expired_sample_has_a_long_term_profile() throws Exception {
        assertEquals(ISignatureFormatDetector.FORMAT_PADES_T_LEVEL,
                SignatureFormatDetectorPadesCades.resolvePDFFormat(Files.readAllBytes(LONG_TERM_EXPIRED)));
    }

    @Test
    void the_long_term_expired_sample_still_matches_its_signed_data() throws Exception {
        assertTrue(onlySignature(LONG_TERM_EXPIRED).verify());
    }

    @Test
    void the_long_term_expired_sample_is_signed_with_the_expired_certificate_of_the_kit()
            throws Exception {
        final PdfPKCS7 signature = onlySignature(LONG_TERM_EXPIRED);

        assertEquals(TestFixtures.expiredCertificate(), signature.getSigningCertificate());
        assertThrows(CertificateExpiredException.class,
                () -> signature.getSigningCertificate().checkValidity());
    }

    private static PdfPKCS7 onlySignature(final Path sample) throws Exception {
        final AcroFields fields = new PdfReader(Files.readAllBytes(sample)).getAcroFields();
        assertEquals(1, fields.getSignatureNames().size());
        return fields.verifySignature((String) fields.getSignatureNames().get(0));
    }
}
