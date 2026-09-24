package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.ByteArrayInputStream;
import java.security.cert.X509Certificate;
import java.util.Properties;
import java.util.zip.ZipEntry;
import java.util.zip.ZipInputStream;

import org.bouncycastle.asn1.pkcs.PKCSObjectIdentifiers;
import org.bouncycastle.cms.CMSProcessableByteArray;
import org.bouncycastle.cms.CMSSignedData;
import org.bouncycastle.cms.SignerInformation;
import org.bouncycastle.cms.jcajce.JcaSimpleSignerInfoVerifierBuilder;
import org.junit.jupiter.api.Test;

import es.gob.afirma.core.signers.asic.ASiCUtil;

/** El sello de tiempo que pide la sede con {@code tsaURL} en CAdES y XAdES: o sale sellada, o no sale. */
class SignatureTimestampTest {

    private static final String XADES_NS = "http://uri.etsi.org/01903/v1.3.2#";
    private static final String INVALID_TSA_URL = "http://tsa invalida";

    @Test
    void a_cades_signature_that_asks_for_a_timestamp_carries_one_on_its_signer() throws Exception {
        final byte[] document = TestFixtures.challenge();
        try (FakeTsa tsa = FakeTsa.start()) {
            final byte[] signature = CadesCycle.sign(document, tsaAt(tsa.url()), "sign");

            final CMSSignedData cms =
                    new CMSSignedData(new CMSProcessableByteArray(document), signature);
            final SignerInformation signer = onlySignerOf(cms);
            assertTrue(isTimestamped(signer), "la firma CAdES no lleva el sello que se pidio");
            assertTrue(signer.verify(new JcaSimpleSignerInfoVerifierBuilder()
                    .build(TestFixtures.activeCertificate())), "el sello rompio la firma CAdES");
        }
    }

    @Test
    void a_cades_asic_s_container_carries_the_timestamped_signature() throws Exception {
        final Properties params = new Properties();
        params.setProperty("format", "CAdES-ASiC-S");
        try (FakeTsa tsa = FakeTsa.start()) {
            params.setProperty("tsaURL", tsa.url());
            final byte[] container = CadesCycle.sign(TestFixtures.challenge(), params, "sign");

            final byte[] signature = entryOf(container, ASiCUtil.ENTRY_NAME_BINARY_SIGNATURE);
            assertNotNull(signature, "el contenedor ASiC-S no lleva la firma");
            final CMSSignedData cms = new CMSSignedData(signature);
            assertNull(cms.getSignedContent(), "la firma del ASiC-S tiene que ser explicita");
            assertTrue(isTimestamped(onlySignerOf(cms)),
                    "la firma del contenedor ASiC-S no lleva el sello que se pidio");
        }
    }

    @Test
    void a_cades_signature_without_tsa_url_is_not_timestamped() throws Exception {
        final byte[] signature = CadesCycle.sign(TestFixtures.challenge(), new Properties(), "sign");

        assertFalse(isTimestamped(onlySignerOf(new CMSSignedData(signature))),
                "sin tsaURL no hay sello que poner");
    }

    @Test
    void a_cades_signature_with_an_invalid_tsa_url_fails_before_the_signing() {
        final TimestampFailedException failure = assertThrows(TimestampFailedException.class,
                () -> CadesCycle.preSign(TestFixtures.challenge(), tsaAt(INVALID_TSA_URL), "sign"));

        assertTrue(failure.getMessage().contains(INVALID_TSA_URL),
                "el fallo tiene que nombrar la URL que no se pudo usar: " + failure.getMessage());
    }

    @Test
    void a_cades_signature_whose_tsa_does_not_answer_fails_instead_of_leaving_unstamped()
            throws Exception {
        final byte[] document = TestFixtures.challenge();
        final X509Certificate[] chain = TestFixtures.certificateChain();
        final CadesBridge.PreSignResult pre =
                CadesCycle.preSign(document, tsaAt(FakeTsa.unreachableUrl()), "sign");

        assertThrows(TimestampFailedException.class, () -> CadesBridge.postSign(
                document, chain, pre.stamp(), pre.session(), CadesCycle.pkcs1For(pre)));
    }

    @Test
    void a_xades_signature_that_asks_for_a_timestamp_carries_one_and_stays_valid()
            throws Exception {
        try (FakeTsa tsa = FakeTsa.start()) {
            final byte[] signature = XadesCycle.sign(XadesCycle.referenceXml(), tsaAt(tsa.url()));

            assertEquals(1, XadesCycle.parse(signature)
                    .getElementsByTagNameNS(XADES_NS, "SignatureTimeStamp").getLength(),
                    "la firma XAdES no lleva el sello que se pidio");
            assertTrue(XadesCycle.xmlsecVerifies(signature), "el sello rompio la firma XAdES");
        }
    }

    @Test
    void a_xades_signature_with_an_invalid_tsa_url_fails_before_the_signing() {
        assertThrows(TimestampFailedException.class, () -> XadesCycle
                .preSign(XadesCycle.referenceXml(), tsaAt(INVALID_TSA_URL), "sign"));
    }

    @Test
    void a_xades_signature_whose_tsa_does_not_answer_fails_instead_of_leaving_unstamped()
            throws Exception {
        final String unreachable = FakeTsa.unreachableUrl();

        assertThrows(TimestampFailedException.class,
                () -> XadesCycle.sign(XadesCycle.referenceXml(), tsaAt(unreachable)));
    }

    private static Properties tsaAt(final String url) {
        final Properties params = new Properties();
        params.setProperty("tsaURL", url);
        return params;
    }

    private static SignerInformation onlySignerOf(final CMSSignedData cms) {
        assertEquals(1, cms.getSignerInfos().size(), "se esperaba un solo firmante");
        return cms.getSignerInfos().getSigners().iterator().next();
    }

    private static boolean isTimestamped(final SignerInformation signer) {
        return signer.getUnsignedAttributes() != null && signer.getUnsignedAttributes()
                .get(PKCSObjectIdentifiers.id_aa_signatureTimeStampToken) != null;
    }

    private static byte[] entryOf(final byte[] zip, final String name) throws Exception {
        try (ZipInputStream entries = new ZipInputStream(new ByteArrayInputStream(zip))) {
            for (ZipEntry entry = entries.getNextEntry(); entry != null;
                    entry = entries.getNextEntry()) {
                if (name.equals(entry.getName())) {
                    return entries.readAllBytes();
                }
            }
        }
        return null;
    }
}
