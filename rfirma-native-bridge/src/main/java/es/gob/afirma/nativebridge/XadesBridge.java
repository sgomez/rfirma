package es.gob.afirma.nativebridge;

import java.nio.charset.StandardCharsets;
import java.security.cert.X509Certificate;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.Properties;
import java.util.TimeZone;

import es.gob.afirma.core.signers.TriphaseData;
import es.gob.afirma.triphase.signer.processors.XAdESTriPhasePreProcessor;

/**
 * Las dos fases de la firma trifasica XAdES que corresponden a Java, en Java
 * puro: sin tipos de GraalVM y sin cadenas C.
 *
 * <p>Hermana de {@link CadesBridge}, con la misma division de trabajo:
 * {@link NativeBridge} es solo la frontera y aqui vive lo que decide algo, donde
 * una prueba de JUnit lo puede llamar sin construir la imagen nativa.
 *
 * <p><b>La fase 2 no esta aqui, y no va a estarlo (ADR-0001).</b> La clave
 * privada no entra nunca en el isolate de Java.
 *
 * <p>Atiende <b>solo {@code sign}</b> y solo la variante <b>Enveloping</b>, que
 * es la que el original toma por defecto. La cofirma, la contrafirma y las demas
 * variantes se rechazan nombrando lo que falta.
 *
 * <p>A diferencia de CAdES, la prefirma XAdES es una <b>firma completa hecha con
 * una clave temporal</b>: la sesion se lleva ese XML en {@code BASE} —sin las
 * partes comunes— y la postfirma lo reinyecta cambiando el {@code SignatureValue}
 * de mentira por el PKCS#1 real. El {@code BASE} es, entonces, el documento que
 * de verdad se firma, y por eso el sello lo cubre.
 */
public final class XadesBridge {

    /** Instante que elige el puente para atar el sello a la sesion. */
    private static final String PROPERTY_SIGN_TIME = "TIME";
    /** El {@code SignedInfo} canonicalizado de la prefirma. */
    private static final String PROPERTY_PRESIGN = "PRE";
    /** Donde deposita Rust el PKCS#1. */
    private static final String PROPERTY_PKCS1 = "PK1";
    /** La operacion que abrio la sesion, para que la postfirma no pueda cambiarla. */
    private static final String PROPERTY_OPERATION = "OP";
    /** El XML firmado con la clave temporal y sin las partes comunes. */
    private static final String PROPERTY_XML_BASE = "BASE";
    /** La codificacion con la que la postfirma descodifica el {@code BASE}. */
    private static final String PROPERTY_XML_ENCODING = "ENCODING";

    private static final String OPERATION_SIGN = "sign";
    private static final String OPERATION_COSIGN = "cosign";
    private static final String OPERATION_COUNTERSIGN = "countersign";

    private static final String PARAM_FORMAT = "format";
    private static final String FORMAT_XADES = "XAdES";
    private static final String FORMAT_ENVELOPING = "XAdES Enveloping";

    private XadesBridge() { }

    /** El {@code SignedInfo} de una de las firmas de la sesion, con su identificador. */
    public record PreSign(String id, String pre) { }

    /** Lo que la prefirma entrega a Rust. */
    public record PreSignResult(String session, List<PreSign> pres, String stamp) { }

    /** Lo que Rust devuelve por cada prefirma: el PKCS#1 de la fase 2. */
    public record SignatureValue(String id, String pkcs1B64) { }

