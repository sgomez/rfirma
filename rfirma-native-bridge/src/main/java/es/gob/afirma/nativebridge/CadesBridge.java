package es.gob.afirma.nativebridge;

import java.nio.charset.StandardCharsets;
import java.security.cert.X509Certificate;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Locale;
import java.util.Properties;
import java.util.Set;
import java.util.TimeZone;

import es.gob.afirma.core.signers.CounterSignTarget;
import es.gob.afirma.core.signers.TriphaseData;
import es.gob.afirma.triphase.signer.processors.CAdESASiCSTriPhasePreProcessor;
import es.gob.afirma.triphase.signer.processors.CAdESTriPhasePreProcessor;

/**
 * Las dos fases de la firma trifasica CAdES que corresponden a Java, en Java
 * puro: sin tipos de GraalVM y sin cadenas C.
 *
 * <p>Hermana de {@link PadesBridge}, y con la misma division de trabajo:
 * {@link NativeBridge} es solo la frontera y aqui vive lo que decide algo, donde
 * una prueba de JUnit lo puede llamar sin construir la imagen nativa.
 *
 * <p><b>La fase 2 no esta aqui, y no va a estarlo (ADR-0001).</b> La clave
 * privada no entra nunca en el isolate de Java.
 *
 * <p>Las tres operaciones —{@code sign}, {@code cosign} y {@code countersign}—
 * entran por los mismos dos metodos. La entrada de una cofirma o de una
 * contrafirma es la <b>firma existente</b>, no el dato original.
 *
 * <p>{@code extraParams.format} elige entre la firma CAdES suelta y el
 * contenedor {@code CAdES-ASiC-S}, que es otro procesador del original; el
 * contenedor no admite ni cofirma ni contrafirma, y quien lo rechaza es el.
 *
 * <p>Una contrafirma prefirma <b>una hoja o mas</b>, asi que la prefirma
 * devuelve una <b>lista</b> de prefirmas identificadas y la postfirma recibe un
 * PKCS#1 por cada una. Para {@code sign} y {@code cosign} la lista tiene un
 * elemento.
 *
 * <p>A diferencia de PAdES <b>no hay cerrojo de zona horaria</b>: el instante de
 * firma de CAdES viaja dentro de los atributos firmados de la prefirma —
 * codificado en UTC— y la postfirma no reconstruye ninguna fecha, asi que no hay
 * estado global que imponer ni que proteger.
 */
public final class CadesBridge {

    /** Instante que elige el puente para atar el sello a la sesion. */
    private static final String PROPERTY_SIGN_TIME = "TIME";
    /** Atributos firmados CAdES de la prefirma. */
    private static final String PROPERTY_PRESIGN = "PRE";
    /** Donde deposita Rust el PKCS#1. */
    private static final String PROPERTY_PKCS1 = "PK1";
    /** La operacion que abrio la sesion, para que la postfirma no pueda cambiarla. */
    private static final String PROPERTY_OPERATION = "OP";
    /** El objetivo de la contrafirma, por lo mismo. */
    private static final String PROPERTY_TARGET = "TARGET";

    private static final String OPERATION_SIGN = "sign";
    private static final String OPERATION_COSIGN = "cosign";
    private static final String OPERATION_COUNTERSIGN = "countersign";
    private static final Set<String> OPERATIONS =
            Set.of(OPERATION_SIGN, OPERATION_COSIGN, OPERATION_COUNTERSIGN);

    private static final String PARAM_FORMAT = "format";
    private static final String FORMAT_ASIC_S = "CAdES-ASiC-S";

    private static final String PARAM_TARGET = "target";
    private static final String TARGET_TREE = "tree";
    private static final String TARGET_LEAFS = "leafs";

    private CadesBridge() { }

    /** Los atributos firmados de una de las firmas de la sesion, con su identificador. */
    public record PreSign(String id, String pre) { }

    /** Lo que la prefirma entrega a Rust. */
    public record PreSignResult(String session, List<PreSign> pres, String stamp) { }

    /** Lo que Rust devuelve por cada prefirma: el PKCS#1 de la fase 2. */
    public record SignatureValue(String id, String pkcs1B64) { }

