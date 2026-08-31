package es.gob.afirma.nativebridge;

import java.nio.charset.StandardCharsets;
import java.util.Base64;

import org.graalvm.nativeimage.IsolateThread;
import org.graalvm.nativeimage.UnmanagedMemory;
import org.graalvm.nativeimage.c.function.CEntryPoint;
import org.graalvm.nativeimage.c.type.CCharPointer;
import org.graalvm.nativeimage.c.type.CTypeConversion;
import org.graalvm.word.PointerBase;

/**
 * La frontera FFI del puente: prefirma y postfirma PAdES vistas desde Rust.
 *
 * <p>Aqui no se decide nada. Esta clase convierte cadenas C a Java, delega en
 * {@link PadesBridge} y devuelve JSON; lo que hace la firma vive alli, donde se
 * puede probar sin construir la imagen nativa.
 *
 * <p><b>Tres entradas y ni una mas</b>: {@code autofirma_pades_presign},
 * {@code autofirma_pades_postsign} y {@code autofirma_free_string}. Se instancia
 * {@code PAdESTriPhasePreProcessor} directamente y NO {@code PreProcessorFactory},
 * que referencia los preprocesadores XAdES, FacturaE, ASiC y PKCS1 y haria
 * alcanzable todo el arbol de formatos dentro de la imagen.
 *
 * <h2>La memoria (ADR-0003, ID-11)</h2>
 *
 * Todo lo que sale por un valor de retorno se reserva <b>a mano</b> con
 * {@link UnmanagedMemory#malloc(int)} y lo libera <b>Rust</b> llamando a
 * {@code autofirma_free_string}. Nunca {@code CTypeConversion.toCString}: GraalVM
 * libera esa memoria al salir del bloque, asi que Rust leeria memoria ya
 * liberada y al liberarla el mismo provocaria un doble {@code free}. Es un fallo
 * silencioso —funciona en pruebas cortas y corrompe memoria bajo carga—, asi que
 * quien anada una entrada aqui devuelve con {@link #toUnmanagedCString(String)} o
 * no devuelve.
 *
 * <h2>El JSON</h2>
 *
 * <pre>
 * presign  ok  {"ok":true,"session":"&lt;xml&gt;","pre":"&lt;b64 DER&gt;","stamp":"&lt;b64&gt;"}
 * postsign ok  {"ok":true,"pdf":"&lt;b64&gt;"}
 * error        {"ok":false,"error":"&lt;clase&gt;: &lt;mensaje&gt;"}
 * </pre>
 *
 * {@code session} y {@code stamp} son <b>opacos</b>: Rust los transporta sin
 * interpretarlos y los devuelve tal cual a la postfirma (ADR-0016).
 */
public final class NativeBridge {

    private NativeBridge() { }

    static {
        // AWT headless antes de que ninguna ruta de firma visible toque java.awt.
        //
        // Ya no se toca java.library.path: al excluir afirma-ui-utils (ID-08) la
        // libreria es UN SOLO fichero y no hay auxiliares de AWT que localizar.
        // Volver a ponerlos "por si acaso" es lo que hace que un JPEG con perfil
        // ICC aborte el proceso en vez de dar un error recuperable (ID-09).
        System.setProperty("java.awt.headless", "true");
    }

    /**
     * Libera una cadena devuelta por este puente. Rust <b>tiene</b> que llamarla
     * por cada valor de retorno, incluidos los caminos de error.
     */
    @CEntryPoint(name = "autofirma_free_string")
    public static void freeString(final IsolateThread thread, final PointerBase pointer) {
        if (pointer.isNonNull()) {
            UnmanagedMemory.free(pointer);
        }
    }

