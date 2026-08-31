package es.gob.afirma.nativebridge;

import java.io.ByteArrayInputStream;
import java.nio.charset.StandardCharsets;
import java.security.cert.CertificateFactory;
import java.security.cert.X509Certificate;
import java.util.ArrayList;
import java.util.Base64;
import java.util.List;
import java.util.Properties;

import org.graalvm.nativeimage.IsolateThread;
import org.graalvm.nativeimage.UnmanagedMemory;
import org.graalvm.nativeimage.c.function.CEntryPoint;
import org.graalvm.nativeimage.c.type.CCharPointer;
import org.graalvm.nativeimage.c.type.CTypeConversion;
import org.graalvm.word.PointerBase;

import es.gob.afirma.core.signers.TriphaseData;
import es.gob.afirma.triphase.signer.processors.PAdESTriPhasePreProcessor;

/**
 * Bridge minimo de medicion: prefirma y postfirma PAdES.
 *
 * Deliberadamente SIN JSON y SIN PreProcessorFactory: la factoria referencia
 * los preprocesadores XAdES, FacturaE, ASiC y PKCS1, y usarla haria alcanzable
 * todo el arbol de formatos. Aqui se instancia PAdESTriPhasePreProcessor
 * directamente para medir solo el coste de PAdES.
 */
public final class NativeBridge {

    private NativeBridge() { }

    static {
        // Medicion ticket #2: forzar AWT headless antes de que cualquier
        // ruta de firma visible toque java.awt.
        System.setProperty("java.awt.headless", "true");
        // Permite apuntar java.library.path al directorio donde Rust extraiga
        // los .so auxiliares del JDK (Substrate ignora LD_LIBRARY_PATH aqui).
        final String libDir = System.getenv("RFIRMA_LIB_DIR");
        if (libDir != null && !libDir.isEmpty()) {
            System.setProperty("java.library.path", libDir);
        }
    }

    @CEntryPoint(name = "rfirma_free_string")
    public static void freeString(final IsolateThread thread, final PointerBase pointer) {
        if (pointer.isNonNull()) {
            UnmanagedMemory.free(pointer);
        }
    }

    /**
     * Prefirma PAdES.
     *
     * @param pdfB64        PDF de entrada en Base64.
     * @param algorithm     p.ej. "SHA256withRSA".
     * @param certChainB64  cadena de certificados en Base64, separados por ';'.
     * @param extraParams   extraParams en formato java.util.Properties (lineas "clave=valor").
     * @return XML del TriphaseData, o "ERROR:<mensaje>". Propiedad del llamante:
     *         debe liberarse con rfirma_free_string.
     */
    @CEntryPoint(name = "rfirma_pades_presign")
    public static CCharPointer padesPreSign(
            final IsolateThread thread,
            final CCharPointer pdfB64,
            final CCharPointer algorithm,
            final CCharPointer certChainB64,
            final CCharPointer extraParams) {
        try {
            final byte[] pdf = Base64.getDecoder().decode(CTypeConversion.toJavaString(pdfB64));
            final X509Certificate[] chain = parseCertificates(CTypeConversion.toJavaString(certChainB64));

            final Properties params = new Properties();
            final String rawParams = CTypeConversion.toJavaString(extraParams);
            if (rawParams != null && !rawParams.isEmpty()) {
                params.load(new ByteArrayInputStream(rawParams.getBytes(StandardCharsets.UTF_8)));
            }

            final TriphaseData td = new PAdESTriPhasePreProcessor().preProcessPreSign(
                    pdf,
                    CTypeConversion.toJavaString(algorithm),
                    chain,
                    params,
                    false);

            return toCStringUnmanaged(td.toString());
        }
        catch (final Throwable e) {
            return toCStringUnmanaged("ERROR:" + e.getClass().getName() + ": " + e.getMessage());
        }
    }

    /**
     * Postfirma PAdES: ensambla el PDF firmado.
     *
     * Medicion ticket #13. Los extraParams y el instante de firma deben ser
     * exactamente los mismos que en la prefirma (el TIME viaja dentro del
     * propio TriphaseData); la postfirma regenera el PDF entero.
     *
     * @param pdfB64        el MISMO PDF de entrada que recibio la prefirma, en Base64.
     * @param algorithm     el MISMO algoritmo que en la prefirma.
     * @param certChainB64  la MISMA cadena de certificados, Base64 separado por ';'.
     * @param extraParams   los MISMOS extraParams que en la prefirma.
     * @param triphaseXml   el XML del TriphaseData de la prefirma, con el campo PK1 anadido.
     * @return PDF firmado en Base64, o "ERROR:&lt;mensaje&gt;". Propiedad del llamante:
     *         debe liberarse con rfirma_free_string.
     */
    @CEntryPoint(name = "rfirma_pades_postsign")
    public static CCharPointer padesPostSign(
            final IsolateThread thread,
            final CCharPointer pdfB64,
            final CCharPointer algorithm,
            final CCharPointer certChainB64,
            final CCharPointer extraParams,
            final CCharPointer triphaseXml) {
        try {
            final byte[] signedPdf = postSign(
                    CTypeConversion.toJavaString(pdfB64),
                    CTypeConversion.toJavaString(algorithm),
                    CTypeConversion.toJavaString(certChainB64),
                    CTypeConversion.toJavaString(extraParams),
                    CTypeConversion.toJavaString(triphaseXml));
            return toCStringUnmanaged(Base64.getEncoder().encodeToString(signedPdf));
        }
        catch (final Throwable e) {
            return toCStringUnmanaged("ERROR:" + e.getClass().getName() + ": " + e.getMessage());
        }
    }

