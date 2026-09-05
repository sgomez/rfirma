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
 * <p><b>Cuatro entradas y ni una mas</b>: {@code autofirma_pades_presign},
 * {@code autofirma_pades_postsign}, {@code autofirma_filter_certificates} y
 * {@code autofirma_free_string}. <b>Ninguna firma</b>, y esa es la invariante:
 * la clave privada no entra al isolate (ADR-0001). Se instancia
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
 * filter   ok  {"ok":true,"selected":[0,2]}
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

    /**
     * Acota un listado de certificados con la expresion de filtro de la sede.
     *
     * <p><b>Sin estado y sin sello</b> (ADR-0016, ID-252): no abre sesion
     * trifasica ninguna, asi que no hay nada que atar entre dos llamadas. El
     * DER ya viaja en cada certificado.
     *
     * <p>La expresion cruza <b>literal</b> (ID-256): quien decide es el motor,
     * y la lista blanca de criterios de Rust decide <i>si se llama</i>, no
     * <i>que se aplica</i>.
     *
     * @param filterProperties las claves {@code filter=} / {@code filters=} /
     *                         {@code filters.N=} en formato
     *                         {@code java.util.Properties}.
     * @param certificatesB64  los certificados a acotar, Base64 del DER
     *                         separado por {@code ';'}, en su orden.
     * @return JSON con los indices que pasan. Propiedad del llamante: se libera
     *         con {@code autofirma_free_string}.
     */
    @CEntryPoint(name = "autofirma_filter_certificates")
    public static CCharPointer filterCertificates(
            final IsolateThread thread,
            final CCharPointer filterProperties,
            final CCharPointer certificatesB64) {
        try {
            final int[] selected = FilterBridge.select(
                    SessionStamp.parseParams(CTypeConversion.toJavaString(filterProperties)),
                    FilterBridge.parseCertificates(CTypeConversion.toJavaString(certificatesB64)));

            final StringBuilder json = new StringBuilder("{\"ok\":true,\"selected\":[");
            for (int i = 0; i < selected.length; i++) {
                if (i > 0) {
                    json.append(',');
                }
                json.append(selected[i]);
            }
            return toUnmanagedCString(json.append("]}").toString());
        }
        catch (final Throwable e) {
            return toUnmanagedCString(errorJson(e));
        }
    }

    /**
     * La clase de fallo que Rust distingue de un fallo cualquiera (ID-296).
     *
     * <p>Un PDF con firmas no registradas no es un error del puente: es una
     * situacion que la sede tiene que confirmar, y sin nombre propio aqui no se
     * puede distinguir del resto al otro lado de la frontera.
     */
    static final String UNREGISTERED_SIGNATURES_KIND = "pdfHasUnregisteredSignatures";

    /** La clase con la que se marca todo lo demas. */
    static final String GENERIC_FAILURE_KIND = "failed";

    /**
     * El nombre de la excepcion de AutoFirma que se distingue. Se compara por
     * nombre y no por {@code instanceof} para no obligar a que la clase este
     * enlazada en la imagen nativa por una rama de error.
     */
    private static final String UNREGISTERED_SIGNATURES_EXCEPTION =
            "es.gob.afirma.signers.pades.common.PdfHasUnregisteredSignaturesException";

    static String errorJson(final Throwable e) {
        final String message = e.getMessage() == null ? e.getClass().getName()
                : e.getClass().getName() + ": " + e.getMessage();
        final StringBuilder json = new StringBuilder("{\"ok\":false");
        field(json, "kind", kindOf(e));
        field(json, "error", message);
        return json.append('}').toString();
    }

    /**
     * Hasta donde se sigue la cadena de causas.
     *
     * {@code initCause} prohibe la autocausa, pero no un ciclo de longitud dos
     * ({@code A -> B -> A}), que si se puede construir pasando las causas por
     * constructor. Nada del puente construye eso hoy; el tope esta porque un
     * bucle sobre datos que vienen de AutoFirma conviene que tenga fondo, y un
     * ciclo aqui se llevaria el hilo del isolate entero.
     */
    private static final int MAX_CAUSE_DEPTH = 32;

    /**
     * La clase de fallo, mirando tambien las causas: AutoFirma envuelve sus
     * excepciones antes de que lleguen hasta aqui.
     */
    static String kindOf(final Throwable e) {
        Throwable cause = e;
        for (int depth = 0; cause != null && depth < MAX_CAUSE_DEPTH; depth++) {
            if (UNREGISTERED_SIGNATURES_EXCEPTION.equals(cause.getClass().getName())) {
                return UNREGISTERED_SIGNATURES_KIND;
            }
            cause = cause.getCause() == cause ? null : cause.getCause();
        }
        return GENERIC_FAILURE_KIND;
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
