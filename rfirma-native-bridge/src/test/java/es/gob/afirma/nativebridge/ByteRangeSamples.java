package es.gob.afirma.nativebridge;

import java.nio.charset.StandardCharsets;
import java.security.Signature;
import java.util.Arrays;
import java.util.Base64;
import java.util.HexFormat;
import java.util.List;
import java.util.Properties;
import java.util.function.UnaryOperator;

import org.spongycastle.asn1.ASN1InputStream;
import org.spongycastle.cert.jcajce.JcaCertStore;
import org.spongycastle.cms.CMSProcessableByteArray;
import org.spongycastle.cms.CMSSignedDataGenerator;
import org.spongycastle.cms.jcajce.JcaSignerInfoGeneratorBuilder;
import org.spongycastle.operator.jcajce.JcaContentSignerBuilder;
import org.spongycastle.operator.jcajce.JcaDigestCalculatorProviderBuilder;

import com.aowagie.text.pdf.AcroFields;
import com.aowagie.text.pdf.PdfName;
import com.aowagie.text.pdf.PdfReader;

/**
 * PDF firmados cuyo {@code /ByteRange} se reescribe despues de firmar, en su sitio y sin mover
 * ningun otro byte, para que la firma siga cuadrando con lo que su rango cubre.
 */
final class ByteRangeSamples {

    private static final String ALGORITHM = "SHA256withRSA";

    private static final byte[] BYTE_RANGE = "/ByteRange".getBytes(StandardCharsets.US_ASCII);

    /** Lo que se escribe en el hueco de {@code /Contents} y la firma no cubre. */
    static final String UNSIGNED_ENTRY = " /Unsigned (Texto que la firma no cubre)";

    private ByteRangeSamples() { }

    static byte[] signed(final byte[] pdf) throws Exception {
        return signedWith(pdf, new Properties());
    }

    static byte[] signedWith(final byte[] pdf, final Properties extraParams) throws Exception {
        final PadesBridge.PreSignResult pre = PadesBridge.preSign(pdf, ALGORITHM,
                TestFixtures.certificateChain(), extraParams);

        final Signature signature = Signature.getInstance(ALGORITHM);
        signature.initSign(TestFixtures.privateKey());
        signature.update(Base64.getDecoder().decode(pre.preSignB64()));

        return PadesBridge.postSign(pdf, TestFixtures.certificateChain(), pre.stamp(),
                pre.session(), Base64.getEncoder().encodeToString(signature.sign()));
    }

    /** Una revision mas firmada con el {@code /SubFilter} de un sello de documento. */
    static byte[] withADocTimeStampAfter(final byte[] pdf) throws Exception {
        final Properties stamp = new Properties();
        stamp.setProperty("signatureSubFilter", "ETSI.RFC3161");
        return signedWith(pdf, stamp);
    }

    /**
     * La ultima firma con su {@code /Contents} recortado a su CMS y, detras, una entrada que cae
     * en el hueco del rango: la firma sigue cuadrando y esos bytes no estan firmados.
     */
    static byte[] withUnsignedBytesInsideTheContentsGap(final byte[] pdf) throws Exception {
        final long[] range = lastRange(pdf);
        final byte[] cms = new ASN1InputStream(lastContents(pdf)).readObject().getEncoded();
        final String gap = "<" + HexFormat.of().formatHex(cms) + ">" + UNSIGNED_ENTRY;
        final int slot = (int) (range[2] - range[1]);
        if (gap.length() > slot) {
            throw new IllegalStateException("la entrada no cabe en el hueco de /Contents");
        }
        final byte[] altered = pdf.clone();
        final byte[] written = (gap + " ".repeat(slot - gap.length()))
                .getBytes(StandardCharsets.US_ASCII);
        System.arraycopy(written, 0, altered, (int) range[1], written.length);
        return altered;
    }

    /** El PDF con bytes de sobra tras su ultimo {@code %%EOF}, fuera de toda revision. */
    static byte[] withSpareBytesAfterTheLastEof(final byte[] pdf) {
        final byte[] spare = Arrays.copyOf(pdf, pdf.length + 16);
        Arrays.fill(spare, pdf.length, spare.length, (byte) 0);
        return spare;
    }

