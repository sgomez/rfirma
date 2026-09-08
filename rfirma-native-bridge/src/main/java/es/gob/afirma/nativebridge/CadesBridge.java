package es.gob.afirma.nativebridge;

import java.nio.charset.StandardCharsets;
import java.security.cert.X509Certificate;
import java.util.Locale;
import java.util.Properties;
import java.util.Set;
import java.util.TimeZone;

import es.gob.afirma.core.signers.TriphaseData;
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

    private static final String OPERATION_SIGN = "sign";
    /** Las que el protocolo nombra y este puente todavia no atiende. */
    private static final Set<String> DEFERRED_OPERATIONS = Set.of("cosign", "countersign");

    private CadesBridge() { }

    /** Lo que la prefirma entrega a Rust. */
    public record PreSignResult(String session, String preSignB64, String stamp) { }

    /**
     * Prefirma CAdES.
     *
     * <p>El {@code preSignB64} son los <b>atributos firmados CAdES en ASN.1
     * DER</b>, igual que en PAdES: Rust recibe un bloque que debe hashear y firmar
     * como cualquier PKCS#1 sobre bytes arbitrarios.
     *
     * @param document    datos a firmar.
     * @param algorithm   algoritmo de firma, p.ej. {@code SHA256withRSA}.
     * @param chain       cadena de certificados del firmante.
     * @param extraParams los extraParams enviados; {@code mode} y la politica
     *                    viajan aqui sin traducir.
     * @param operation   {@code sign}, {@code cosign} o {@code countersign}.
     */
    public static PreSignResult preSign(final byte[] document, final String algorithm,
            final X509Certificate[] chain, final Properties extraParams, final String operation)
            throws Exception {

        requireSignOperation(operation);

        final TimeZone timeZone = TimeZone.getDefault();
        final String time = Long.toString(System.currentTimeMillis());
        final TriphaseData session = new CAdESTriPhasePreProcessor().preProcessPreSign(
                document, algorithm, chain, extraParams, false);

        if (session.getSignsCount() < 1) {
            throw new IllegalStateException("la prefirma CAdES no ha devuelto ninguna firma");
        }
        final TriphaseData.TriSign signConfig = session.getSign(0);
        final String preSign = signConfig.getProperty(PROPERTY_PRESIGN);
        if (preSign == null) {
            throw new IllegalStateException("la prefirma CAdES no ha devuelto PRE");
        }

        // La sesion CAdES no trae TIME —solo PAdES lo pone—, asi que el puente
        // anade el suyo: es lo que la postfirma compara contra el sello para saber
        // que la sesion que recibe es la que se prefirmo.
        signConfig.addProperty(PROPERTY_SIGN_TIME, time);

        final SessionStamp stamp =
                SessionStamp.of(algorithm, time, timeZone, extraParams, document, chain);

        return new PreSignResult(session.toString(), preSign, stamp.encode());
    }

    /**
     * Postfirma CAdES: ensambla el CMS firmado.
     *
     * <p>Los {@code extraParams} y el algoritmo salen del <b>sello</b>, no del
     * llamante (ADR-0016). Lo que viaja aparte —la sesion trifasica, el documento
     * y la cadena de certificados— se compara contra el sello antes de firmar.
     *
     * @param document   el MISMO documento que recibio la prefirma.
     * @param chain      la MISMA cadena de certificados.
     * @param stampB64   el sello que devolvio la prefirma, tal cual.
     * @param sessionXml el {@code TriphaseData} de la prefirma, tal cual.
     * @param pkcs1B64   el PKCS#1 que Rust calculo sobre los atributos firmados.
     */
    public static byte[] postSign(final byte[] document, final X509Certificate[] chain,
            final String stampB64, final String sessionXml, final String pkcs1B64)
            throws Exception {

        final SessionStamp stamp = SessionStamp.decode(stampB64);
        final TriphaseData session =
                TriphaseData.parser(sessionXml.getBytes(StandardCharsets.UTF_8));
        if (session.getSignsCount() < 1) {
            throw new IllegalStateException("la sesion trifasica no contiene ninguna firma");
        }
        final TriphaseData.TriSign signConfig = session.getSign(0);

        final String sessionTime = signConfig.getProperty(PROPERTY_SIGN_TIME);
        if (!stamp.matchesSessionTime(sessionTime)) {
            throw new SessionStampMismatchException(
                    "el sello de sesion no corresponde a esta sesion trifasica: TIME "
                            + stamp.time() + " en el sello frente a " + sessionTime
                            + " en la sesion. Firmar asi produciria un CMS que no es el que"
                            + " se prefirmo, y sin dar ningun error.");
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

        if (pkcs1B64 == null || pkcs1B64.isBlank()) {
            throw new IllegalArgumentException("falta el PKCS#1 de la fase 2");
        }
        signConfig.addProperty(PROPERTY_PKCS1, pkcs1B64.trim());

        return new CAdESTriPhasePreProcessor().preProcessPostSign(
                document, stamp.algorithm(), chain, stamp.extraParams(), session);
    }

    private static void requireSignOperation(final String operation) {
        final String requested = operation == null ? "" : operation.trim().toLowerCase(Locale.ROOT);
        if (OPERATION_SIGN.equals(requested)) {
            return;
        }
        if (DEFERRED_OPERATIONS.contains(requested)) {
            throw new UnsupportedOperationException(
                    "la operacion CAdES «" + requested + "» todavia no la atiende este puente");
        }
        throw new IllegalArgumentException(
                "operacion CAdES desconocida: «" + operation + "»; se esperaba sign, cosign"
                        + " o countersign");
    }
}
