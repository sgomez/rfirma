package es.gob.afirma.nativebridge;

import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Base64;
import java.util.List;
import java.util.function.BiFunction;
import java.util.function.Function;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

import org.graalvm.nativeimage.IsolateThread;
import org.graalvm.nativeimage.UnmanagedMemory;
import org.graalvm.nativeimage.c.function.CEntryPoint;
import org.graalvm.nativeimage.c.type.CCharPointer;
import org.graalvm.nativeimage.c.type.CTypeConversion;
import org.graalvm.word.PointerBase;

/**
 * La frontera FFI del puente: las prefirmas y postfirmas vistas desde Rust.
 *
 * <p>Aqui no se decide nada. Esta clase convierte cadenas C a Java, delega en
 * {@link PadesBridge}, en {@link CadesBridge} o en {@link XadesBridge} y devuelve
 * JSON; lo que hace la firma vive alli, donde se puede probar sin construir la
 * imagen nativa.
 *
 * <p><b>Diez entradas y ni una mas</b>: {@code autofirma_pades_presign},
 * {@code autofirma_pades_postsign}, {@code autofirma_cades_presign},
 * {@code autofirma_cades_postsign}, {@code autofirma_xades_presign},
 * {@code autofirma_xades_postsign}, {@code autofirma_filter_certificates},
 * {@code autofirma_expand_extra_params}, {@code autofirma_validate_signatures} y
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
 *              y en CAdES "pres":[{"id":"..","pre":"&lt;b64 DER&gt;"}] en vez de "pre"
 * postsign ok  {"ok":true,"pdf":"&lt;b64&gt;"}   y en CAdES {"ok":true,"signature":"&lt;b64&gt;"}
 * filter   ok  {"ok":true,"selected":[0,2]}
 * expand   ok  {"ok":true,"params":"&lt;bloque properties&gt;"}
 * validate ok  {"ok":true,"verdict":"valid"}
 *              {"ok":true,"verdict":"invalid","reason":"&lt;VALIDITY_ERROR&gt;"}
 *              {"ok":true,"verdict":"confirmationNeeded","param":"&lt;clave&gt;","text":"&lt;codigo&gt;"}
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
     * Prefirma CAdES.
     *
     * @param dataB64      datos a firmar en Base64.
     * @param algorithm    p.ej. {@code SHA256withRSA}.
     * @param certChainB64 cadena de certificados en Base64, separados por {@code ';'}.
     * @param extraParams  extraParams en formato {@code java.util.Properties}
     *                     (lineas {@code clave=valor}).
     * @param operation    {@code sign}, {@code cosign} o {@code countersign}.
     * @return JSON. Propiedad del llamante: se libera con {@code autofirma_free_string}.
     */
    @CEntryPoint(name = "autofirma_cades_presign")
    public static CCharPointer cadesPreSign(
            final IsolateThread thread,
            final CCharPointer dataB64,
            final CCharPointer algorithm,
            final CCharPointer certChainB64,
            final CCharPointer extraParams,
            final CCharPointer operation) {
        try {
            final CadesBridge.PreSignResult result = CadesBridge.preSign(
                    Base64.getDecoder().decode(CTypeConversion.toJavaString(dataB64)),
                    CTypeConversion.toJavaString(algorithm),
                    PadesBridge.parseCertificates(CTypeConversion.toJavaString(certChainB64)),
                    SessionStamp.parseParams(CTypeConversion.toJavaString(extraParams)),
                    CTypeConversion.toJavaString(operation));

            final StringBuilder json = new StringBuilder("{\"ok\":true");
            field(json, "session", result.session());
            pres(json, result.pres(), CadesBridge.PreSign::id,
                    CadesBridge.PreSign::pre);
            field(json, "stamp", result.stamp());
            return toUnmanagedCString(json.append('}').toString());
        }
        catch (final Throwable e) {
            return toUnmanagedCString(errorJson(e));
        }
    }

    private static <T> void pres(final StringBuilder json, final List<T> pres,
            final Function<T, String> id, final Function<T, String> value) {
        json.append(",\"pres\":[");
        for (int i = 0; i < pres.size(); i++) {
            if (i > 0) {
                json.append(',');
            }
            json.append('{');
            member(json, "id", id.apply(pres.get(i)));
            field(json, "pre", value.apply(pres.get(i)));
            json.append('}');
        }
        json.append(']');
    }

    /**
     * Postfirma CAdES: ensambla el CMS firmado.
     *
     * <p>No recibe ni algoritmo ni extraParams: los toma del sello (ADR-0016).
     *
     * @param dataB64      los MISMOS datos que recibio la prefirma, en Base64.
     * @param certChainB64 la MISMA cadena de certificados, Base64 separado por {@code ';'}.
     * @param stampB64     el sello de sesion que devolvio la prefirma, tal cual.
     * @param sessionXml   el {@code TriphaseData} de la prefirma, tal cual.
     * @param pkcs1Json    los PKCS#1 calculados por Rust, uno por prefirma:
     *                     {@code [{"id":"..","pk1":".."}]}.
     * @return JSON. Propiedad del llamante: se libera con {@code autofirma_free_string}.
     */
    @CEntryPoint(name = "autofirma_cades_postsign")
    public static CCharPointer cadesPostSign(
            final IsolateThread thread,
            final CCharPointer dataB64,
            final CCharPointer certChainB64,
            final CCharPointer stampB64,
            final CCharPointer sessionXml,
            final CCharPointer pkcs1Json) {
        try {
            final byte[] signature = CadesBridge.postSign(
                    Base64.getDecoder().decode(CTypeConversion.toJavaString(dataB64)),
                    PadesBridge.parseCertificates(CTypeConversion.toJavaString(certChainB64)),
                    CTypeConversion.toJavaString(stampB64),
                    CTypeConversion.toJavaString(sessionXml),
                    parsePkcs1List(CTypeConversion.toJavaString(pkcs1Json)));

            final StringBuilder json = new StringBuilder("{\"ok\":true");
            field(json, "signature", Base64.getEncoder().encodeToString(signature));
            return toUnmanagedCString(json.append('}').toString());
        }
        catch (final Throwable e) {
            return toUnmanagedCString(errorJson(e));
        }
    }

    /**
     * Prefirma XAdES en la variante que pida {@code extraParams.format}.
     *
     * @param xmlB64       XML de entrada en Base64.
     * @param algorithm    p.ej. {@code SHA256withRSA}.
     * @param certChainB64 cadena de certificados en Base64, separados por {@code ';'}.
     * @param extraParams  extraParams en formato {@code java.util.Properties}
     *                     (lineas {@code clave=valor}).
     * @param operation    {@code sign}, {@code cosign} o {@code countersign}.
     * @return JSON. Propiedad del llamante: se libera con {@code autofirma_free_string}.
     */
    @CEntryPoint(name = "autofirma_xades_presign")
    public static CCharPointer xadesPreSign(
            final IsolateThread thread,
            final CCharPointer xmlB64,
            final CCharPointer algorithm,
            final CCharPointer certChainB64,
            final CCharPointer extraParams,
            final CCharPointer operation) {
        try {
            final XadesBridge.PreSignResult result = XadesBridge.preSign(
                    Base64.getDecoder().decode(CTypeConversion.toJavaString(xmlB64)),
                    CTypeConversion.toJavaString(algorithm),
                    PadesBridge.parseCertificates(CTypeConversion.toJavaString(certChainB64)),
                    SessionStamp.parseParams(CTypeConversion.toJavaString(extraParams)),
                    CTypeConversion.toJavaString(operation));

            final StringBuilder json = new StringBuilder("{\"ok\":true");
            field(json, "session", result.session());
            pres(json, result.pres(), XadesBridge.PreSign::id, XadesBridge.PreSign::pre);
            field(json, "stamp", result.stamp());
            return toUnmanagedCString(json.append('}').toString());
        }
        catch (final Throwable e) {
            return toUnmanagedCString(errorJson(e));
        }
    }

    /**
     * Postfirma XAdES: devuelve el XML firmado.
     *
     * <p>No recibe ni algoritmo ni extraParams: los toma del sello (ADR-0016).
     *
     * @param xmlB64       el MISMO XML que recibio la prefirma, en Base64.
     * @param certChainB64 la MISMA cadena de certificados, Base64 separado por {@code ';'}.
     * @param stampB64     el sello de sesion que devolvio la prefirma, tal cual.
     * @param sessionXml   el {@code TriphaseData} de la prefirma, tal cual.
     * @param pkcs1Json    el PKCS#1 calculado por Rust: {@code [{"id":"..","pk1":".."}]}.
     * @return JSON. Propiedad del llamante: se libera con {@code autofirma_free_string}.
     */
    @CEntryPoint(name = "autofirma_xades_postsign")
    public static CCharPointer xadesPostSign(
            final IsolateThread thread,
            final CCharPointer xmlB64,
            final CCharPointer certChainB64,
            final CCharPointer stampB64,
            final CCharPointer sessionXml,
            final CCharPointer pkcs1Json) {
        try {
            final byte[] signature = XadesBridge.postSign(
                    Base64.getDecoder().decode(CTypeConversion.toJavaString(xmlB64)),
                    PadesBridge.parseCertificates(CTypeConversion.toJavaString(certChainB64)),
                    CTypeConversion.toJavaString(stampB64),
                    CTypeConversion.toJavaString(sessionXml),
                    parsePkcs1List(CTypeConversion.toJavaString(pkcs1Json),
                            XadesBridge.SignatureValue::new));

            final StringBuilder json = new StringBuilder("{\"ok\":true");
            field(json, "signature", Base64.getEncoder().encodeToString(signature));
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
     * Expande la politica de firma que declara la sede (ID-266).
     *
     * <p>La expansion es del original: {@code ExtraParamsProcessor} vive dentro
     * de {@code afirma-core} y sabe en que se convierte
     * {@code expPolicy=FirmaAGE}. Aqui solo se cruzan las cadenas.
     *
     * @param extraParams los {@code extraParams} de la sede, en formato
     *                    {@code java.util.Properties}.
     * @param format      el formato de firma, {@code PAdES}.
     * @return JSON con el bloque expandido. Propiedad del llamante: se libera
     *         con {@code autofirma_free_string}.
     */
    @CEntryPoint(name = "autofirma_expand_extra_params")
    public static CCharPointer expandExtraParams(
            final IsolateThread thread,
            final CCharPointer extraParams,
            final CCharPointer format) {
        try {
            final String expanded = ExtraParamsBridge.expand(
                    SessionStamp.parseParams(CTypeConversion.toJavaString(extraParams)),
                    CTypeConversion.toJavaString(format));

            final StringBuilder json = new StringBuilder("{\"ok\":true");
            field(json, "params", expanded);
            return toUnmanagedCString(json.append('}').toString());
        }
        catch (final Throwable e) {
            return toUnmanagedCString(errorJson(e));
        }
    }

    /**
     * Valida con el validador del original las firmas que ya trae un documento.
     *
     * @param documentB64 documento de entrada en Base64.
     * @param format      formato de firma, {@code PAdES}, {@code CAdES},
     *                    {@code XAdES *} o {@code FacturaE}.
     * @return JSON con el veredicto. Propiedad del llamante: se libera con
     *         {@code autofirma_free_string}.
     */
    @CEntryPoint(name = "autofirma_validate_signatures")
    public static CCharPointer validateSignatures(
            final IsolateThread thread,
            final CCharPointer documentB64,
            final CCharPointer format) {
        try {
            final ValidationBridge.Verdict verdict = ValidationBridge.validate(
                    Base64.getDecoder().decode(CTypeConversion.toJavaString(documentB64)),
                    CTypeConversion.toJavaString(format));

            final StringBuilder json = new StringBuilder("{\"ok\":true");
            field(json, "verdict", verdict.outcome());
            if (verdict.reason() != null) {
                field(json, "reason", verdict.reason());
            }
            if (verdict.param() != null) {
                field(json, "param", verdict.param());
                field(json, "text", verdict.text());
            }
            return toUnmanagedCString(json.append('}').toString());
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

    /**
     * La otra clase con nombre propio: la politica que la sede declara no se
     * puede aplicar al formato pedido (ID-266).
     *
     * <p>Sin nombre propio se colapsaria en «la firma no ha salido», y lo que
     * ha pasado es que la sede pidio una politica que no existe o que no case
     * con PAdES: eso tiene codigo propio en el catalogo publicado.
     */
    static final String INCOMPATIBLE_POLICY_KIND = "incompatiblePolicy";

    /** La clase con la que se marca todo lo demas. */
    static final String GENERIC_FAILURE_KIND = "failed";

    /**
     * El nombre de la excepcion de AutoFirma que se distingue. Se compara por
     * nombre y no por {@code instanceof} para no obligar a que la clase este
     * enlazada en la imagen nativa por una rama de error.
     */
    private static final String UNREGISTERED_SIGNATURES_EXCEPTION =
            "es.gob.afirma.signers.pades.common.PdfHasUnregisteredSignaturesException";

    /** Igual que la de arriba, y por lo mismo: se compara por nombre. */
    private static final String INCOMPATIBLE_POLICY_EXCEPTION =
            "es.gob.afirma.core.signers.ExtraParamsProcessor$IncompatiblePolicyException";

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
            if (INCOMPATIBLE_POLICY_EXCEPTION.equals(cause.getClass().getName())) {
                return INCOMPATIBLE_POLICY_KIND;
            }
            cause = cause.getCause() == cause ? null : cause.getCause();
        }
        return GENERIC_FAILURE_KIND;
    }

    private static final Pattern PKCS1_LIST = Pattern.compile(
            "\\s*\\[\\s*(\\{[^\\[\\]{}]*\\}(\\s*,\\s*\\{[^\\[\\]{}]*\\})*)?\\s*\\]\\s*");
    private static final Pattern PKCS1_ENTRY = Pattern.compile(
            "\\{\\s*\"(id|pk1)\"\\s*:\\s*\"([^\"\\\\]*)\"\\s*,"
                    + "\\s*\"(id|pk1)\"\\s*:\\s*\"([^\"\\\\]*)\"\\s*\\}");

    /**
     * Los PKCS#1 de la fase 2 tal y como los envia Rust:
     * {@code [{"id":"..","pk1":".."}]}, en cualquiera de los dos ordenes.
     *
     * <p>Vive junto a {@link #member}, que escribe el otro lado del mismo JSON.
     * Valida la cadena entera —no busca dentro de ella— y rechaza los escapes:
     * identificadores y PKCS#1 son Base64.
     */
    static List<CadesBridge.SignatureValue> parsePkcs1List(final String json) {
        return parsePkcs1List(json, CadesBridge.SignatureValue::new);
    }

    /** El mismo JSON, para el formato que lo pida: cada uno tiene su propio par. */
    static <T> List<T> parsePkcs1List(final String json,
            final BiFunction<String, String, T> value) {
        if (json == null || !PKCS1_LIST.matcher(json).matches()) {
            throw new IllegalArgumentException(
                    "el PKCS#1 de la fase 2 no llega como lista: se esperaba"
                            + " [{\"id\":\"..\",\"pk1\":\"..\"}] y nada mas");
        }
        final List<T> values = new ArrayList<>();
        final Matcher entry = PKCS1_ENTRY.matcher(json);
        while (entry.find()) {
            if (entry.group(1).equals(entry.group(3))) {
                throw new IllegalArgumentException(
                        "el PKCS#1 de la fase 2 repite el campo \u00ab" + entry.group(1)
                                + "\u00bb");
            }
            values.add(value.apply(
                    "id".equals(entry.group(1)) ? entry.group(2) : entry.group(4),
                    "pk1".equals(entry.group(1)) ? entry.group(2) : entry.group(4)));
        }
        if (values.isEmpty() || values.size() != countEntries(json)) {
            throw new IllegalArgumentException(
                    "falta el PKCS#1 de la fase 2: se esperaba"
                            + " [{\"id\":\"..\",\"pk1\":\"..\"}] en Base64, sin escapes");
        }
        return values;
    }

    private static int countEntries(final String json) {
        int entries = 0;
        for (int i = 0; i < json.length(); i++) {
            if (json.charAt(i) == '{') {
                entries++;
            }
        }
        return entries;
    }

    private static void field(final StringBuilder json, final String name, final String value) {
        json.append(',');
        member(json, name, value);
    }

    private static void member(final StringBuilder json, final String name, final String value) {
        json.append('"').append(name).append("\":");
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
