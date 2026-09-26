package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.security.PrivateKey;
import java.security.Signature;
import java.security.cert.X509Certificate;
import java.time.Instant;
import java.util.Base64;
import java.util.List;
import java.util.Properties;

import org.junit.jupiter.api.Test;

/** Quien firmo y cuando, sobre firmas y cofirmas hechas aqui mismo. */
class PreviousSignaturesBridgeTest {

    private static final String ALGORITHM = "SHA256withRSA";

    @Test
    void an_unsigned_pdf_has_no_previous_signatures() throws Exception {
        assertTrue(PreviousSignaturesBridge.read(TestFixtures.samplePdf()).isEmpty());
    }

    @Test
    void a_signed_pdf_reports_the_signer_and_the_signing_time() throws Exception {
        final X509Certificate signer = TestFixtures.activeCertificate();
        final byte[] pdf = signed(TestFixtures.samplePdf(),
                TestFixtures.certificateChain(), TestFixtures.privateKey());

        final List<PreviousSignaturesBridge.Signature> signatures =
                PreviousSignaturesBridge.read(pdf);

        assertEquals(1, signatures.size());
        final PreviousSignaturesBridge.Signature signature = signatures.get(0);
        assertEquals(PreviousSignaturesBridge.readable(signer.getSubjectX500Principal()),
                signature.subject());
        assertEquals(PreviousSignaturesBridge.readable(signer.getIssuerX500Principal()),
                signature.issuer());
        assertEquals(signer.getSerialNumber().toString(), signature.serialNumber());
        assertTrue(Instant.parse(signature.signingTime()).isBefore(Instant.now().plusSeconds(1)),
                "la fecha de firma cruza en ISO-8601");
    }

    @Test
    void a_cosigned_pdf_reports_both_signers_in_chronological_order() throws Exception {
        final byte[] once = signed(TestFixtures.samplePdf(),
                TestFixtures.certificateChain(), TestFixtures.privateKey());
        // El /M de una firma PDF solo tiene resolucion de segundo: sin esta
        // espera ambas firmas podrian caer en el mismo segundo.
        Thread.sleep(1_100);
        final byte[] twice = signed(once,
                TestFixtures.otherCertificateChain(), TestFixtures.otherPrivateKey());

        final List<PreviousSignaturesBridge.Signature> signatures =
                PreviousSignaturesBridge.read(twice);

        assertEquals(2, signatures.size());
        assertEquals(TestFixtures.activeCertificate().getSerialNumber().toString(),
                signatures.get(0).serialNumber(), "la primera firma es la mas antigua");
        assertEquals(TestFixtures.otherCertificateChain()[0].getSerialNumber().toString(),
                signatures.get(1).serialNumber());
        assertTrue(Instant.parse(signatures.get(0).signingTime())
                .isBefore(Instant.parse(signatures.get(1).signingTime())));
    }

    @Test
    void the_subject_names_the_id_number_by_keyword_and_not_as_hex() throws Exception {
        final String subject = PreviousSignaturesBridge.readable(
                TestFixtures.activeCertificate().getSubjectX500Principal());

        assertTrue(subject.contains("SERIALNUMBER=IDCES-99999999R"), subject);
    }

    private static byte[] signed(final byte[] pdf, final X509Certificate[] chain,
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