    /** Nucleo compartido por el CEntryPoint nativo y el main de control en JVM. */
    static byte[] postSign(final String pdfB64, final String algorithm,
            final String certChainB64, final String rawParams,
            final String triphaseXml) throws Exception {
        final byte[] pdf = Base64.getDecoder().decode(pdfB64);
        final X509Certificate[] chain = parseCertificates(certChainB64);
        final Properties params = loadParams(rawParams);
        final TriphaseData session = TriphaseData.parser(triphaseXml.getBytes(StandardCharsets.UTF_8));

        return new PAdESTriPhasePreProcessor().preProcessPostSign(
                pdf, algorithm, chain, params, session);
    }

    /** Nucleo compartido por el CEntryPoint nativo y el main de control en JVM. */
    static TriphaseData preSign(final String pdfB64, final String algorithm,
            final String certChainB64, final String rawParams) throws Exception {
        return new PAdESTriPhasePreProcessor().preProcessPreSign(
                Base64.getDecoder().decode(pdfB64),
                algorithm,
                parseCertificates(certChainB64),
                loadParams(rawParams),
                false);
    }

    private static Properties loadParams(final String rawParams) throws Exception {
        final Properties params = new Properties();
        if (rawParams != null && !rawParams.isEmpty()) {
            params.load(new ByteArrayInputStream(rawParams.getBytes(StandardCharsets.UTF_8)));
        }
        return params;
    }

    /**
     * Control en JVM normal, para distinguir un fallo de native-image de un
     * fallo de AutoFirma o del montaje de la prueba.
     *
     * uso: presign  &lt;pdf.b64&gt; &lt;cert.b64&gt; [extra.properties]
     *      postsign &lt;pdf.b64&gt; &lt;cert.b64&gt; &lt;triphase.xml&gt; [extra.properties]
     */
    public static void main(final String[] args) throws Exception {
        final String pdfB64 = java.nio.file.Files.readString(java.nio.file.Path.of(args[1])).trim();
        final String certB64 = java.nio.file.Files.readString(java.nio.file.Path.of(args[2])).trim();
        if ("presign".equals(args[0])) {
            final String extra = args.length > 3
                    ? java.nio.file.Files.readString(java.nio.file.Path.of(args[3])) : "";
            System.out.println(preSign(pdfB64, "SHA256withRSA", certB64, extra).toString());
        }
        else if ("postsign".equals(args[0])) {
            final String xml = java.nio.file.Files.readString(java.nio.file.Path.of(args[3]));
            final String extra = args.length > 4
                    ? java.nio.file.Files.readString(java.nio.file.Path.of(args[4])) : "";
            final byte[] out = postSign(pdfB64, "SHA256withRSA", certB64, extra, xml);
            java.nio.file.Files.write(java.nio.file.Path.of("jvm-postsign.pdf"), out);
            System.out.println("POSTSIGN OK (" + out.length + " bytes) -> jvm-postsign.pdf");
        }
        else {
            throw new IllegalArgumentException("modo desconocido: " + args[0]);
        }
    }

    private static X509Certificate[] parseCertificates(final String chainB64) throws Exception {
        final CertificateFactory cf = CertificateFactory.getInstance("X.509");
        final List<X509Certificate> certs = new ArrayList<>();
        for (final String b64 : chainB64.split(";")) {
            if (b64.isBlank()) {
                continue;
            }
            certs.add((X509Certificate) cf.generateCertificate(
                    new ByteArrayInputStream(Base64.getDecoder().decode(b64.trim()))));
        }
        return certs.toArray(new X509Certificate[0]);
    }

    private static CCharPointer toCStringUnmanaged(final String s) {
        final byte[] bytes = s.getBytes(StandardCharsets.UTF_8);
        final CCharPointer p = UnmanagedMemory.malloc(bytes.length + 1);
        for (int i = 0; i < bytes.length; i++) {
            p.write(i, bytes[i]);
        }
        p.write(bytes.length, (byte) 0);
        return p;
    }
}
