package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.ByteArrayOutputStream;
import java.security.GeneralSecurityException;
import java.security.MessageDigest;
import java.security.cert.X509Certificate;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

import org.junit.jupiter.api.Test;

import com.aowagie.text.pdf.PdfDictionary;
import com.aowagie.text.pdf.PdfName;
import com.aowagie.text.pdf.PdfReader;
import com.aowagie.text.pdf.PdfSignatureAppearance;
import com.aowagie.text.pdf.PdfStamper;
import com.aowagie.text.pdf.PdfString;
import org.spongycastle.asn1.DEROctetString;
import org.spongycastle.asn1.DERSequence;
import org.spongycastle.asn1.cms.AttributeTable;
import org.spongycastle.asn1.pkcs.PKCSObjectIdentifiers;
import org.spongycastle.cert.jcajce.JcaCertStore;
import org.spongycastle.cms.CMSProcessableByteArray;
import org.spongycastle.cms.CMSSignedData;
import org.spongycastle.cms.CMSSignedDataGenerator;
import org.spongycastle.cms.DefaultSignedAttributeTableGenerator;
import org.spongycastle.cms.jcajce.JcaSignerInfoGeneratorBuilder;
import org.spongycastle.operator.jcajce.JcaContentSignerBuilder;
import org.spongycastle.operator.jcajce.JcaDigestCalculatorProviderBuilder;

/** Una firma con MD5 no es valida en local y la sede la sigue aceptando como el original. */
class Md5SignaturesTest {

    private static final String MD5 = "MD5withRSA";

    private static final int CONTENTS_BYTES = 8192;

    @Test
    void a_pdf_signed_with_md5_is_invalid_with_the_unsupported_algorithm_reason() throws Exception {
        final PreviousSignaturesBridge.Signature signature =
                PreviousSignaturesBridge.read(md5Pdf()).signatures().get(0);

        assertEquals(PreviousSignaturesBridge.Validity.INVALID, signature.validity());
        assertEquals(PreviousSignaturesBridge.Problem.UNSUPPORTED_ALGORITHM,
                signature.validityReason().problem());
    }

    @Test
    void a_cades_signed_with_md5_is_invalid_with_the_unsupported_algorithm_reason()
            throws Exception {
        final PreviousSignaturesBridge.Signature signature =
                PreviousSignaturesBridge.read(md5Cades()).signatures().get(0);

        assertEquals(PreviousSignaturesBridge.Validity.INVALID, signature.validity());
        assertEquals(PreviousSignaturesBridge.Problem.UNSUPPORTED_ALGORITHM,
                signature.validityReason().problem());
    }

    @Test
    void a_pdf_signed_with_sha256_keeps_being_valid() throws Exception {
        final PreviousSignaturesBridge.Signature signature = PreviousSignaturesBridge.read(
                signedPdf("SHA256withRSA")).signatures().get(0);

        assertEquals(PreviousSignaturesBridge.Validity.VALID, signature.validity());
        assertNull(signature.validityReason());
    }

    @Test
    void a_cades_signed_with_sha1_keeps_being_valid() throws Exception {
        final PreviousSignaturesBridge.Signature signature = PreviousSignaturesBridge.read(
                cades("SHA1withRSA")).signatures().get(0);

        assertEquals(PreviousSignaturesBridge.Validity.VALID, signature.validity());
    }

    @Test
    void verify_adds_a_result_for_md5_and_not_for_sha256() throws Exception {
        final List<String> clean = ValidationBridge.results(signedPdf("SHA256withRSA"), "PAdES");
        final List<String> md5Pdf = ValidationBridge.results(md5Pdf(), "PAdES");
        final List<String> md5Cades = ValidationBridge.results(md5Cades(), "CAdES");

        assertEquals(clean.size() + 1, md5Pdf.size());
        assertTrue(md5Cades.size() > 1);
        assertTrue(md5Pdf.get(md5Pdf.size() - 1).length() > 0);
    }

