package es.gob.afirma.nativebridge;

import java.io.ByteArrayInputStream;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Base64;
import java.util.Collections;
import java.util.List;
import java.util.Properties;
import java.util.TimeZone;

/**
 * El sello de sesion del ADR-0016: el bloque unico que la prefirma devuelve,
 * rFirma transporta <b>sin leer</b> y la postfirma vuelve a imponer.
 *
 * <p>La postfirma PAdES <b>regenera el PDF entero</b>, asi que exige recibir lo
 * mismo que la prefirma en tres cosas a la vez (ID-17): los {@code extraParams}
 * <b>efectivos</b>, el instante de firma y la zona horaria. Si algo difiere, la
 * postfirma <b>no falla</b>: completa, y el PDF sale con {@code Digest
 * Mismatch}. La firma se invalida en silencio.
 *
 * <p>AutoFirma lleva esos datos en campos separados —{@code TIME} por un lado,
 * los {@code extraParams} por otro, la zona horaria por ninguno— y esa forma es
 * justo la que dejo colarse el tercero durante anos. Aqui van en un solo
 * bloque, y la postfirma los toma <b>de el</b> en vez de del llamante: no hay
 * campos que recordar porque no hay campos que pasar. Lo unico que queda por
 * comprobar es que el sello y la sesion trifasica sean de la misma prefirma, y
 * eso es {@link #matchesSessionTime(String)}, una comparacion de bytes.
 *
 * <p>Dentro van el algoritmo, el {@code TIME} y la zona horaria. Fuera quedan
 * {@code PRE} y {@code PID}, que son salida de la prefirma y no configuracion.
 *
 * <p>El sello es <b>opaco para Rust por diseno</b>. Si algun dia hace falta leer
 * un valor de dentro, la respuesta es que la prefirma lo devuelva aparte: en
 * cuanto Rust lo interpreta puede reconstruirlo, y un sello reconstruible no
 * protege de nada.
 */
public final class SessionStamp {

    /** Primera linea del bloque. Cambiarla invalida los sellos antiguos a proposito. */
    private static final String MAGIC = "rfirma-session-stamp/1";

    private static final String KEY_ALGORITHM = "ALG";
    private static final String KEY_TIME = "TIME";
    private static final String KEY_TIME_ZONE = "TZ";
    /** Prefijo de cada extraParam efectivo. */
    private static final String PARAM_PREFIX = "P.";

    private final String algorithm;
    private final String time;
    private final String timeZoneId;
    private final Properties extraParams;

    private SessionStamp(final String algorithm, final String time,
            final String timeZoneId, final Properties extraParams) {
        this.algorithm = algorithm;
        this.time = time;
        this.timeZoneId = timeZoneId;
        this.extraParams = extraParams;
    }

    /**
     * Sella lo que la prefirma acaba de usar.
     *
     * @param algorithm    algoritmo de firma, p.ej. {@code SHA256withRSA}.
     * @param time         el {@code TIME} que la prefirma dejo en el {@code TriphaseData},
     *                     tal cual, como cadena: se compara con el, no con un {@code long}
     *                     reformateado.
     * @param timeZone     la zona horaria con la que se genero ese instante.
     * @param effectiveParams los {@code extraParams} <b>efectivos</b>, no los enviados:
     *                     {@code PdfSessionManager} muta el {@code Properties} que recibe y
     *                     {@code PAdESTriPhaseSigner:174} no lo clona, asi que el puente relee
     *                     el objeto justo despues de la prefirma. Guardar lo enviado
     *                     reintroduciria el fallo por otra puerta.
     */
    public static SessionStamp of(final String algorithm, final String time,
            final TimeZone timeZone, final Properties effectiveParams) {
        final Properties copy = new Properties();
        for (final String name : effectiveParams.stringPropertyNames()) {
            copy.setProperty(name, effectiveParams.getProperty(name));
        }
        return new SessionStamp(algorithm, time, timeZone.getID(), copy);
    }

    /** El bloque, en Base64. Es lo unico que sale del puente. */
    public String encode() {
        final StringBuilder sb = new StringBuilder(MAGIC).append('\n');
        append(sb, KEY_ALGORITHM, this.algorithm);
        append(sb, KEY_TIME, this.time);
        append(sb, KEY_TIME_ZONE, this.timeZoneId);
        final List<String> names = new ArrayList<>(this.extraParams.stringPropertyNames());
        // Orden fijo: un Properties no lo tiene, y sin esto dos sellos del mismo
        // contenido saldrian distintos segun el orden de iteracion.
        Collections.sort(names);
        for (final String name : names) {
            append(sb, PARAM_PREFIX + name, this.extraParams.getProperty(name));
        }
        return Base64.getEncoder().encodeToString(sb.toString().getBytes(StandardCharsets.UTF_8));
    }

