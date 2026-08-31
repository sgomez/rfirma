package es.gob.afirma.nativebridge;

import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.KeyStore;
import java.security.PrivateKey;
import java.security.cert.Certificate;
import java.security.cert.X509Certificate;
import java.util.ArrayList;
import java.util.Base64;
import java.util.Enumeration;
import java.util.List;

import com.aowagie.text.Document;
import com.aowagie.text.Paragraph;
import com.aowagie.text.pdf.PdfWriter;

/**
 * Material de las pruebas del puente.
 *
 * <p>El certificado sale del kit FNMT versionado en {@code testdata/fnmt/}: es
 * material publico por diseno, con la contrasena publicada por su emisor. El
 * certificado personal del titular no se usa en ningun punto del proyecto.
 *
 * <p>El PDF se fabrica aqui en vez de versionarlo: es una pagina, lo genera
 * iText —que ya esta en el classpath por afirma-crypto-pdf— y asi no hay un
 * binario mas que explicar en el repositorio.
 */
final class TestFixtures {

    /** Camino feliz del kit: RSA 2048, OCSP good, caduca el 2028-10-30. */
    private static final Path ACTIVE_P12 = Path.of("..", "testdata", "fnmt", "active-rsa.p12");
    /** El segundo certificado del kit. Aqui solo se usa por ser OTRO, no por estar revocado. */
    private static final Path REVOKED_P12 = Path.of("..", "testdata", "fnmt", "revoked-rsa.p12");
    private static final char[] PASSWORD = "1234".toCharArray();

    private TestFixtures() { }

    static byte[] samplePdf() throws Exception {
        final ByteArrayOutputStream out = new ByteArrayOutputStream();
        final Document document = new Document();
        PdfWriter.getInstance(document, out);
        document.open();
        document.add(new Paragraph("Documento de prueba de rfirma."));
        document.close();
        return out.toByteArray();
    }

    static KeyStore keyStore() throws Exception {
        return keyStore(ACTIVE_P12);
    }

    private static KeyStore keyStore(final Path p12) throws Exception {
        final KeyStore ks = KeyStore.getInstance("PKCS12");
        try (InputStream in = Files.newInputStream(p12)) {
            ks.load(in, PASSWORD);
        }
        return ks;
    }

    static String alias() throws Exception {
        return alias(keyStore());
    }

    private static String alias(final KeyStore ks) throws Exception {
        for (final Enumeration<String> aliases = ks.aliases(); aliases.hasMoreElements();) {
            final String alias = aliases.nextElement();
            if (ks.isKeyEntry(alias)) {
                return alias;
            }
        }
        throw new IllegalStateException("el kit FNMT no tiene ninguna entrada con clave privada");
    }

    static X509Certificate[] certificateChain() throws Exception {
        return certificateChain(keyStore());
    }

    /** Otra cadena distinta del kit, para las pruebas que necesitan una que no sea la del sello. */
    static X509Certificate[] otherCertificateChain() throws Exception {
        return certificateChain(keyStore(REVOKED_P12));
    }

    private static X509Certificate[] certificateChain(final KeyStore ks) throws Exception {
        final Certificate[] chain = ks.getCertificateChain(alias(ks));
        final List<X509Certificate> certs = new ArrayList<>();
        for (final Certificate cert : chain) {
            certs.add((X509Certificate) cert);
        }
        return certs.toArray(new X509Certificate[0]);
    }

    /** La cadena tal y como la recibe la frontera FFI: Base64 separado por ';'. */
    static String certificateChainB64() throws Exception {
        final StringBuilder sb = new StringBuilder();
        for (final X509Certificate cert : certificateChain()) {
            if (sb.length() > 0) {
                sb.append(';');
            }
            sb.append(Base64.getEncoder().encodeToString(cert.getEncoded()));
        }
        return sb.toString();
    }

    static PrivateKey privateKey() throws Exception {
        return (PrivateKey) keyStore().getKey(alias(), PASSWORD);
    }
}
