package probe;

import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.KeyStore;
import java.security.PrivateKey;
import java.security.cert.Certificate;
import java.util.Enumeration;
import java.util.Properties;

import es.gob.afirma.signers.pades.AOPDFSigner;

/** Firma un PDF con AutoFirma 1.9.1 (JVM completa) y el parametro signaturePages. */
public final class Probe {
    public static void main(final String[] args) throws Exception {
        final Path in = Path.of(args[0]);
        final Path p12 = Path.of(args[1]);
        final String pages = args[2];
        final Path out = Path.of(args[3]);

        final KeyStore ks = KeyStore.getInstance("PKCS12");
        try (InputStream is = Files.newInputStream(p12)) { ks.load(is, "1234".toCharArray()); }
        String alias = null;
        for (final Enumeration<String> e = ks.aliases(); e.hasMoreElements();) {
            final String a = e.nextElement();
            if (ks.isKeyEntry(a)) { alias = a; break; }
        }
        final PrivateKey key = (PrivateKey) ks.getKey(alias, "1234".toCharArray());
        final Certificate[] chain = ks.getCertificateChain(alias);

        final Properties p = new Properties();
        p.setProperty("signatureSubFilter", "ETSI.CAdES.detached");
        p.setProperty("signaturePages", pages);
        p.setProperty("signaturePositionOnPageLowerLeftX", "100");
        p.setProperty("signaturePositionOnPageLowerLeftY", "100");
        p.setProperty("signaturePositionOnPageUpperRightX", "300");
        p.setProperty("signaturePositionOnPageUpperRightY", "160");
        p.setProperty("layer2Text", "Firmado por: prueba rfirma #116\nsignaturePages=" + pages);
        p.setProperty("signReason", "Sondeo #116");

        final byte[] signed = new AOPDFSigner().sign(Files.readAllBytes(in), "SHA256withRSA", key, chain, p);
        Files.write(out, signed);
        System.out.println("OK " + out + " " + signed.length + " bytes");
    }
}
