package es.gob.afirma.xadesspike;

import java.io.FileOutputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.KeyStore;
import java.security.PrivateKey;
import java.security.Signature;
import java.security.cert.X509Certificate;
import java.util.Properties;

import es.gob.afirma.core.signers.TriphaseData;
import es.gob.afirma.triphase.signer.processors.XAdESTriPhasePreProcessor;

/**
 * Banco de un solo uso para el issue #534: prefirma y postfirma XAdES
 * Enveloping en un unico proceso, sin separar la fase 2 (aqui firma con la
 * clave privada en el mismo isolate: no es el diseno de produccion, ADR-0001,
 * solo la forma mas corta de comprobar que native-image compila y ejecuta el
 * preprocesador).
 *
 * <p>Uso: {@code Main <p12> <alias> <pass> <algoritmo> <xml-entrada>
 * <xml-salida>}
 */
public final class Main {

    private Main() { }

    public static void main(final String[] args) throws Exception {
        if (args.length != 6) {
            System.err.println(
                    "Uso: Main <p12> <alias> <pass> <algoritmo> <xml-entrada> <xml-salida>");
            System.exit(2);
        }
        final String p12Path = args[0];
        final String alias = args[1];
        final String pass = args[2];
        final String algorithm = args[3];
        final Path xmlIn = Path.of(args[4]);
        final Path xmlOut = Path.of(args[5]);

        final KeyStore ks = KeyStore.getInstance("PKCS12");
        try (var in = Files.newInputStream(Path.of(p12Path))) {
            ks.load(in, pass.toCharArray());
        }
        final PrivateKey key = (PrivateKey) ks.getKey(alias, pass.toCharArray());
        final X509Certificate cert = (X509Certificate) ks.getCertificate(alias);

        final byte[] data = Files.readAllBytes(xmlIn);
        final Properties extraParams = new Properties();
        extraParams.setProperty("format", "XAdES Enveloping");

        final XAdESTriPhasePreProcessor preProcessor = new XAdESTriPhasePreProcessor();

        final long t0 = System.nanoTime();
        final TriphaseData session = preProcessor.preProcessPreSign(
                data, algorithm, new X509Certificate[] { cert }, extraParams, false);
        final long t1 = System.nanoTime();

        if (session.getSignsCount() < 1) {
            throw new IllegalStateException("la prefirma XAdES no ha devuelto ninguna firma");
        }
        final TriphaseData.TriSign signConfig = session.getSign(0);
        final String preSignB64 = signConfig.getProperty("PRE");
        if (preSignB64 == null) {
            throw new IllegalStateException("la prefirma XAdES no ha devuelto PRE");
        }
        System.out.println("PK1_DECODED=" + signConfig.getProperty("PK1_DECODED"));

        final byte[] preSign = java.util.Base64.getDecoder().decode(preSignB64);
        final Signature signer = Signature.getInstance(algorithm);
        signer.initSign(key);
        signer.update(preSign);
        final byte[] pk1 = signer.sign();
        signConfig.addProperty("PK1", java.util.Base64.getEncoder().encodeToString(pk1));

        final long t2 = System.nanoTime();
        final byte[] signed = preProcessor.preProcessPostSign(
                data, algorithm, new X509Certificate[] { cert }, extraParams, session);
        final long t3 = System.nanoTime();

        try (FileOutputStream out = new FileOutputStream(xmlOut.toFile())) {
            out.write(signed);
        }

        System.out.println("PRESIGN_NS=" + (t1 - t0));
        System.out.println("POSTSIGN_NS=" + (t3 - t2));
        System.out.println("PRESIGN OK (" + preSign.length + " bytes) -> PRE");
        System.out.println("POSTSIGN OK (" + signed.length + " bytes) -> " + xmlOut);
    }
}