    /**
     * Prefirma CAdES.
     *
     * <p>Cada {@code pre} son los <b>atributos firmados CAdES en ASN.1 DER</b>,
     * igual que en PAdES: Rust recibe un bloque que debe hashear y firmar como
     * cualquier PKCS#1 sobre bytes arbitrarios.
     *
     * @param document    datos a firmar, o la firma CAdES a cofirmar o contrafirmar.
     * @param algorithm   algoritmo de firma, p.ej. {@code SHA256withRSA}.
     * @param chain       cadena de certificados del firmante.
     * @param extraParams los extraParams enviados; {@code format}, {@code mode},
     *                    {@code target} y la politica viajan aqui sin traducir.
     * @param operation   {@code sign}, {@code cosign} o {@code countersign}.
     */
    public static PreSignResult preSign(final byte[] document, final String algorithm,
            final X509Certificate[] chain, final Properties extraParams, final String operation)
            throws Exception {

        final String requested = requireKnownOperation(operation);
        final String target =
                OPERATION_COUNTERSIGN.equals(requested) ? counterSignTarget(extraParams) : null;
        final Properties effectiveParams = copyOf(extraParams);

        final TimeZone timeZone = TimeZone.getDefault();
        final String time = Long.toString(System.currentTimeMillis());
        final CAdESTriPhasePreProcessor processor = processorFor(effectiveParams);
        final TriphaseData session = switch (requested) {
            case OPERATION_COSIGN -> processor.preProcessPreCoSign(
                    document, algorithm, chain, effectiveParams, false);
            case OPERATION_COUNTERSIGN -> processor.preProcessPreCounterSign(
                    document, algorithm, chain, effectiveParams,
                    CounterSignTarget.getTarget(target), false);
            default -> processor.preProcessPreSign(
                    document, algorithm, chain, effectiveParams, false);
        };

        if (session.getSignsCount() < 1) {
            throw new IllegalStateException("la prefirma CAdES no ha devuelto ninguna firma");
        }
        final List<PreSign> pres = new ArrayList<>();
        for (final TriphaseData.TriSign signConfig : session.getTriSigns()) {
            final String pre = signConfig.getProperty(PROPERTY_PRESIGN);
            if (pre == null) {
                throw new IllegalStateException("la prefirma CAdES no ha devuelto PRE");
            }
            pres.add(new PreSign(signConfig.getId(), pre));
        }

        // La sesion CAdES no trae TIME —solo PAdES lo pone—, asi que el puente
        // anade el suyo, y con el la operacion: es lo que la postfirma compara
        // contra el sello para saber que la sesion que recibe es la que se prefirmo.
        final TriphaseData.TriSign first = session.getSign(0);
        first.addProperty(PROPERTY_SIGN_TIME, time);
        first.addProperty(PROPERTY_OPERATION, requested);
        if (target != null) {
            first.addProperty(PROPERTY_TARGET, target);
        }

        final SessionStamp stamp = SessionStamp.of(algorithm, time, timeZone, effectiveParams,
                document, chain, requested, target);

        return new PreSignResult(session.toString(), List.copyOf(pres), stamp.encode());
    }

