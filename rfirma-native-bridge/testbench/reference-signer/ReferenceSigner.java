import java.io.ByteArrayOutputStream;
import java.io.FileInputStream;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.security.KeyStore;
import java.util.GregorianCalendar;
import java.util.HexFormat;
import java.util.Properties;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

import org.spongycastle.asn1.ASN1InputStream;

import com.aowagie.text.Document;
import com.aowagie.text.Paragraph;
import com.aowagie.text.pdf.PdfWriter;

import es.gob.afirma.core.signers.AOSignConstants;
import es.gob.afirma.core.signers.CounterSignTarget;
import es.gob.afirma.signers.cades.AOCAdESSigner;
import es.gob.afirma.signers.pades.AOPDFSigner;
import es.gob.afirma.signers.tsp.pkcs7.CMSTimestamper;
import es.gob.afirma.signers.tsp.pkcs7.TsaParams;
import es.gob.afirma.signers.xades.AOFacturaESigner;
import es.gob.afirma.signers.xades.AOXAdESSigner;

/** Firma con los firmadores monofasicos del 1.9.2 para producir testdata/reference/ y testdata/previous-signatures/. */
public final class ReferenceSigner {

    private static final String ALGORITHM = "SHA256withRSA";

    public static void main(String[] args) throws Exception {
        if (args.length < 1) {
            usageAndExit();
        }
        switch (args[0]) {
            case "cades" -> cades(args);
            case "xades" -> xades(args);
            case "facturae" -> facturae(args);
            case "pdf" -> pdf(args);
            case "pades-timestamped" -> padesTimestamped(args);
            case "cosign" -> cosign(args);
            case "countersign" -> countersign(args);
            default -> usageAndExit();
        }
    }

    private static void usageAndExit() {
        System.err.println("""
                Uso:
                  ReferenceSigner cades <implicit|explicit> <entrada> <p12> <pin> <salida>
                  ReferenceSigner xades <detached|enveloping|enveloped> <entrada.xml> <p12> <pin> <salida>
                  ReferenceSigner facturae <invoice.xml> <p12> <pin> <salida>
                  ReferenceSigner pdf <salida>
                  ReferenceSigner pades-timestamped <entrada.pdf> <p12> <pin> <tsaURL> <salida>
                  ReferenceSigner cosign <cades|xades> <firma-origen> <p12> <pin> <salida>
                  ReferenceSigner countersign <cades|xades> <tree|leafs> <firma-origen> <p12> <pin> <salida>
                """);
        System.exit(2);
    }

    private static void cades(String[] args) throws Exception {
        Properties extraParams = new Properties();
        extraParams.setProperty("mode", args[1]);
        byte[] data = Files.readAllBytes(Path.of(args[2]));
        KeyStore.PrivateKeyEntry pke = loadKey(args[3], args[4]);
        byte[] result = new AOCAdESSigner().sign(
                data, ALGORITHM, pke.getPrivateKey(), pke.getCertificateChain(), extraParams);
        Files.write(Path.of(args[5]), result);
    }

    private static void xades(String[] args) throws Exception {
        Properties extraParams = new Properties();
        extraParams.setProperty("format", xadesFormat(args[1]));
        byte[] data = Files.readAllBytes(Path.of(args[2]));
        KeyStore.PrivateKeyEntry pke = loadKey(args[3], args[4]);
        byte[] result = new AOXAdESSigner().sign(
                data, ALGORITHM, pke.getPrivateKey(), pke.getCertificateChain(), extraParams);
        Files.write(Path.of(args[5]), result);
    }

    private static String xadesFormat(String mode) {
        return switch (mode) {
            case "detached" -> AOSignConstants.SIGN_FORMAT_XADES_DETACHED;
            case "enveloping" -> AOSignConstants.SIGN_FORMAT_XADES_ENVELOPING;
            case "enveloped" -> AOSignConstants.SIGN_FORMAT_XADES_ENVELOPED;
            default -> throw new IllegalArgumentException("formato XAdES desconocido: " + mode);
        };
    }

    private static void facturae(String[] args) throws Exception {
        byte[] data = Files.readAllBytes(Path.of(args[1]));
        KeyStore.PrivateKeyEntry pke = loadKey(args[2], args[3]);
        byte[] result = new AOFacturaESigner().sign(
                data, ALGORITHM, pke.getPrivateKey(), pke.getCertificateChain(), new Properties());
        Files.write(Path.of(args[4]), result);
    }

    private static void pdf(String[] args) throws Exception {
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        Document document = new Document();
        PdfWriter.getInstance(document, out);
        document.open();
        document.add(new Paragraph("Documento de prueba de rfirma."));
        document.close();
        Files.write(Path.of(args[1]), out.toByteArray());
    }