    /**
     * Prefirma XAdES Enveloping.
     *
     * <p>Cada {@code pre} es el <b>{@code SignedInfo} canonicalizado</b>: Rust lo
     * hashea y lo firma igual que el bloque de CAdES, sin tratarlo distinto.
     *
     * @param document    XML a firmar.
     * @param algorithm   algoritmo de firma, p.ej. {@code SHA256withRSA}.
     * @param chain       cadena de certificados del firmante.
     * @param extraParams los extraParams enviados; la politica y {@code format} viajan
     *                    aqui sin traducir.
     * @param operation   {@code sign}: ninguna otra esta implementada todavia.
     */
    public static PreSignResult preSign(final byte[] document, final String algorithm,
            final X509Certificate[] chain, final Properties extraParams, final String operation)
            throws Exception {

        requireSignOperation(operation);
        final Properties effectiveParams = envelopingParams(extraParams);

        final TimeZone timeZone = TimeZone.getDefault();
        final String time = Long.toString(System.currentTimeMillis());
        final TriphaseData session = new XAdESTriPhasePreProcessor()
                .preProcessPreSign(document, algorithm, chain, effectiveParams, false);

        if (session.getSignsCount() != 1) {
            throw new IllegalStateException("una firma XAdES prefirma una sola vez, y la prefirma"
                    + " ha devuelto " + session.getSignsCount() + " firmas");
        }
        final List<PreSign> pres = new ArrayList<>();
        for (final TriphaseData.TriSign signConfig : session.getTriSigns()) {
            final String pre = signConfig.getProperty(PROPERTY_PRESIGN);
            if (pre == null) {
                throw new IllegalStateException("la prefirma XAdES no ha devuelto PRE");
            }
            pres.add(new PreSign(signConfig.getId(), pre));
        }

        final TriphaseData.TriSign first = session.getSign(0);
        final String xmlBase = first.getProperty(PROPERTY_XML_BASE);
        if (xmlBase == null) {
            throw new IllegalStateException("la prefirma XAdES no ha devuelto BASE");
        }

        // Igual que en CAdES, la sesion no trae TIME: lo anade el puente, y con el
        // la operacion, para que la postfirma tenga contra que comparar el sello.
        first.addProperty(PROPERTY_SIGN_TIME, time);
        first.addProperty(PROPERTY_OPERATION, OPERATION_SIGN);

        final SessionStamp stamp = SessionStamp
                .of(algorithm, time, timeZone, effectiveParams, document, chain, OPERATION_SIGN,
                        null)
                .withXmlBase(xmlBase, first.getProperty(PROPERTY_XML_ENCODING));

        return new PreSignResult(session.toString(), List.copyOf(pres), stamp.encode());
    }

    /**
     * Postfirma XAdES: sustituye el {@code SignatureValue} temporal por el PKCS#1
     * real y devuelve el XML firmado.
     *
     * <p>Los {@code extraParams}, el algoritmo y la operacion salen del <b>sello</b>,
     * no del llamante (ADR-0016). Lo que viaja aparte —la sesion trifasica, el
     * documento y la cadena de certificados— se compara contra el sello antes de
     * firmar, y en XAdES eso incluye el {@code BASE}.
     *
     * @param document   el MISMO XML que recibio la prefirma.
     * @param chain      la MISMA cadena de certificados.
     * @param stampB64   el sello que devolvio la prefirma, tal cual.
     * @param sessionXml el {@code TriphaseData} de la prefirma, tal cual.
     * @param pkcs1s     el PKCS#1 de la prefirma de la sesion, con su identificador.
     */
    public static byte[] postSign(final byte[] document, final X509Certificate[] chain,
            final String stampB64, final String sessionXml, final List<SignatureValue> pkcs1s)
            throws Exception {

        final SessionStamp stamp = SessionStamp.decode(stampB64);
        final TriphaseData session =
                TriphaseData.parser(sessionXml.getBytes(StandardCharsets.UTF_8));
        if (session.getSignsCount() != 1) {
            throw new IllegalStateException("una sesion trifasica de firma XAdES contiene una sola"
                    + " firma, y esta contiene " + session.getSignsCount());
        }
        final TriphaseData.TriSign first = session.getSign(0);

        final String sessionTime = first.getProperty(PROPERTY_SIGN_TIME);
        if (!stamp.matchesSessionTime(sessionTime)) {
            throw new SessionStampMismatchException(
                    "el sello de sesion no corresponde a esta sesion trifasica: TIME "
                            + stamp.time() + " en el sello frente a " + sessionTime
                            + " en la sesion. Firmar asi produciria un XML que no es el que se"
                            + " prefirmo, y sin dar ningun error.");
        }

        final String sessionOperation = first.getProperty(PROPERTY_OPERATION);
        if (!stamp.matchesOperation(sessionOperation, null)) {
            throw new SessionStampMismatchException(
                    "la operacion de la sesion trifasica no es la que se sello: "
                            + stamp.operation() + " en el sello frente a " + sessionOperation
                            + " en la sesion.");
        }

        if (!stamp.matchesDocument(document)) {
            throw new SessionStampMismatchException(
                    "el documento que recibe la postfirma no es el que se prefirmo: el sello"
                            + " lleva el SHA-256 " + stamp.documentDigest() + ". Firmar asi"
                            + " produciria una firma que no cubre estos datos, sin dar ningun"
                            + " error.");
        }

        if (!stamp.matchesChain(chain)) {
            throw new SessionStampMismatchException(
                    "la cadena de certificados que recibe la postfirma no es la que prefirmo: el"
                            + " sello lleva el SHA-256 " + stamp.chainDigest() + ". Firmar asi"
                            + " produciria un XML que dice estar firmado por quien no lo firmo,"
                            + " con la firma invalida y sin dar ningun error.");
        }

        final String xmlBase = first.getProperty(PROPERTY_XML_BASE);
        if (!stamp.matchesXmlBase(xmlBase, first.getProperty(PROPERTY_XML_ENCODING))) {
            // El BASE es lo que la postfirma reinyecta, asi que es el documento que de
            // verdad se firma: con otro sale un XML completo cuya firma cubre algo que
            // nadie ha visto, y no falla nada.
            throw new SessionStampMismatchException(
                    "el BASE de la sesion trifasica no es el que se prefirmo: el sello lleva el"
                            + " SHA-256 " + stamp.xmlBaseDigest() + ". Firmar asi produciria una"
                            + " firma sobre otro documento, sin dar ningun error.");
        }

        attachPkcs1(first, pkcs1s);

        return new XAdESTriPhasePreProcessor().preProcessPostSign(
                document, stamp.algorithm(), chain, stamp.extraParams(), session);
    }