    /** Reconstruye el sello que devolvio la prefirma. */
    public static SessionStamp decode(final String encoded) {
        if (encoded == null || encoded.isEmpty()) {
            throw new IllegalArgumentException("falta el sello de sesion de la prefirma");
        }
        final String block;
        try {
            block = new String(Base64.getDecoder().decode(encoded.trim()), StandardCharsets.UTF_8);
        }
        catch (final IllegalArgumentException e) {
            throw new IllegalArgumentException("el sello de sesion no es Base64 valido", e);
        }
        final String[] lines = block.split("\n", -1);
        if (lines.length == 0 || !MAGIC.equals(lines[0])) {
            throw new IllegalArgumentException(
                    "el sello de sesion no es de esta version del puente: se esperaba " + MAGIC);
        }

        String algorithm = null;
        String time = null;
        String timeZoneId = null;
        final Properties params = new Properties();
        for (int i = 1; i < lines.length; i++) {
            if (lines[i].isEmpty()) {
                continue;
            }
            final int eq = lines[i].indexOf('=');
            if (eq < 0) {
                throw new IllegalArgumentException("linea sin '=' en el sello de sesion: " + lines[i]);
            }
            final String key = lines[i].substring(0, eq);
            final String value = unescape(lines[i].substring(eq + 1));
            switch (key) {
                case KEY_ALGORITHM -> algorithm = value;
                case KEY_TIME -> time = value;
                case KEY_TIME_ZONE -> timeZoneId = value;
                default -> {
                    if (!key.startsWith(PARAM_PREFIX)) {
                        throw new IllegalArgumentException(
                                "campo desconocido en el sello de sesion: " + key);
                    }
                    params.setProperty(key.substring(PARAM_PREFIX.length()), value);
                }
            }
        }
        if (algorithm == null || time == null || timeZoneId == null) {
            throw new IllegalArgumentException(
                    "al sello de sesion le falta ALG, TIME o TZ");
        }
        return new SessionStamp(algorithm, time, timeZoneId, params);
    }

    /**
     * La comprobacion del ADR-0016, hecha antes de firmar: el {@code TIME} del
     * sello y el de la sesion trifasica tienen que ser el mismo byte a byte.
     *
     * <p>Es el unico campo que hay que comparar, y no por descuido: el
     * algoritmo, los {@code extraParams} y la zona horaria la postfirma los toma
     * del sello, no del llamante, asi que no pueden desviarse. El {@code TIME}
     * es el que viaja aparte —dentro del {@code TriphaseData}, junto al
     * {@code PK1} que Rust anade—, y por tanto el unico que puede llegar
     * desparejado.
     */
    public boolean matchesSessionTime(final String sessionTime) {
        return this.time.equals(sessionTime);
    }

    public String algorithm() {
        return this.algorithm;
    }

    public String time() {
        return this.time;
    }

    public TimeZone timeZone() {
        return TimeZone.getTimeZone(this.timeZoneId);
    }

    /** Copia de los {@code extraParams} efectivos: el sello no se deja mutar. */
    public Properties extraParams() {
        final Properties copy = new Properties();
        for (final String name : this.extraParams.stringPropertyNames()) {
            copy.setProperty(name, this.extraParams.getProperty(name));
        }
        return copy;
    }

    /** Los {@code extraParams} tal cual los envia el llamante: lineas "clave=valor". */
    public static Properties parseParams(final String raw) {
        final Properties params = new Properties();
        if (raw == null || raw.isEmpty()) {
            return params;
        }
        try {
            params.load(new ByteArrayInputStream(raw.getBytes(StandardCharsets.UTF_8)));
        }
        catch (final java.io.IOException e) {
            throw new IllegalArgumentException("extraParams mal formados: " + e.getMessage(), e);
        }
        return params;
    }

    private static void append(final StringBuilder sb, final String key, final String value) {
        sb.append(key).append('=').append(escape(value)).append('\n');
    }

    /** Un valor con salto de linea (p.ej. el texto de la rubrica) partiria el bloque. */
    private static String escape(final String value) {
        return value.replace("\\", "\\\\").replace("\n", "\\n").replace("\r", "\\r");
    }

    private static String unescape(final String value) {
        final StringBuilder sb = new StringBuilder(value.length());
        for (int i = 0; i < value.length(); i++) {
            final char c = value.charAt(i);
            if (c != '\\' || i + 1 >= value.length()) {
                sb.append(c);
                continue;
            }
            i++;
            switch (value.charAt(i)) {
                case 'n' -> sb.append('\n');
                case 'r' -> sb.append('\r');
                case '\\' -> sb.append('\\');
                default -> throw new IllegalArgumentException(
                        "escape desconocido en el sello de sesion: \\" + value.charAt(i));
            }
        }
        return sb.toString();
    }
}
