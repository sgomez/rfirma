package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.net.Proxy;
import java.net.ProxySelector;
import java.net.SocketAddress;
import java.net.URI;
import java.security.PrivateKey;
import java.security.Signature;
import java.security.cert.X509Certificate;
import java.time.Instant;
import java.util.Base64;
import java.util.List;
import java.util.Properties;
import java.util.Map;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.function.Function;
import java.util.stream.Collectors;
import java.util.stream.Stream;

import org.junit.jupiter.api.Test;

import es.gob.afirma.signvalidation.SignValidity;
import es.gob.afirma.signvalidation.SignValidity.SIGN_DETAIL_TYPE;
import es.gob.afirma.signvalidation.SignValidity.VALIDITY_ERROR;

/** Quien firmo y cuando, sobre firmas y cofirmas hechas aqui mismo. */
class PreviousSignaturesBridgeTest {

    private static final String ALGORITHM = "SHA256withRSA";

    @Test
    void an_unsigned_pdf_has_no_previous_signatures() throws Exception {
        assertTrue(PreviousSignaturesBridge.read(TestFixtures.samplePdf()).signatures().isEmpty());
    }

    @Test
    void a_signed_pdf_reports_the_signer_and_the_signing_time() throws Exception {
        final X509Certificate signer = TestFixtures.activeCertificate();
        final byte[] pdf = signed(TestFixtures.samplePdf(),
                TestFixtures.certificateChain(), TestFixtures.privateKey());

        final List<PreviousSignaturesBridge.Signature> signatures =
                PreviousSignaturesBridge.read(pdf).signatures();

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
                PreviousSignaturesBridge.read(twice).signatures();

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

    @Test
    void a_freshly_signed_pdf_reports_its_signature_as_valid() throws Exception {
        final PreviousSignaturesBridge.Signature signature = PreviousSignaturesBridge.read(
                signed(TestFixtures.samplePdf(), TestFixtures.certificateChain(),
                        TestFixtures.privateKey())).signatures().get(0);

        assertEquals(PreviousSignaturesBridge.Status.VALID, signature.status(),
                "motivo: " + signature.reason());
        assertNull(signature.reason());
    }

    @Test
    void a_signature_whose_signed_bytes_were_altered_is_broken_with_its_reason() throws Exception {
        final byte[] altered = TestFixtures.withOneByteChangedInsideTheSignedRange(
                signed(TestFixtures.samplePdf(), TestFixtures.certificateChain(),
                        TestFixtures.privateKey()));

        final List<PreviousSignaturesBridge.Signature> signatures =
                PreviousSignaturesBridge.read(altered).signatures();

        assertEquals(1, signatures.size());
        assertEquals(PreviousSignaturesBridge.Status.BROKEN, signatures.get(0).status());
        assertEquals("NO_MATCH_DATA", signatures.get(0).reason());
    }

    @Test
    void a_signature_added_after_a_certified_pdf_is_broken_with_its_reason() throws Exception {
        final List<PreviousSignaturesBridge.Signature> signatures = PreviousSignaturesBridge.read(
                TestFixtures.certifiedPdfWithSignatureInALaterRevision()).signatures();

        assertEquals(2, signatures.size());
        assertEquals(PreviousSignaturesBridge.Status.VALID, signatures.get(0).status(),
                "la firma que certifica el PDF sigue siendo valida");
        assertEquals(PreviousSignaturesBridge.Status.BROKEN, signatures.get(1).status());
        assertEquals("CERTIFIED_SIGN_REVISION", signatures.get(1).reason());
    }

    @Test
    void a_signature_with_an_unrecognized_subfilter_cannot_be_validated() throws Exception {
        final byte[] pdf = TestFixtures.signedWithUnrecognizedSubFilter(TestFixtures.samplePdf(),
                TestFixtures.certificateChain(), TestFixtures.privateKey());

        final List<PreviousSignaturesBridge.Signature> signatures =
                PreviousSignaturesBridge.read(pdf).signatures();

        assertEquals(1, signatures.size());
        assertEquals(PreviousSignaturesBridge.Status.UNVERIFIABLE, signatures.get(0).status());
        assertEquals("UNKOWN_SIGNATURE_FORMAT", signatures.get(0).reason());
    }

    @Test
    void a_recognized_signature_is_not_reclassified_by_a_later_unrecognized_subfilter()
            throws Exception {
        final byte[] once = signed(TestFixtures.samplePdf(),
                TestFixtures.certificateChain(), TestFixtures.privateKey());
        Thread.sleep(1_100);
        final byte[] twice = TestFixtures.signedWithUnrecognizedSubFilter(once,
                TestFixtures.otherCertificateChain(), TestFixtures.otherPrivateKey());

        final List<PreviousSignaturesBridge.Signature> signatures =
                PreviousSignaturesBridge.read(twice).signatures();

        assertEquals(2, signatures.size());
        assertEquals(PreviousSignaturesBridge.Status.NOT_FULLY_CHECKED, signatures.get(0).status(),
                "motivo: " + signatures.get(0).reason());
        assertEquals(PreviousSignaturesBridge.Status.UNVERIFIABLE, signatures.get(1).status());
        assertEquals("UNKOWN_SIGNATURE_FORMAT", signatures.get(1).reason());
    }

    @Test
    void each_verdict_of_the_original_validator_maps_to_its_status() {
        assertEquals(Map.of(
                VALIDITY_ERROR.CERTIFICATE_EXPIRED, PreviousSignaturesBridge.Status.CERTIFICATE_EXPIRED,
                VALIDITY_ERROR.CERTIFICATE_NOT_VALID_YET,
                        PreviousSignaturesBridge.Status.CERTIFICATE_NOT_YET_VALID,
                VALIDITY_ERROR.NO_MATCH_DATA, PreviousSignaturesBridge.Status.BROKEN,
                VALIDITY_ERROR.CORRUPTED_SIGN, PreviousSignaturesBridge.Status.BROKEN,
                VALIDITY_ERROR.CERTIFIED_SIGN_REVISION, PreviousSignaturesBridge.Status.BROKEN,
                VALIDITY_ERROR.ALGORITHM_NOT_SUPPORTED, PreviousSignaturesBridge.Status.UNVERIFIABLE,
                VALIDITY_ERROR.UNKOWN_SIGNATURE_FORMAT, PreviousSignaturesBridge.Status.UNVERIFIABLE,
                VALIDITY_ERROR.SIGN_PROFILE_NOT_CHECKED,
                        PreviousSignaturesBridge.Status.NOT_FULLY_CHECKED),
                Stream.of(VALIDITY_ERROR.CERTIFICATE_EXPIRED, VALIDITY_ERROR.CERTIFICATE_NOT_VALID_YET,
                        VALIDITY_ERROR.NO_MATCH_DATA, VALIDITY_ERROR.CORRUPTED_SIGN,
                        VALIDITY_ERROR.CERTIFIED_SIGN_REVISION, VALIDITY_ERROR.ALGORITHM_NOT_SUPPORTED,
                        VALIDITY_ERROR.UNKOWN_SIGNATURE_FORMAT, VALIDITY_ERROR.SIGN_PROFILE_NOT_CHECKED)
                        .collect(Collectors.toMap(Function.identity(), error ->
                                PreviousSignaturesBridge.statusOf(
                                        new SignValidity(SIGN_DETAIL_TYPE.KO, error)))));
        assertEquals(PreviousSignaturesBridge.Status.VALID,
                PreviousSignaturesBridge.statusOf(new SignValidity(SIGN_DETAIL_TYPE.OK, null)));
    }

    @Test
    void validating_the_previous_signatures_makes_no_network_request() throws Exception {
        final byte[] pdf = signed(TestFixtures.samplePdf(), TestFixtures.certificateChain(),
                TestFixtures.privateKey());
        final List<URI> requested = new CopyOnWriteArrayList<>();
        final ProxySelector previous = ProxySelector.getDefault();
        ProxySelector.setDefault(new ProxySelector() {
            @Override
            public List<Proxy> select(final URI uri) {
                requested.add(uri);
                return List.of(Proxy.NO_PROXY);
            }

            @Override
            public void connectFailed(final URI uri, final SocketAddress address,
                    final IOException e) {
                requested.add(uri);
            }
        });
        try {
            PreviousSignaturesBridge.read(pdf);
        }
        finally {
            ProxySelector.setDefault(previous);
        }

        assertEquals(List.of(), requested);
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
