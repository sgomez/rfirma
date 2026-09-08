import java.io.FileInputStream;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.KeyStore;
import java.util.Properties;

import es.gob.afirma.core.signers.AOSignConstants;
import es.gob.afirma.core.signers.CounterSignTarget;
import es.gob.afirma.signers.cades.AOCAdESSigner;
import es.gob.afirma.signers.xades.AOFacturaESigner;
import es.gob.afirma.signers.xades.AOXAdESSigner;

/** Firma con los firmadores monofasicos del 1.9.2 para producir testdata/reference/. */
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