    private static void padesTimestamped(String[] args) throws Exception {
        byte[] data = Files.readAllBytes(Path.of(args[1]));
        KeyStore.PrivateKeyEntry pke = loadKey(args[2], args[3]);
        Properties extraParams = new Properties();
        extraParams.setProperty("headless", "true");
        byte[] signed = new AOPDFSigner().sign(
                data, ALGORITHM, pke.getPrivateKey(), pke.getCertificateChain(), extraParams);
        Properties tsaParams = new Properties();
        tsaParams.setProperty("tsaURL", args[4]);
        tsaParams.setProperty("tsaPolicy", "0.4.0.2023.1.1");
        tsaParams.setProperty("tsaHashAlgorithm", "SHA-256");
        Files.write(Path.of(args[5]), withSignatureTimestamp(signed, new TsaParams(tsaParams)));
    }

    // El PdfTimestamper del 1.9.2 devuelve la firma sin sello (ADR-0030): se sella el CMS en su
    // hueco de /Contents, fuera del ByteRange.
    private static byte[] withSignatureTimestamp(byte[] pdf, TsaParams tsa) throws Exception {
        String text = new String(pdf, StandardCharsets.ISO_8859_1);
        Matcher byteRange = Pattern
                .compile("/ByteRange\\s*\\[\\s*(\\d+)\\s+(\\d+)\\s+(\\d+)\\s+(\\d+)\\s*\\]")
                .matcher(text);
        if (!byteRange.find()) {
            throw new IllegalStateException("el PDF firmado no trae ByteRange");
        }
        int open = Integer.parseInt(byteRange.group(2));
        int close = Integer.parseInt(byteRange.group(3)) - 1;
        byte[] container = HexFormat.of().parseHex(text.substring(open + 1, close));
        byte[] cms;
        try (ASN1InputStream in = new ASN1InputStream(container)) {
            cms = in.readObject().getEncoded("DER");
        }
        byte[] stamped = new CMSTimestamper(tsa)
                .addTimestamp(cms, tsa.getTsaHashAlgorithm(), new GregorianCalendar());
        String hex = HexFormat.of().formatHex(stamped);
        int room = close - open - 1;
        if (hex.length() > room) {
            throw new IllegalStateException(
                    "el sello no cabe en /Contents: " + hex.length() + " > " + room);
        }
        byte[] result = pdf.clone();
        byte[] padded = (hex + "0".repeat(room - hex.length())).getBytes(StandardCharsets.US_ASCII);
        System.arraycopy(padded, 0, result, open + 1, room);
        return result;
    }

    private static void cosign(String[] args) throws Exception {
        byte[] sign = Files.readAllBytes(Path.of(args[2]));
        KeyStore.PrivateKeyEntry pke = loadKey(args[3], args[4]);
        Properties extraParams = new Properties();
        byte[] result = switch (args[1]) {
            case "cades" -> new AOCAdESSigner().cosign(
                    sign, ALGORITHM, pke.getPrivateKey(), pke.getCertificateChain(), extraParams);
            case "xades" -> new AOXAdESSigner().cosign(
                    sign, ALGORITHM, pke.getPrivateKey(), pke.getCertificateChain(), extraParams);
            default -> throw new IllegalArgumentException("formato desconocido: " + args[1]);
        };
        Files.write(Path.of(args[5]), result);
    }

    private static void countersign(String[] args) throws Exception {
        CounterSignTarget target =
                "tree".equals(args[2]) ? CounterSignTarget.TREE : CounterSignTarget.LEAFS;
        byte[] sign = Files.readAllBytes(Path.of(args[3]));
        KeyStore.PrivateKeyEntry pke = loadKey(args[4], args[5]);
        Properties extraParams = new Properties();
        byte[] result = switch (args[1]) {
            case "cades" -> new AOCAdESSigner().countersign(
                    sign, ALGORITHM, target, null, pke.getPrivateKey(), pke.getCertificateChain(),
                    extraParams);
            case "xades" -> new AOXAdESSigner().countersign(
                    sign, ALGORITHM, target, null, pke.getPrivateKey(), pke.getCertificateChain(),
                    extraParams);
            default -> throw new IllegalArgumentException("formato desconocido: " + args[1]);
        };
        Files.write(Path.of(args[6]), result);
    }

    private static KeyStore.PrivateKeyEntry loadKey(String p12Path, String pin) throws Exception {
        KeyStore ks = KeyStore.getInstance("PKCS12");
        try (InputStream in = new FileInputStream(p12Path)) {
            ks.load(in, pin.toCharArray());
        }
        String alias = ks.aliases().nextElement();
        return (KeyStore.PrivateKeyEntry) ks.getEntry(
                alias, new KeyStore.PasswordProtection(pin.toCharArray()));
    }
}
