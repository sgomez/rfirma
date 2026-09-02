package probe;

import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.KeyStore;
import java.security.PrivateKey;
import java.security.cert.Certificate;
import java.util.Enumeration;
import java.util.List;
import java.util.Properties;

import es.gob.afirma.signers.pades.AOPDFSigner;
import es.gob.afirma.signers.pades.PdfUtil;

/** Sondeo #149: enumeracion de campos de firma vacios y firma sobre uno de ellos.
 *
 * <p>Uso:
 * <ul>
 *   <li>{@code Probe list &lt;pdf&gt;} — enumera los campos vacios con PdfUtil.</li>
 *   <li>{@code Probe sign &lt;pdf&gt; &lt;p12&gt; &lt;campo|-&gt; &lt;salida&gt;} — firma, con o
 *       sin {@code signatureField}, enviando SIEMPRE los cuatro parametros de
 *       posicion apuntando a otro sitio.</li>
 * </ul>
 */
public final class Probe {

    public static void main(final String[] args) throws Exception {
        if ("list".equals(args[0])) {
            list(Path.of(args[1]));
        } else {
            sign(Path.of(args[1]), Path.of(args[2]), args[3], Path.of(args[4]),
                 args.length > 5 ? args[5] : "");
        }
    }

    private static void list(final Path pdf) throws Exception {
        final List<PdfUtil.SignatureField> fields =
            PdfUtil.getPdfEmptySignatureFields(Files.readAllBytes(pdf));
        System.out.println("campos vacios: " + fields.size());
        for (final PdfUtil.SignatureField f : fields) {
            System.out.printf("  %-16s pagina=%d rect=[%d %d %d %d]%n",
                f.getName(), Integer.valueOf(f.getPage()),
                Integer.valueOf(f.getSignaturePositionOnPageLowerLeftX()),
                Integer.valueOf(f.getSignaturePositionOnPageLowerLeftY()),
                Integer.valueOf(f.getSignaturePositionOnPageUpperRightX()),
                Integer.valueOf(f.getSignaturePositionOnPageUpperRightY()));
        }
    }

    private static void sign(final Path in, final Path p12, final String field, final Path out,
            final String extra) throws Exception {
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
        // Posicion "senuelo": pagina 1, esquina distinta a la de cualquier campo.
        p.setProperty("signaturePage", "1");
        p.setProperty("signaturePositionOnPageLowerLeftX", "40");
        p.setProperty("signaturePositionOnPageLowerLeftY", "40");
        p.setProperty("signaturePositionOnPageUpperRightX", "160");
        p.setProperty("signaturePositionOnPageUpperRightY", "90");
        p.setProperty("layer2Text", "Firmado por: sondeo rfirma 149");
        p.setProperty("signReason", "Sondeo #149");
        if (!"-".equals(field)) {
            p.setProperty("signatureField", field);
        }
        // Parametros adicionales sueltos: "clave=valor;clave=valor".
        // Un valor vacio ("clave=") borra la clave, para probar su ausencia.
        for (final String kv : extra.split(";")) {
            final int eq = kv.indexOf('=');
            if (eq <= 0) { continue; }
            final String k = kv.substring(0, eq), v = kv.substring(eq + 1);
            if (v.isEmpty()) { p.remove(k); } else { p.setProperty(k, v); }
        }

        final byte[] signed = new AOPDFSigner()
            .sign(Files.readAllBytes(in), "SHA256withRSA", key, chain, p);
        Files.write(out, signed);
        System.out.println("OK " + out + " " + signed.length + " bytes  signatureField=" + field + " extra=" + extra);
    }
}
