package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;

import java.security.cert.X509Certificate;
import java.time.Instant;
import java.util.Properties;

import org.junit.jupiter.api.Tag;
import org.junit.jupiter.api.Test;

/** Los campos del certificado y de la firma que el puente devuelve de una firma CAdES y de una XAdES. */
@Tag("gradaC")
class PreviousSignatureCertificateFieldsTest {

    @Test
    void a_cades_signature_reports_the_validity_of_its_certificate_its_algorithm_and_profile()
            throws Exception {
        final byte[] signature = CadesCycle.sign(TestFixtures.challenge(), new Properties(), "sign");

        final PreviousSignaturesBridge.Signature read =
                PreviousSignaturesBridge.read(signature).signatures().get(0);

        assertCertificateFields(read);
        assertEquals("SHA256withRSA", read.signatureAlgorithm());
        assertNotNull(read.profile());
    }

    @Test
    void a_xades_signature_reports_the_validity_of_its_certificate_its_algorithm_and_profile()
            throws Exception {
        final byte[] signature = XadesCycle.sign(XadesCycle.referenceXml(), new Properties());

        final PreviousSignaturesBridge.Signature read =
                PreviousSignaturesBridge.read(signature).signatures().get(0);

        assertCertificateFields(read);
        assertEquals("SHA256withRSA", read.signatureAlgorithm());
        assertNotNull(read.profile());
    }

    private static void assertCertificateFields(final PreviousSignaturesBridge.Signature read)
            throws Exception {
        final X509Certificate certificate = TestFixtures.activeCertificate();
        assertEquals(certificate.getNotBefore().toInstant(), Instant.parse(read.validFrom()));
        assertEquals(certificate.getNotAfter().toInstant(), Instant.parse(read.validUntil()));
    }
}
