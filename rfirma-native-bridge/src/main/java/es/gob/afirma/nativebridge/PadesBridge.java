package es.gob.afirma.nativebridge;

import java.io.ByteArrayInputStream;
import java.nio.charset.StandardCharsets;
import java.security.cert.CertificateFactory;
import java.security.cert.X509Certificate;
import java.util.ArrayList;
import java.util.Base64;
import java.util.List;
import java.util.Properties;
import java.util.TimeZone;

import es.gob.afirma.core.signers.TriphaseData;
import es.gob.afirma.triphase.signer.processors.PAdESTriPhasePreProcessor;

/**
 * Las dos fases de la firma trifasica PAdES que corresponden a Java, en Java
 * puro: sin tipos de GraalVM y sin cadenas C.
 *
 * <p>La separacion es lo que hace probable el puente. {@link NativeBridge} es
 * solo la frontera —convierte cadenas, serializa JSON y reserva memoria del
 * C-heap— y todo lo que decide algo vive aqui, donde una prueba de JUnit lo
 * puede llamar sin construir la imagen nativa.
 *
 * <p><b>La fase 2 no esta aqui, y no va a estarlo (ADR-0001).</b> La clave
 * privada no entra nunca en el isolate de Java: el PKCS#1 sobre los atributos
 * firmados lo calcula Rust contra el PKCS#11 del sistema. Java hace la prefirma
 * y la postfirma, y nada mas.
 */
public final class PadesBridge {

    /** Instante de la firma, fijado por la prefirma. */
    private static final String PROPERTY_SIGN_TIME = "TIME";
    /** Atributos firmados CAdES de la prefirma. */
    private static final String PROPERTY_PRESIGN = "PRE";
    /** Donde deposita Rust el PKCS#1. */
    private static final String PROPERTY_PKCS1 = "PK1";

    private PadesBridge() { }

    /** Lo que la prefirma entrega a Rust. */
    public record PreSignResult(String session, String preSignB64, String stamp) { }

    /**
     * Prefirma PAdES.
     *
     * <p>El {@code preSignB64} son los <b>atributos firmados CAdES en ASN.1
     * DER</b> (ID-15), no un hash y no un {@code DigestInfo}: Rust recibe un
     * bloque que debe hashear y firmar como cualquier PKCS#1 sobre bytes
     * arbitrarios.
     *
     * @param pdf         PDF de entrada.
     * @param algorithm   algoritmo de firma, p.ej. {@code SHA256withRSA}.
     * @param chain       cadena de certificados del firmante.
     * @param extraParams los extraParams <b>enviados</b>. AutoFirma los muta en
     *                    el sitio; lo que va al sello son los efectivos, releidos
     *                    de este mismo objeto despues de la llamada (ADR-0016).
     */
    public static PreSignResult preSign(final byte[] pdf, final String algorithm,
            final X509Certificate[] chain, final Properties extraParams) throws Exception {

        // La zona horaria se captura AQUI porque preProcessPreSign construye su
        // GregorianCalendar con la de por defecto, y el desfase entra dentro del
        // rango firmado (#23). Fuera del sello se heredaria del entorno de la
        // postfirma, que puede no ser el mismo.
        final TimeZone timeZone = TimeZone.getDefault();

        final TriphaseData session = new PAdESTriPhasePreProcessor().preProcessPreSign(
                pdf, algorithm, chain, extraParams, false);

        if (session.getSignsCount() < 1) {
            throw new IllegalStateException("la prefirma PAdES no ha devuelto ninguna firma");
        }
        final TriphaseData.TriSign signConfig = session.getSign(0);
        final String time = signConfig.getProperty(PROPERTY_SIGN_TIME);
        final String preSign = signConfig.getProperty(PROPERTY_PRESIGN);

        // extraParams EFECTIVOS: el objeto que acaba de mutar la prefirma.
        final SessionStamp stamp = SessionStamp.of(algorithm, time, timeZone, extraParams);

        return new PreSignResult(session.toString(), preSign, stamp.encode());
    }