    @Test
    void the_site_verdict_still_accepts_a_md5_pdf() throws Exception {
        final ValidationBridge.Verdict verdict =
                ValidationBridge.validate(md5Pdf(), "PAdES", false);

        assertEquals(ValidationBridge.VALID, verdict.outcome(), "motivo: " + verdict.reason());
    }

    @Test
    void the_site_verdict_still_accepts_a_md5_cades() throws Exception {
        final ValidationBridge.Verdict verdict =
                ValidationBridge.validate(md5Cades(), "CAdES", false);

        assertEquals(ValidationBridge.VALID, verdict.outcome(), "motivo: " + verdict.reason());
    }

    private static byte[] md5Pdf() throws Exception {
        return signedPdf(MD5);
    }

    private static byte[] md5Cades() throws Exception {
        return cades(MD5);
    }

    private static byte[] signedPdf(final String algorithm) throws Exception {
        final X509Certificate[] chain = TestFixtures.certificateChain();
        final ByteArrayOutputStream out = new ByteArrayOutputStream();
        final PdfStamper stamper = PdfStamper.createSignature(
                new PdfReader(TestFixtures.samplePdf()), out, '\0', null, true);
        final PdfSignatureAppearance appearance = stamper.getSignatureAppearance();
        appearance.setCrypto(null, chain, null, null);
        final PdfDictionary dictionary = new PdfDictionary();
        dictionary.put(PdfName.FILTER, new PdfName("Adobe.PPKLite"));
        dictionary.put(PdfName.SUBFILTER, new PdfName("adbe.pkcs7.detached"));
        appearance.setCryptoDictionary(dictionary);
        final HashMap<PdfName, Integer> sizes = new HashMap<>();
        sizes.put(PdfName.CONTENTS, CONTENTS_BYTES * 2 + 2);
        appearance.preClose(sizes);
        final byte[] ranges = appearance.getRangeStream().readAllBytes();
        final byte[] container = cmsOver(ranges, algorithm, false);
        final byte[] padded = new byte[CONTENTS_BYTES];
        System.arraycopy(container, 0, padded, 0, container.length);
        final PdfDictionary signature = new PdfDictionary();
        signature.put(PdfName.CONTENTS, new PdfString(padded).setHexWriting(true));
        appearance.close(signature);
        return out.toByteArray();
    }

    private static byte[] cades(final String algorithm) throws Exception {
        return cmsOver(TestFixtures.challenge(), algorithm, true);
    }

    private static byte[] cmsOver(final byte[] content, final String algorithm,
            final boolean implicit) throws Exception {
        final X509Certificate[] chain = TestFixtures.certificateChain();
        final CMSSignedDataGenerator generator = new CMSSignedDataGenerator();
        generator.addSignerInfoGenerator(new JcaSignerInfoGeneratorBuilder(
                new JcaDigestCalculatorProviderBuilder().build())
                .setSignedAttributeGenerator(new CadesAttributes(chain[0]))
                .build(new JcaContentSignerBuilder(algorithm).build(TestFixtures.privateKey()),
                        chain[0]));
        generator.addCertificates(new JcaCertStore(List.of(chain)));
        return generator.generate(new CMSProcessableByteArray(content), implicit)
                .toASN1Structure().getEncoded("DER");
    }

    /** Los atributos firmados de un CAdES: los de CMS y el certificado firmante. */
    private static final class CadesAttributes extends DefaultSignedAttributeTableGenerator {

        private final X509Certificate signer;

        CadesAttributes(final X509Certificate signer) {
            this.signer = signer;
        }

        @Override
        public AttributeTable getAttributes(final Map parameters) {
            try {
                final byte[] hash = MessageDigest.getInstance("SHA-1").digest(signer.getEncoded());
                final DERSequence essCertId = new DERSequence(new DEROctetString(hash));
                final DERSequence signingCertificate = new DERSequence(new DERSequence(essCertId));
                return super.getAttributes(parameters).add(
                        PKCSObjectIdentifiers.id_aa_signingCertificate, signingCertificate);
            }
            catch (final GeneralSecurityException e) {
                throw new IllegalStateException(e);
            }
        }
    }
}