    /**
     * Postfirma CAdES: ensambla el CMS firmado.
     *
     * <p>Los {@code extraParams}, el algoritmo, la operacion y el objetivo de la
     * contrafirma salen del <b>sello</b>, no del llamante (ADR-0016). Lo que viaja
     * aparte —la sesion trifasica, el documento y la cadena de certificados— se
     * compara contra el sello antes de firmar.
     *
     * @param document   el MISMO documento que recibio la prefirma.
     * @param chain      la MISMA cadena de certificados.
     * @param stampB64   el sello que devolvio la prefirma, tal cual.
     * @param sessionXml el {@code TriphaseData} de la prefirma, tal cual.
     * @param pkcs1s     un PKCS#1 por cada prefirma de la sesion, con su identificador.
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
                            + " en la sesion. Firmar asi produciria un CMS que no es el que"
                            + " se prefirmo, y sin dar ningun error.");
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
            // El documento viaja aparte y la postfirma no lo vuelve a hashear: los
            // atributos firmados llevan el digest del que recibio la prefirma. Con
            // otro sale un CMS completo cuya firma no cubre lo que dice cubrir.
            throw new SessionStampMismatchException(
                    "el documento que recibe la postfirma no es el que se prefirmo: el sello"
                            + " lleva el SHA-256 " + stamp.documentDigest() + ". Firmar asi produciria"
                            + " una firma que no cubre estos datos, sin dar ningun error.");
        }

        if (!stamp.matchesChain(chain)) {
            throw new SessionStampMismatchException(
                    "la cadena de certificados que recibe la postfirma no es la que"
                            + " prefirmo: el sello lleva el SHA-256 " + stamp.chainDigest()
                            + ". Firmar asi produciria un CMS que dice estar firmado por"
                            + " quien no lo firmo, con la firma invalida y sin dar ningun"
                            + " error.");
        }

        attachPkcs1(session, pkcs1s);

        final Properties effectiveParams = stamp.extraParams();
        final CAdESTriPhasePreProcessor processor = processorFor(effectiveParams);
        return switch (requireKnownOperation(stamp.operation())) {
            case OPERATION_COSIGN -> processor.preProcessPostCoSign(
                    document, stamp.algorithm(), chain, effectiveParams, session);
            case OPERATION_COUNTERSIGN -> processor.preProcessPostCounterSign(
                    document, stamp.algorithm(), chain, effectiveParams, session,
                    CounterSignTarget.getTarget(stamp.target()));
            default -> processor.preProcessPostSign(
                    document, stamp.algorithm(), chain, effectiveParams, session);
        };
    }

    private static void attachPkcs1(final TriphaseData session,
            final List<SignatureValue> pkcs1s) {
        if (pkcs1s == null || pkcs1s.isEmpty()) {
            throw new IllegalArgumentException("falta el PKCS#1 de la fase 2");
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
            // Por la lista viva, no por getTriSigns(id): devuelve copias.
            boolean attached = false;
            for (final TriphaseData.TriSign signConfig : session.getTriSigns()) {
                if (signConfig.getId().equals(value.id())) {
                    signConfig.addProperty(PROPERTY_PKCS1, value.pkcs1B64().trim());
                    attached = true;
                }
            }
            if (!attached) {
                throw new IllegalArgumentException("el PKCS#1 «" + value.id()
                        + "» no corresponde a ninguna prefirma de esta sesion trifasica");
            }
        }
        for (final TriphaseData.TriSign signConfig : session.getTriSigns()) {
            if (signConfig.getProperty(PROPERTY_PKCS1) == null) {
                throw new IllegalArgumentException("la prefirma «" + signConfig.getId()
                        + "» se ha quedado sin PKCS#1: la firma saldria incompleta");
            }
        }
    }

    private static String requireKnownOperation(final String operation) {
        final String requested = operation == null ? "" : operation.trim().toLowerCase(Locale.ROOT);
        if (OPERATIONS.contains(requested)) {
            return requested;
        }
        throw new IllegalArgumentException(
                "operacion CAdES desconocida: «" + operation + "»; se esperaba sign, cosign"
                        + " o countersign");
    }

    private static String counterSignTarget(final Properties extraParams) {
        final String sent = extraParams == null ? null : extraParams.getProperty(PARAM_TARGET);
        if (sent == null || sent.isBlank()) {
            return TARGET_LEAFS;
        }
        final String requested = sent.trim().toLowerCase(Locale.ROOT);
        if (TARGET_TREE.equals(requested) || TARGET_LEAFS.equals(requested)) {
            return requested;
        }
        throw new IllegalArgumentException("objetivo de contrafirma CAdES desconocido: «" + sent
                + "»; se esperaba tree o leafs");
    }

    private static String describe(final String operation, final String target) {
        return target == null ? String.valueOf(operation) : operation + " sobre " + target;
    }

    /** El procesador ASiC-S impone sus propias reglas; el puente no decide ninguna. */
    private static CAdESTriPhasePreProcessor processorFor(final Properties effectiveParams) {
        return isAsicS(effectiveParams) ? new CAdESASiCSTriPhasePreProcessor()
                : new CAdESTriPhasePreProcessor();
    }

    private static boolean isAsicS(final Properties effectiveParams) {
        final String format = effectiveParams.getProperty(PARAM_FORMAT);
        return format != null && FORMAT_ASIC_S.equalsIgnoreCase(format.trim());
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