    /**
     * Prefirma PAdES.
     *
     * @param pdfB64       PDF de entrada en Base64.
     * @param algorithm    p.ej. {@code SHA256withRSA}.
     * @param certChainB64 cadena de certificados en Base64, separados por {@code ';'}.
     * @param extraParams  extraParams en formato {@code java.util.Properties}
     *                     (lineas {@code clave=valor}).
     * @return JSON. Propiedad del llamante: se libera con {@code autofirma_free_string}.
     */
    @CEntryPoint(name = "autofirma_pades_presign")
    public static CCharPointer padesPreSign(
            final IsolateThread thread,
            final CCharPointer pdfB64,
            final CCharPointer algorithm,
            final CCharPointer certChainB64,
            final CCharPointer extraParams) {
        try {
            final PadesBridge.PreSignResult result = PadesBridge.preSign(
                    Base64.getDecoder().decode(CTypeConversion.toJavaString(pdfB64)),
                    CTypeConversion.toJavaString(algorithm),
                    PadesBridge.parseCertificates(CTypeConversion.toJavaString(certChainB64)),
                    SessionStamp.parseParams(CTypeConversion.toJavaString(extraParams)));

            final StringBuilder json = new StringBuilder("{\"ok\":true");
            field(json, "session", result.session());
            field(json, "pre", result.preSignB64());
            field(json, "stamp", result.stamp());
            return toUnmanagedCString(json.append('}').toString());
        }
        catch (final Throwable e) {
            return toUnmanagedCString(errorJson(e));
        }
    }

    /**
     * Postfirma PAdES: ensambla el PDF firmado.
     *
     * <p>No recibe ni algoritmo ni extraParams: los toma del sello, que es
     * justamente lo que impide que se desvien de la prefirma (ADR-0016).
     *
     * @param pdfB64       el MISMO PDF de entrada que recibio la prefirma, en Base64.
     * @param certChainB64 la MISMA cadena de certificados, Base64 separado por {@code ';'}.
     * @param stampB64     el sello de sesion que devolvio la prefirma, tal cual.
     * @param sessionXml   el {@code TriphaseData} de la prefirma, tal cual.
     * @param pkcs1B64     el PKCS#1 calculado por Rust sobre los atributos firmados.
     * @return JSON. Propiedad del llamante: se libera con {@code autofirma_free_string}.
     */
    @CEntryPoint(name = "autofirma_pades_postsign")
    public static CCharPointer padesPostSign(
            final IsolateThread thread,
            final CCharPointer pdfB64,
            final CCharPointer certChainB64,
            final CCharPointer stampB64,
            final CCharPointer sessionXml,
            final CCharPointer pkcs1B64) {
        try {
            final byte[] signed = PadesBridge.postSign(
                    Base64.getDecoder().decode(CTypeConversion.toJavaString(pdfB64)),
                    PadesBridge.parseCertificates(CTypeConversion.toJavaString(certChainB64)),
                    CTypeConversion.toJavaString(stampB64),
                    CTypeConversion.toJavaString(sessionXml),
                    CTypeConversion.toJavaString(pkcs1B64));

            final StringBuilder json = new StringBuilder("{\"ok\":true");
            field(json, "pdf", Base64.getEncoder().encodeToString(signed));
            return toUnmanagedCString(json.append('}').toString());
        }
        catch (final Throwable e) {
            return toUnmanagedCString(errorJson(e));
        }
    }

    static String errorJson(final Throwable e) {
        final String message = e.getMessage() == null ? e.getClass().getName()
                : e.getClass().getName() + ": " + e.getMessage();
        final StringBuilder json = new StringBuilder("{\"ok\":false");
        field(json, "error", message);
        return json.append('}').toString();
    }

    private static void field(final StringBuilder json, final String name, final String value) {
        json.append(",\"").append(name).append("\":");
        if (value == null) {
            json.append("null");
            return;
        }
        json.append('"');
        for (int i = 0; i < value.length(); i++) {
            final char c = value.charAt(i);
            switch (c) {
                case '"' -> json.append("\\\"");
                case '\\' -> json.append("\\\\");
                case '\n' -> json.append("\\n");
                case '\r' -> json.append("\\r");
                case '\t' -> json.append("\\t");
                default -> {
                    if (c < 0x20) {
                        json.append(String.format("\\u%04x", Integer.valueOf(c)));
                    }
                    else {
                        json.append(c);
                    }
                }
            }
        }
        json.append('"');
    }

    /**
     * Reserva la cadena en el C-heap. La libera Rust, no GraalVM: ver ADR-0003 y
     * el aviso de la cabecera de esta clase.
     */
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