    /**
     * Postfirma PAdES: ensambla el PDF firmado.
     *
     * <p>Los {@code extraParams}, el algoritmo y la zona horaria salen del
     * <b>sello</b>, no del llamante: la postfirma los impone en vez de
     * confiar en que vuelvan a coincidir. Lo unico que se compara es el
     * {@code TIME}, que es el que viaja aparte dentro del {@code TriphaseData}.
     *
     * @param pdf        el MISMO PDF que recibio la prefirma.
     * @param chain      la MISMA cadena de certificados.
     * @param stampB64   el sello que devolvio la prefirma, tal cual.
     * @param sessionXml el {@code TriphaseData} de la prefirma, tal cual.
     * @param pkcs1B64   el PKCS#1 que Rust calculo sobre los atributos firmados.
     */
    public static byte[] postSign(final byte[] pdf, final X509Certificate[] chain,
            final String stampB64, final String sessionXml, final String pkcs1B64)
            throws Exception {

        final SessionStamp stamp = SessionStamp.decode(stampB64);
        final TriphaseData session = TriphaseData.parser(sessionXml.getBytes(StandardCharsets.UTF_8));
        if (session.getSignsCount() < 1) {
            throw new IllegalStateException(
                    "la sesion trifasica no contiene ninguna firma");
        }
        final TriphaseData.TriSign signConfig = session.getSign(0);

        final String sessionTime = signConfig.getProperty(PROPERTY_SIGN_TIME);
        if (!stamp.matchesSessionTime(sessionTime)) {
            // Sin esta comprobacion la postfirma NO fallaria: completaria, y el
            // PDF saldria con "Digest Mismatch". La firma se invalida en
            // silencio, que es el fallo que el ADR-0016 existe para cerrar.
            throw new SessionStampMismatchException(
                    "el sello de sesion no corresponde a esta sesion trifasica: TIME "
                            + stamp.time() + " en el sello frente a " + sessionTime
                            + " en la sesion. Firmar asi produciria un PDF con «Digest"
                            + " Mismatch» sin dar ningun error.");
        }

        if (pkcs1B64 == null || pkcs1B64.isBlank()) {
            throw new IllegalArgumentException("falta el PKCS#1 de la fase 2");
        }
        signConfig.addProperty(PROPERTY_PKCS1, pkcs1B64.trim());

        // La zona horaria de la prefirma se IMPONE: preProcessPostSign reconstruye
        // el instante con Calendar.getInstance(), que usa la de por defecto.
        final TimeZone previous = TimeZone.getDefault();
        TimeZone.setDefault(stamp.timeZone());
        try {
            return new PAdESTriPhasePreProcessor().preProcessPostSign(
                    pdf, stamp.algorithm(), chain, stamp.extraParams(), session);
        }
        finally {
            TimeZone.setDefault(previous);
        }
    }

    /** Cadena de certificados en Base64, separados por {@code ';'}. */
    public static X509Certificate[] parseCertificates(final String chainB64) throws Exception {
        final CertificateFactory cf = CertificateFactory.getInstance("X.509");
        final List<X509Certificate> certs = new ArrayList<>();
        for (final String b64 : chainB64.split(";")) {
            if (b64.isBlank()) {
                continue;
            }
            certs.add((X509Certificate) cf.generateCertificate(
                    new ByteArrayInputStream(Base64.getDecoder().decode(b64.trim()))));
        }
        if (certs.isEmpty()) {
            throw new IllegalArgumentException("la cadena de certificados esta vacia");
        }
        return certs.toArray(new X509Certificate[0]);
    }

    /** El sello recibido no es el de esta sesion trifasica. */
    public static final class SessionStampMismatchException extends IllegalStateException {

        private static final long serialVersionUID = 1L;

        SessionStampMismatchException(final String message) {
            super(message);
        }
    }
}
