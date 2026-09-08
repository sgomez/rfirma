package es.gob.afirma.xadesspike;

import java.nio.charset.StandardCharsets;
import java.security.cert.X509Certificate;
import java.util.Base64;
import java.util.Properties;

import org.graalvm.nativeimage.IsolateThread;
import org.graalvm.nativeimage.UnmanagedMemory;
import org.graalvm.nativeimage.c.function.CEntryPoint;
import org.graalvm.nativeimage.c.type.CCharPointer;
import org.graalvm.nativeimage.c.type.CTypeConversion;

import es.gob.afirma.core.signers.TriphaseData;
import es.gob.afirma.triphase.signer.processors.XAdESTriPhasePreProcessor;

/**
 * Un {@code @CEntryPoint} minimo, solo para que {@code --shared} tenga algo
 * que analizar y el tamano del {@code .so} sea comparable al de
 * {@code rfirma-native-bridge} (mismo patron que {@code NativeBridge}, ADR-0003).
 * No es el diseno de produccion: aqui la prefirma solo se ejercita para que
 * {@code native-image} incluya el arbol de alcanzabilidad de XAdES.
 */
public final class EntryPoint {

    private EntryPoint() { }

    @CEntryPoint(name = "xades_spike_presign")
    public static CCharPointer preSign(
            final IsolateThread thread,
            final CCharPointer dataB64,
            final CCharPointer algorithm,
            final CCharPointer certB64) {
        try {
            final byte[] data = Base64.getDecoder().decode(CTypeConversion.toJavaString(dataB64));
            final String alg = CTypeConversion.toJavaString(algorithm);
            final java.security.cert.CertificateFactory cf =
                    java.security.cert.CertificateFactory.getInstance("X.509");
            final X509Certificate cert = (X509Certificate) cf.generateCertificate(
                    new java.io.ByteArrayInputStream(
                            Base64.getDecoder().decode(CTypeConversion.toJavaString(certB64))));

            final Properties extraParams = new Properties();
            extraParams.setProperty("format", "XAdES Enveloping");

            final TriphaseData session = new XAdESTriPhasePreProcessor().preProcessPreSign(
                    data, alg, new X509Certificate[] { cert }, extraParams, false);

            return toUnmanagedCString(session.toString());
        }
        catch (final Throwable e) {
            return toUnmanagedCString("{\"ok\":false,\"error\":\"" + e + "\"}");
        }
    }

    private static CCharPointer toUnmanagedCString(final String s) {
        final byte[] bytes = s.getBytes(StandardCharsets.UTF_8);
        final CCharPointer p = UnmanagedMemory.malloc(bytes.length + 1);
        for (int i = 0; i < bytes.length; i++) {
            p.write(i, bytes[i]);
        }
        p.write(bytes.length, (byte) 0);
        return p;
    }
}