    private static void attachPkcs1(final TriphaseData.TriSign signConfig,
            final List<SignatureValue> pkcs1s) {
        if (pkcs1s == null || pkcs1s.size() != 1) {
            throw new IllegalArgumentException("una firma XAdES necesita exactamente un PKCS#1, y"
                    + " han llegado " + (pkcs1s == null ? 0 : pkcs1s.size()));
        }
        final SignatureValue value = pkcs1s.get(0);
        if (value.pkcs1B64() == null || value.pkcs1B64().isBlank()) {
            throw new IllegalArgumentException(
                    "falta el PKCS#1 de la prefirma «" + value.id() + "»");
        }
        if (!signConfig.getId().equals(value.id())) {
            throw new IllegalArgumentException("el PKCS#1 «" + value.id() + "» no corresponde a la"
                    + " prefirma «" + signConfig.getId() + "» de esta sesion trifasica");
        }
        signConfig.addProperty(PROPERTY_PKCS1, value.pkcs1B64().trim());
    }

    private static void requireSignOperation(final String operation) {
        final String requested = operation == null ? "" : operation.trim().toLowerCase(Locale.ROOT);
        if (OPERATION_SIGN.equals(requested)) {
            return;
        }
        if (OPERATION_COSIGN.equals(requested) || OPERATION_COUNTERSIGN.equals(requested)) {
            throw new IllegalArgumentException("la operacion XAdES «" + requested + "» todavia no"
                    + " esta implementada en el puente: solo lo esta sign");
        }
        throw new IllegalArgumentException(
                "operacion XAdES desconocida: «" + operation + "»; se esperaba sign");
    }

    /**
     * Los {@code extraParams} con la variante fijada a Enveloping.
     *
     * <p>Se copian antes de tocarlos porque el {@code Properties} es del llamante,
     * y se fija {@code format} en vez de dejarlo al defecto del builder para que
     * el sello diga cual se firmo y no haya que deducirlo.
     */
    private static Properties envelopingParams(final Properties extraParams) {
        final Properties copy = new Properties();
        if (extraParams != null) {
            for (final String name : extraParams.stringPropertyNames()) {
                copy.setProperty(name, extraParams.getProperty(name));
            }
        }
        final String requested = copy.getProperty(PARAM_FORMAT);
        if (requested != null && !requested.isBlank()
                && !FORMAT_XADES.equalsIgnoreCase(requested.trim())
                && !FORMAT_ENVELOPING.equalsIgnoreCase(requested.trim())) {
            throw new IllegalArgumentException("la variante XAdES «" + requested.trim() + "»"
                    + " todavia no esta implementada en el puente: solo lo esta "
                    + FORMAT_ENVELOPING);
        }
        copy.setProperty(PARAM_FORMAT, FORMAT_ENVELOPING);
        return copy;
    }
}
