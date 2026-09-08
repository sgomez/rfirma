package es.gob.afirma.nativebridge;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.cert.X509Certificate;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Locale;
import java.util.Properties;
import java.util.Set;
import java.util.TimeZone;

import es.gob.afirma.core.signers.AOSignConstants;
import es.gob.afirma.core.signers.CounterSignTarget;
import es.gob.afirma.core.signers.TriphaseData;
import es.gob.afirma.core.signers.asic.ASiCUtil;
import es.gob.afirma.signers.xades.XAdESConstants;
import es.gob.afirma.signers.xades.asic.AOXAdESASiCSSigner;
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
 * <p>Las tres operaciones —{@code sign}, {@code cosign} y {@code countersign}—
 * entran por los mismos dos metodos. Solo {@code sign} elige variante
 * —Enveloping, Detached, Enveloped o ASiC-S—: cofirmar y contrafirmar operan
 * sobre la estructura del XML que reciben, sin volver a elegirla.
 *
 * <p>A diferencia de CAdES, la prefirma XAdES es una <b>firma completa hecha con
 * una clave temporal</b>: la sesion se lleva ese XML en {@code BASE} —sin las
 * partes comunes— y la postfirma lo reinyecta cambiando el {@code SignatureValue}
 * de mentira por el PKCS#1 real. El {@code BASE} es, entonces, el documento que
 * de verdad se firma, y por eso el sello lo cubre. Una contrafirma prefirma
 * <b>una hoja o mas</b>, pero el {@code BASE} solo viaja en la primera prefirma
 * de la sesion (una sola sustitucion basta para todas).
 *
 * <p>El identificador de cada {@link PreSign} es su <b>posicion</b> en la sesion,
 * no el {@code Id} que trae el {@code TriSign}: en una contrafirma de varias
 * hojas el original puede repetir el mismo {@code Id} en mas de una, porque solo
 * lo usa para localizar el hueco a sustituir por indice.
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
    /** El objetivo de la contrafirma, por lo mismo. */
    private static final String PROPERTY_TARGET = "TARGET";
    /** El XML firmado con la clave temporal y sin las partes comunes. */
    private static final String PROPERTY_XML_BASE = "BASE";
    /** La codificacion con la que la postfirma descodifica el {@code BASE}. */
    private static final String PROPERTY_XML_ENCODING = "ENCODING";

    private static final String OPERATION_SIGN = "sign";
    private static final String OPERATION_COSIGN = "cosign";
    private static final String OPERATION_COUNTERSIGN = "countersign";
    private static final Set<String> OPERATIONS =
            Set.of(OPERATION_SIGN, OPERATION_COSIGN, OPERATION_COUNTERSIGN);

    private static final String PARAM_FORMAT = "format";
    private static final String FORMAT_XADES = "XAdES";
    private static final String FORMAT_ENVELOPING = "XAdES Enveloping";
    private static final String FORMAT_DETACHED = "XAdES Detached";
    private static final String FORMAT_ENVELOPED = "XAdES Enveloped";
    private static final String FORMAT_ASIC_S = "XAdES-ASiC-S";
    private static final String FORMAT_EXTERNALLY_DETACHED = "XAdES Externally Detached";

    private static final String PARAM_KEEP_KEYINFO_UNSIGNED = "keepKeyInfoUnsigned";
    private static final String PARAM_PRECALCULATED_HASH = "precalculatedHashAlgorithm";
    private static final String PARAM_REFERENCES_DIGEST = "referencesDigestMethod";
    private static final String PARAM_ASICS_FILENAME = "asicsFilename";

    private static final String PARAM_TARGET = "target";
    private static final String TARGET_TREE = "tree";
    private static final String TARGET_LEAFS = "leafs";

    private XadesBridge() { }

    /** El {@code SignedInfo} de una de las firmas de la sesion, con su posicion. */
    public record PreSign(String id, String pre) { }

    /** Lo que la prefirma entrega a Rust. */
    public record PreSignResult(String session, List<PreSign> pres, String stamp) { }

    /** Lo que Rust devuelve por cada prefirma: el PKCS#1 de la fase 2. */
    public record SignatureValue(String id, String pkcs1B64) { }

    /**
     * Prefirma XAdES: {@code sign} en la variante que pida {@code extraParams.format},
     * {@code cosign} o {@code countersign} —con {@code extraParams.target}— sobre un
     * XML ya firmado.
     *
     * <p>Cada {@code pre} es el <b>{@code SignedInfo} canonicalizado</b>: Rust lo
     * hashea y lo firma igual que el bloque de CAdES, sin tratarlo distinto.
     *
     * @param document    XML a firmar, o la firma XAdES a cofirmar o contrafirmar.
     * @param algorithm   algoritmo de firma, p.ej. {@code SHA256withRSA}.
     * @param chain       cadena de certificados del firmante.
     * @param extraParams los extraParams enviados; la politica, {@code format} y
     *                    {@code target} viajan aqui sin traducir.
     * @param operation   {@code sign}, {@code cosign} o {@code countersign}.
     */
    public static PreSignResult preSign(final byte[] document, final String algorithm,
            final X509Certificate[] chain, final Properties extraParams, final String operation)
            throws Exception {

        final String requested = requireKnownOperation(operation);
        final String target =
                OPERATION_COUNTERSIGN.equals(requested) ? counterSignTarget(extraParams) : null;
        final Properties effectiveParams =
                OPERATION_SIGN.equals(requested) ? variantParams(extraParams) : copyOf(extraParams);
        if (target != null) {
            effectiveParams.setProperty(PARAM_TARGET, target);
        }

        final TimeZone timeZone = TimeZone.getDefault();
        final String time = Long.toString(System.currentTimeMillis());
        final XAdESTriPhasePreProcessor processor = new XAdESTriPhasePreProcessor();
        final TriphaseData session = switch (requested) {
            case OPERATION_COSIGN -> processor.preProcessPreCoSign(
                    document, algorithm, chain, effectiveParams, false);
            case OPERATION_COUNTERSIGN -> processor.preProcessPreCounterSign(
                    document, algorithm, chain, effectiveParams,
                    CounterSignTarget.getTarget(target), false);
            default -> processor.preProcessPreSign(
                    signerDocument(effectiveParams, document), algorithm, chain,
                    signerParams(effectiveParams, document), false);
        };

        if (session.getSignsCount() < 1) {
            throw new IllegalStateException("la prefirma XAdES no ha devuelto ninguna firma");
        }
        final List<TriphaseData.TriSign> signs = session.getTriSigns();
        final List<PreSign> pres = new ArrayList<>();
        for (int i = 0; i < signs.size(); i++) {
            final String pre = signs.get(i).getProperty(PROPERTY_PRESIGN);
            if (pre == null) {
                throw new IllegalStateException("la prefirma XAdES no ha devuelto PRE");
            }
            pres.add(new PreSign(String.valueOf(i), pre));
        }

        final TriphaseData.TriSign first = session.getSign(0);
        final String xmlBase = first.getProperty(PROPERTY_XML_BASE);
        if (xmlBase == null) {
            throw new IllegalStateException("la prefirma XAdES no ha devuelto BASE");
        }

        // Igual que en CAdES, la sesion no trae TIME: lo anade el puente, y con el
        // la operacion, para que la postfirma tenga contra que comparar el sello.
        first.addProperty(PROPERTY_SIGN_TIME, time);
        first.addProperty(PROPERTY_OPERATION, requested);
        if (target != null) {
            first.addProperty(PROPERTY_TARGET, target);
        }

        final SessionStamp stamp = SessionStamp
                .of(algorithm, time, timeZone, effectiveParams, document, chain, requested, target)
                .withXmlBase(xmlBase, first.getProperty(PROPERTY_XML_ENCODING));

        return new PreSignResult(session.toString(), List.copyOf(pres), stamp.encode());
    }

    /**
     * Postfirma XAdES: sustituye el {@code SignatureValue} temporal por el PKCS#1
     * real y devuelve el XML firmado.
     *
     * <p>Los {@code extraParams}, el algoritmo, la operacion y el objetivo de la
     * contrafirma salen del <b>sello</b>, no del llamante (ADR-0016). Lo que viaja
     * aparte —la sesion trifasica, el documento y la cadena de certificados— se
     * compara contra el sello antes de firmar, y en XAdES eso incluye el {@code BASE}.
     *
     * @param document   el MISMO XML que recibio la prefirma.
     * @param chain      la MISMA cadena de certificados.
     * @param stampB64   el sello que devolvio la prefirma, tal cual.
     * @param sessionXml el {@code TriphaseData} de la prefirma, tal cual.
     * @param pkcs1s     un PKCS#1 por cada prefirma de la sesion, con su posicion.
     */
    public static byte[] postSign(final byte[] document, final X509Certificate[] chain,
            final String stampB64, final String sessionXml, final List<SignatureValue> pkcs1s)
            throws Exception {

        final SessionStamp stamp = SessionStamp.decode(stampB64);
        final TriphaseData session =
                TriphaseData.parser(sessionXml.getBytes(StandardCharsets.UTF_8));
        if (session.getSignsCount() < 1) {
            throw new IllegalStateException("la sesion trifasica no contiene ninguna firma");
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
        final String sessionTarget = first.getProperty(PROPERTY_TARGET);
        if (!stamp.matchesOperation(sessionOperation, sessionTarget)) {
            throw new SessionStampMismatchException(
                    "la operacion de la sesion trifasica no es la que se sello: "
                            + describe(stamp.operation(), stamp.target()) + " en el sello frente a "
                            + describe(sessionOperation, sessionTarget) + " en la sesion. Firmar"
                            + " asi produciria una firma de otra operacion distinta de la que se"
                            + " prefirmo, y sin dar ningun error.");
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
            throw new SessionStampMismatchException(
                    "el BASE de la sesion trifasica, o su ENCODING, no es el que se prefirmo:"
                            + " el sello lleva del BASE el SHA-256 " + stamp.xmlBaseDigest()
                            + ". Firmar asi produciria una firma sobre otro documento, sin dar"
                            + " ningun error.");
        }

        attachPkcs1(session, pkcs1s);

        final Properties effectiveParams = stamp.extraParams();
        final XAdESTriPhasePreProcessor processor = new XAdESTriPhasePreProcessor();
        final byte[] signature = switch (requireKnownOperation(stamp.operation())) {
            case OPERATION_COSIGN -> processor.preProcessPostCoSign(
                    document, stamp.algorithm(), chain, effectiveParams, session);
            case OPERATION_COUNTERSIGN -> processor.preProcessPostCounterSign(
                    document, stamp.algorithm(), chain, effectiveParams, session,
                    CounterSignTarget.getTarget(stamp.target()));
            default -> processor.preProcessPostSign(
                    signerDocument(effectiveParams, document), stamp.algorithm(), chain,
                    signerParams(effectiveParams, document), session);
        };

        return isAsicS(effectiveParams) ? asicSContainer(signature, document, effectiveParams)
                : signature;
    }

    /**
     * Cada PKCS#1 se identifica por su <b>posicion</b> en la sesion, no por el
     * {@code Id} del {@code TriSign}: en una contrafirma de varias hojas el
     * original puede repetirlo, y la postfirma lo usa solo para localizar el
     * hueco a sustituir por indice.
     */
    private static void attachPkcs1(final TriphaseData session,
            final List<SignatureValue> pkcs1s) {
        final List<TriphaseData.TriSign> signs = session.getTriSigns();
        if (pkcs1s == null || pkcs1s.size() != signs.size()) {
            throw new IllegalArgumentException("la sesion XAdES tiene " + signs.size()
                    + " prefirma(s), y han llegado " + (pkcs1s == null ? 0 : pkcs1s.size())
                    + " PKCS#1");
        }
        final Set<String> seen = new HashSet<>();
        for (final SignatureValue value : pkcs1s) {
            if (value.pkcs1B64() == null || value.pkcs1B64().isBlank()) {
                throw new IllegalArgumentException(
                        "falta el PKCS#1 de la prefirma «" + value.id() + "»");
            }
            if (!seen.add(value.id())) {
                throw new IllegalArgumentException("el PKCS#1 «" + value.id()
                        + "» llega dos veces: el segundo pisaria al primero sin decirlo");
            }
            signs.get(indexOf(value.id(), signs.size())).addProperty(PROPERTY_PKCS1,
                    value.pkcs1B64().trim());
        }
    }

    private static int indexOf(final String id, final int count) {
        try {
            final int index = Integer.parseInt(id);
            if (index >= 0 && index < count) {
                return index;
            }
        }
        catch (final NumberFormatException ignored) {
            // Cae al mismo fallo que un indice fuera de rango.
        }
        throw new IllegalArgumentException("el PKCS#1 «" + id + "» no corresponde a ninguna"
                + " prefirma de esta sesion trifasica");
    }

    private static String requireKnownOperation(final String operation) {
        final String requested = operation == null ? "" : operation.trim().toLowerCase(Locale.ROOT);
        if (OPERATIONS.contains(requested)) {
            return requested;
        }
        throw new IllegalArgumentException(
                "operacion XAdES desconocida: «" + operation + "»; se esperaba sign, cosign"
                        + " o countersign");
    }

    /**
     * El objetivo de la contrafirma. El defecto del original en XAdES es
     * {@code tree}, al reves que en CAdES: {@code XAdESTriPhaseSignerServerSide}
     * solo elige {@code leafs} cuando el llamante lo pide expresamente.
     */
    private static String counterSignTarget(final Properties extraParams) {
        final String sent = extraParams == null ? null : extraParams.getProperty(PARAM_TARGET);
        if (sent == null || sent.isBlank()) {
            return TARGET_TREE;
        }
        final String requested = sent.trim().toLowerCase(Locale.ROOT);
        if (TARGET_TREE.equals(requested) || TARGET_LEAFS.equals(requested)) {
            return requested;
        }
        throw new IllegalArgumentException("objetivo de contrafirma XAdES desconocido: «" + sent
                + "»; se esperaba tree o leafs");
    }

    private static String describe(final String operation, final String target) {
        return target == null ? String.valueOf(operation) : operation + " sobre " + target;
    }

    /**
     * Los {@code extraParams} con la variante resuelta a su nombre canonico.
     *
     * <p>Se copian antes de tocarlos porque el {@code Properties} es del llamante,
     * y se fija {@code format} en vez de dejarlo al defecto del builder para que
     * el sello diga cual se firmo y no haya que deducirlo.
     */
    private static Properties variantParams(final Properties extraParams) {
        final Properties copy = copyOf(extraParams);
        copy.setProperty(PARAM_FORMAT, variantOf(copy.getProperty(PARAM_FORMAT)));
        return copy;
    }

    private static String variantOf(final String requested) {
        final String name = requested == null ? "" : requested.trim();
        if (name.isEmpty() || FORMAT_XADES.equalsIgnoreCase(name)
                || FORMAT_ENVELOPING.equalsIgnoreCase(name)) {
            return FORMAT_ENVELOPING;
        }
        if (FORMAT_DETACHED.equalsIgnoreCase(name)) {
            return FORMAT_DETACHED;
        }
        if (FORMAT_ENVELOPED.equalsIgnoreCase(name)) {
            return FORMAT_ENVELOPED;
        }
        if (FORMAT_ASIC_S.equalsIgnoreCase(name)) {
            return FORMAT_ASIC_S;
        }
        throw new IllegalArgumentException("la variante XAdES «" + name + "» no la"
                + " atiende el puente: solo " + FORMAT_ENVELOPING + ", " + FORMAT_DETACHED + ", "
                + FORMAT_ENVELOPED + " y " + FORMAT_ASIC_S);
    }

    /**
     * Lo que se le da a firmar al firmador XAdES.
     *
     * <p>Un ASiC-S firma una referencia externa al fichero que va dentro del ZIP,
     * asi que lo que cruza es la huella y no el documento; en las demas variantes
     * es el documento mismo.
     */
    private static byte[] signerDocument(final Properties variantParams, final byte[] document)
            throws Exception {
        if (!isAsicS(variantParams)) {
            return document;
        }
        return MessageDigest.getInstance(AOSignConstants
                .getDigestAlgorithmName(externalReferencesHashAlgorithm(variantParams)))
                .digest(document);
    }

    /**
     * Los {@code extraParams} tal como los espera el firmador XAdES de dentro.
     *
     * <p>{@code XAdES-ASiC-S} nombra el contenedor, no una variante que el firmador
     * conozca: la firma que va dentro del ZIP es Externally Detached contra el
     * nombre del fichero empaquetado, como en el firmador ASiC del original.
     */
    private static Properties signerParams(final Properties variantParams, final byte[] document) {
        if (!isAsicS(variantParams)) {
            return variantParams;
        }
        final Properties copy =
                AOXAdESASiCSSigner.setASiCProperties(copyOf(variantParams), document);
        copy.setProperty(PARAM_KEEP_KEYINFO_UNSIGNED, Boolean.TRUE.toString());
        copy.setProperty(PARAM_FORMAT, FORMAT_EXTERNALLY_DETACHED);
        return copy;
    }

    private static byte[] asicSContainer(final byte[] signature, final byte[] document,
            final Properties variantParams) throws Exception {
        return ASiCUtil.createSContainer(signature, document, ASiCUtil.ENTRY_NAME_XML_SIGNATURE,
                variantParams.getProperty(PARAM_ASICS_FILENAME));
    }

    private static String externalReferencesHashAlgorithm(final Properties variantParams) {
        return variantParams.getProperty(PARAM_PRECALCULATED_HASH, variantParams
                .getProperty(PARAM_REFERENCES_DIGEST, XAdESConstants.DEFAULT_DIGEST_METHOD));
    }

    private static boolean isAsicS(final Properties variantParams) {
        return FORMAT_ASIC_S.equals(variantParams.getProperty(PARAM_FORMAT));
    }

    private static Properties copyOf(final Properties extraParams) {
        final Properties copy = new Properties();
        if (extraParams != null) {
            for (final String name : extraParams.stringPropertyNames()) {
                copy.setProperty(name, extraParams.getProperty(name));
            }
        }
        return copy;
    }
}
