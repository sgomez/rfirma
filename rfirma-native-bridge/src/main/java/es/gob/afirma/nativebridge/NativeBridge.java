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
 * Bridge minimo de medicion: un unico punto de entrada de prefirma PAdES.
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