    /** La ultima firma con otro texto en su {@code /ByteRange}, en su sitio y en los blancos que lo siguen. */
    static byte[] withTheByteRangeWritten(final byte[] pdf, final String array) {
        final int[] at = lastRangeArray(pdf);
        int end = at[1];
        while (pdf[end] == ' ') {
            end++;
        }
        final int room = end - at[0];
        if (array.length() > room) {
            throw new IllegalStateException("el rango no cabe en el sitio del original");
        }
        final byte[] altered = pdf.clone();
        final byte[] written = (array + " ".repeat(room - array.length()))
                .getBytes(StandardCharsets.US_ASCII);
        System.arraycopy(written, 0, altered, at[0], written.length);
        return altered;
    }

    /** La ultima firma con su segundo tramo 2^32 bytes mas alla y un fin que no cabe en un {@code long}. */
    static byte[] withAByteRangeBeyondEveryOffset(final byte[] pdf) {
        final long[] range = lastRange(pdf);
        return withTheByteRangeWritten(pdf, "[0 " + range[1] + " " + ((1L << 32) + range[2]) + " "
                + Long.MAX_VALUE + "]");
    }

    /**
     * La ultima firma rehecha sobre el rango que da {@code change} a partir del suyo, con el CMS
     * nuevo en el mismo hueco de {@code /Contents}: el rango es otro y la firma cuadra con el.
     */
    static byte[] resignedOver(final byte[] pdf, final UnaryOperator<long[]> change)
            throws Exception {
        final long[] original = lastRange(pdf);
        final long[] range = change.apply(original.clone());
        final byte[] rewritten = withTheByteRangeWritten(pdf,
                "[" + range[0] + " " + range[1] + " " + range[2] + " " + range[3] + "]");

        final byte[] signedBytes = new byte[(int) (range[1] + range[3])];
        System.arraycopy(rewritten, (int) range[0], signedBytes, 0, (int) range[1]);
        System.arraycopy(rewritten, (int) range[2], signedBytes, (int) range[1], (int) range[3]);

        final String hex = HexFormat.of().formatHex(detachedCms(signedBytes));
        final int digits = (int) (original[2] - original[1]) - 2;
        if (hex.length() > digits) {
            throw new IllegalStateException("el CMS no cabe en el hueco de /Contents");
        }
        final byte[] written = (hex + "0".repeat(digits - hex.length()))
                .getBytes(StandardCharsets.US_ASCII);
        System.arraycopy(written, 0, rewritten, (int) original[1] + 1, written.length);
        return rewritten;
    }

    private static byte[] detachedCms(final byte[] content) throws Exception {
        final CMSSignedDataGenerator generator = new CMSSignedDataGenerator();
        generator.addSignerInfoGenerator(new JcaSignerInfoGeneratorBuilder(
                new JcaDigestCalculatorProviderBuilder().build())
                .build(new JcaContentSignerBuilder(ALGORITHM).build(TestFixtures.privateKey()),
                        TestFixtures.certificateChain()[0]));
        generator.addCertificates(new JcaCertStore(List.of(TestFixtures.certificateChain())));
        return generator.generate(new CMSProcessableByteArray(content), false).getEncoded();
    }

    static long[] lastRange(final byte[] pdf) {
        final int[] at = lastRangeArray(pdf);
        final String array = new String(pdf, at[0] + 1, at[1] - at[0] - 2, StandardCharsets.US_ASCII);
        return Arrays.stream(array.trim().split("\\s+")).mapToLong(Long::parseLong).toArray();
    }

    /** Desde el {@code [} hasta pasado el {@code ]} del ultimo {@code /ByteRange} del fichero. */
    private static int[] lastRangeArray(final byte[] pdf) {
        int key = -1;
        for (int at = pdf.length - BYTE_RANGE.length; at >= 0 && key < 0; at--) {
            if (Arrays.equals(pdf, at, at + BYTE_RANGE.length, BYTE_RANGE, 0, BYTE_RANGE.length)) {
                key = at;
            }
        }
        int open = key + BYTE_RANGE.length;
        while (pdf[open] != '[') {
            open++;
        }
        int close = open;
        while (pdf[close] != ']') {
            close++;
        }
        return new int[] {open, close + 1};
    }

    private static byte[] lastContents(final byte[] pdf) throws Exception {
        final AcroFields fields = new PdfReader(pdf).getAcroFields();
        String latest = null;
        for (final String name : fields.getSignatureNames()) {
            if (latest == null || fields.getRevision(name) > fields.getRevision(latest)) {
                latest = name;
            }
        }
        return fields.getSignatureDictionary(latest).getAsString(PdfName.CONTENTS)
                .getOriginalBytes();
    }
}
