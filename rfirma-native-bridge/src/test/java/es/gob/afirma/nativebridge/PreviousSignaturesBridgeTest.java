package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.net.Proxy;
import java.net.ProxySelector;
import java.net.SocketAddress;
import java.net.URI;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.PrivateKey;
import java.security.Signature;
import java.security.cert.X509Certificate;
import java.time.Duration;
import java.time.Instant;
import java.util.Base64;
import java.util.Date;
import java.util.List;
import java.util.Properties;
import java.util.concurrent.CopyOnWriteArrayList;

import org.junit.jupiter.api.Test;

import es.gob.afirma.signvalidation.SignValidity;
import es.gob.afirma.signvalidation.SignValidity.SIGN_DETAIL_TYPE;
import es.gob.afirma.signvalidation.SignValidity.VALIDITY_ERROR;
import es.gob.afirma.signvalidation.ValidatePdfSignature;

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
        assertEquals(signer.getNotBefore().toInstant(), Instant.parse(signature.validFrom()));
        assertEquals(signer.getNotAfter().toInstant(), Instant.parse(signature.validUntil()));
        assertEquals(ALGORITHM, signature.signatureAlgorithm());
        assertEquals("PAdES B-B-Level", signature.profile());
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
    void the_profile_is_attributed_only_to_the_signature_of_the_highest_revision() throws Exception {
        final byte[] once = signed(TestFixtures.samplePdf(),
                TestFixtures.certificateChain(), TestFixtures.privateKey());
        Thread.sleep(1_100);
        final byte[] twice = signed(once,
                TestFixtures.otherCertificateChain(), TestFixtures.otherPrivateKey());

        final List<PreviousSignaturesBridge.Signature> signatures =
                PreviousSignaturesBridge.read(twice).signatures();

        assertNull(signatures.get(0).profile());
        assertEquals("PAdES B-B-Level", signatures.get(1).profile());
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

        assertEquals(PreviousSignaturesBridge.Validity.VALID, signature.validity());
        assertNull(signature.validityReason());
    }

    @Test
    void a_pdf_signed_with_the_expired_certificate_of_the_kit_is_certificate_expired()
            throws Exception {
        final PreviousSignaturesBridge.Signature signature = PreviousSignaturesBridge.read(
                signed(TestFixtures.samplePdf(), TestFixtures.expiredCertificateChain(),
                        TestFixtures.expiredPrivateKey())).signatures().get(0);

        assertEquals(PreviousSignaturesBridge.Validity.EXPIRED, signature.validity());
        assertEquals(PreviousSignaturesBridge.Problem.CERTIFICATE_EXPIRED,
                signature.validityReason().problem());
    }

    @Test
    void a_signature_whose_signed_bytes_were_altered_is_invalid_with_its_reason() throws Exception {
        final byte[] altered = TestFixtures.withOneByteChangedInsideTheSignedRange(
                signed(TestFixtures.samplePdf(), TestFixtures.certificateChain(),
                        TestFixtures.privateKey()));

        final List<PreviousSignaturesBridge.Signature> signatures =
                PreviousSignaturesBridge.read(altered).signatures();

        assertEquals(1, signatures.size());
        assertEquals(PreviousSignaturesBridge.Validity.INVALID, signatures.get(0).validity());
        assertEquals(PreviousSignaturesBridge.Problem.MODIFIED_AFTER_SIGNING,
                signatures.get(0).validityReason().problem());
    }

    @Test
    void a_signature_added_after_a_certified_pdf_is_invalid_with_its_reason() throws Exception {
        final List<PreviousSignaturesBridge.Signature> signatures = PreviousSignaturesBridge.read(
                TestFixtures.certifiedPdfWithSignatureInALaterRevision()).signatures();

        assertEquals(2, signatures.size());
        assertEquals(PreviousSignaturesBridge.Validity.VALID, signatures.get(0).validity(),
                "la firma que certifica el PDF sigue siendo valida");
        assertEquals(PreviousSignaturesBridge.Validity.INVALID, signatures.get(1).validity());
        assertEquals(PreviousSignaturesBridge.Problem.COSIGN_NOT_ADMITTED,
                signatures.get(1).validityReason().problem());
    }

    @Test
    void the_certification_that_closes_a_pdf_is_marked_and_named_in_the_later_cosign()
            throws Exception {
        final List<PreviousSignaturesBridge.Signature> signatures = PreviousSignaturesBridge.read(
                TestFixtures.certifiedPdfWithSignatureInALaterRevision()).signatures();

        assertTrue(signatures.get(0).closesDocument());
        assertEquals(PreviousSignaturesBridge.Validity.VALID, signatures.get(0).validity());
        assertFalse(signatures.get(1).closesDocument());
        assertEquals(PreviousSignaturesBridge.Validity.INVALID, signatures.get(1).validity());
        assertEquals(PreviousSignaturesBridge.Problem.COSIGN_NOT_ADMITTED,
                signatures.get(1).validityReason().problem());
        assertEquals(PreviousSignaturesBridge.readable(
                        TestFixtures.activeCertificate().getSubjectX500Principal()),
                signatures.get(1).validityReason().closedBy());
    }

    @Test
    void with_several_certifications_the_last_one_closes_the_pdf() throws Exception {
        final List<PreviousSignaturesBridge.Signature> signatures = PreviousSignaturesBridge.read(
                TestFixtures.pdfCertifiedTwiceAndSignedAfter()).signatures();

        assertEquals(3, signatures.size());
        assertFalse(signatures.get(0).closesDocument());
        assertEquals(PreviousSignaturesBridge.Validity.VALID, signatures.get(0).validity());
        assertTrue(signatures.get(1).closesDocument());
        assertEquals(PreviousSignaturesBridge.Validity.VALID, signatures.get(1).validity(),
                "motivo: " + signatures.get(1).validityReason());
        assertEquals(PreviousSignaturesBridge.Validity.INVALID, signatures.get(2).validity(),
                "la cofirma no admitida pesa mas que su certificado caducado");
        assertEquals(PreviousSignaturesBridge.Problem.COSIGN_NOT_ADMITTED,
                signatures.get(2).validityReason().problem());
        assertEquals(PreviousSignaturesBridge.readable(
                        TestFixtures.otherCertificateChain()[0].getSubjectX500Principal()),
                signatures.get(2).validityReason().closedBy());
    }

    @Test
    void two_ordinary_signatures_are_both_valid_as_the_original_validator_says() throws Exception {
        final byte[] pdf = TestFixtures.pdfWithTwoOrdinarySignatures();

        final List<SignValidity> original = new ValidatePdfSignature().validate(pdf, true);
        final List<PreviousSignaturesBridge.Signature> signatures =
                PreviousSignaturesBridge.read(pdf).signatures();

        assertEquals(List.of(SIGN_DETAIL_TYPE.OK),
                original.stream().map(SignValidity::getValidity).toList(),
                "el rev <= 0 del original no marca dos firmas corrientes: " + original);
        assertEquals(2, signatures.size());
        for (final PreviousSignaturesBridge.Signature signature : signatures) {
            assertEquals(PreviousSignaturesBridge.Validity.VALID, signature.validity(),
                    "motivo: " + signature.validityReason());
            assertFalse(signature.closesDocument());
        }
    }

    @Test
    void a_signature_stamped_while_its_certificate_was_in_force_has_no_certificate_problem()
            throws Exception {
        final X509Certificate expired = TestFixtures.expiredCertificate();
        final List<SignValidity> today = List.of(
                new SignValidity(SIGN_DETAIL_TYPE.KO, VALIDITY_ERROR.CERTIFICATE_EXPIRED),
                new SignValidity(SIGN_DETAIL_TYPE.UNKNOWN, VALIDITY_ERROR.SIGN_PROFILE_NOT_CHECKED));

        final List<SignValidity> atStamp = PreviousSignaturesBridge.atStampTime(today, expired,
                Date.from(expired.getNotAfter().toInstant().minus(Duration.ofDays(1))));

        assertEquals(List.of(VALIDITY_ERROR.SIGN_PROFILE_NOT_CHECKED),
                atStamp.stream().map(SignValidity::getError).toList());
    }

    @Test
    void a_signature_stamped_after_its_certificate_expired_is_still_expired() throws Exception {
        final X509Certificate expired = TestFixtures.expiredCertificate();

        final List<SignValidity> atStamp = PreviousSignaturesBridge.atStampTime(List.of(), expired,
                Date.from(expired.getNotAfter().toInstant().plus(Duration.ofDays(1))));

        assertEquals(List.of(VALIDITY_ERROR.CERTIFICATE_EXPIRED),
                atStamp.stream().map(SignValidity::getError).toList());
    }

    @Test
    void a_signature_stamped_before_its_certificate_expired_is_valid_and_dated_by_the_stamp()
            throws Exception {
        final byte[] pdf = Files.readAllBytes(Path.of("..", "testdata", "previous-signatures",
                "pades-stamped-while-in-force.pdf"));

        final PreviousSignaturesBridge.Signature signature =
                PreviousSignaturesBridge.read(pdf).signatures().get(0);

        assertEquals(PreviousSignaturesBridge.Validity.VALID, signature.validity(),
                "motivo: " + signature.validityReason());
        assertEquals(new PreviousSignaturesBridge.SigningDate("2019-06-01T00:00:00Z",
                "CN=rfirma backdated TSA"), signature.signingDate());
    }

    @Test
    void a_signature_without_a_stamp_has_its_date_declared() throws Exception {
        final PreviousSignaturesBridge.Signature signature = PreviousSignaturesBridge.read(
                signed(TestFixtures.samplePdf(), TestFixtures.certificateChain(),
                        TestFixtures.privateKey())).signatures().get(0);

        assertEquals(new PreviousSignaturesBridge.SigningDate(signature.signingTime(), null),
                signature.signingDate());
    }

    @Test
    void a_signature_with_a_stamp_of_its_own_has_its_date_stamped_by_the_tsa() throws Exception {
        final byte[] pdf = Files.readAllBytes(
                Path.of("..", "testdata", "previous-signatures", "pades-long-term-active.pdf"));

        final PreviousSignaturesBridge.SigningDate date =
                PreviousSignaturesBridge.read(pdf).signatures().get(0).signingDate();

        assertEquals("CN=rfirma fake TSA", date.tsa());
        assertTrue(Instant.parse(date.at()).isAfter(Instant.parse("2026-01-01T00:00:00Z")),
                date.at());
    }

    @Test
    void a_signature_with_an_unrecognized_subfilter_is_invalid_of_unknown_type() throws Exception {
        // El ciclo de rFirma fija el /SubFilter: el .so no puede fabricar esta muestra.
        final PreviousSignaturesBridge.Signature signature = PreviousSignaturesBridge.read(
                TestFixtures.signedWithUnrecognizedSubFilter(TestFixtures.samplePdf(),
                        TestFixtures.certificateChain(), TestFixtures.privateKey()))
                .signatures().get(0);

        assertEquals(PreviousSignaturesBridge.Validity.INVALID, signature.validity());
        assertEquals(PreviousSignaturesBridge.Problem.UNKNOWN_SIGNATURE_TYPE,
                signature.validityReason().problem());
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
        assertEquals(PreviousSignaturesBridge.Validity.VALID, signatures.get(0).validity());
        assertEquals(PreviousSignaturesBridge.Validity.INVALID, signatures.get(1).validity());
        assertEquals(PreviousSignaturesBridge.Problem.UNKNOWN_SIGNATURE_TYPE,
                signatures.get(1).validityReason().problem());
    }

    @Test
    void a_revision_added_after_the_last_signature_without_signing_it_changed_the_document()
            throws Exception {
        final byte[] pdf = TestFixtures.withThePageRepaintedAfterSigning(
                signed(TestFixtures.samplePdf(), TestFixtures.certificateChain(),
                        TestFixtures.privateKey()));

        assertTrue(PreviousSignaturesBridge.read(pdf).changedAfterLastSignature());
    }

    @Test
    void a_normal_cosign_where_every_revision_carries_its_own_signature_does_not_flag_it()
            throws Exception {
        final byte[] once = signed(TestFixtures.samplePdf(),
                TestFixtures.certificateChain(), TestFixtures.privateKey());
        Thread.sleep(1_100);
        final byte[] twice = signed(once,
                TestFixtures.otherCertificateChain(), TestFixtures.otherPrivateKey());

        assertFalse(PreviousSignaturesBridge.read(twice).changedAfterLastSignature());
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
