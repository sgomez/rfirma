package es.gob.afirma.nativebridge;

import java.util.Enumeration;
import java.util.Properties;
import java.util.TreeSet;

import es.gob.afirma.core.signers.ExtraParamsProcessor;

/**
 * El expansor de {@code expPolicy} del original, prestado (ID-266).
 *
 * <p><b>Aqui no se decide nada</b>, igual que en {@link FilterBridge}: quien
 * sabe en que se convierte {@code expPolicy=FirmaAGE} es
 * {@link ExtraParamsProcessor}, que ya vive dentro de {@code afirma-core}.
 * Reimplementarlo en Rust seria escribir a mano el identificador de la
 * politica, su huella, su algoritmo y su calificador, y expandirlos mal es
 * firmar con una politica distinta de la que la sede declaro: el fallo no se
 * ve, la firma sale, y lo que sale no es lo que se pidio.
 *
 * <h2>Sin estado y sin sello (ADR-0016)</h2>
 *
 * Entra el bloque de {@code extraParams} tal y como lo mando la sede y sale el
 * bloque expandido. No abre ninguna sesion trifasica, asi que no lleva sello.
 *
 * <h2>Lo que el original hace, y se hereda</h2>
 *
 * <ul>
 * <li>Sin {@code expPolicy}, {@code expandProperties} devuelve una copia y no
 * toca nada ({@code ExtraParamsProcessor:137}-{@code 139}).</li>
 * <li>Una politica que no admite expansion, o que no case con el formato,
 * lanza {@code IncompatiblePolicyException} <b>y borra la clave</b>
 * ({@code :146}, {@code :172}). Ese fallo sube tal cual: una sede que declara
 * una politica que no se puede aplicar no debe recibir una firma sin ella.</li>
 * <li>En CAdES sin {@code mode} declarado, un documento de menos de 1 MB se
 * firma en modo implicito y uno mayor en explicito ({@code :230}-{@code :241}).</li>
 * <li>En PAdES con la politica de la AGE el subfiltro tiene que ser
 * {@code ETSI.CAdES.detached}; otro distinto es incompatible ({@code :294}-
 * {@code :303}). Es el mismo que rFirma envia siempre
 * ({@code signing::config::SUB_FILTER}), asi que la incompatibilidad solo
 * puede venir de la sede.</li>
 * </ul>
 */
public final class ExtraParamsBridge {

    private ExtraParamsBridge() { }

    /** Lo unico que el original mira de los datos firmados: si llegan a 1 MB ({@code ExtraParamsProcessor:239}). */
    private static final int SIZE_1MB = 1024 * 1024;

    /**
     * El bloque de {@code extraParams} con la politica ya expandida.
     *
     * @param extraParams      el bloque {@code java.util.Properties} de la sede.
     * @param format           el formato de firma.
     * @param signedDataLength la longitud de los datos que se van a firmar.
     * @return el bloque expandido, en el mismo formato.
     * @throws ExtraParamsProcessor.IncompatiblePolicyException si la politica
     *         declarada no se puede aplicar a ese formato.
     */
    public static String expand(final Properties extraParams, final String format,
            final long signedDataLength)
            throws ExtraParamsProcessor.IncompatiblePolicyException {
        return write(ExtraParamsProcessor.expandProperties(
                extraParams, dataOfTheSameSizeClass(signedDataLength), format));
    }

    private static byte[] dataOfTheSameSizeClass(final long signedDataLength) {
        return new byte[(int) Math.min(signedDataLength, SIZE_1MB)];
    }

    /**
     * El bloque {@code java.util.Properties} de unas propiedades, en orden
     * estable y con los mismos escapes que escribe el lado de Rust.
     *
     * <p>No se usa {@code Properties.store}: escribe una linea de comentario
     * con la fecha, y una salida que cambia en cada llamada no se puede
     * comparar en una prueba.
     */
    static String write(final Properties properties) {
        final TreeSet<String> keys = new TreeSet<>();
        final Enumeration<?> names = properties.propertyNames();
        while (names.hasMoreElements()) {
            keys.add(String.valueOf(names.nextElement()));
        }

        final StringBuilder block = new StringBuilder();
        for (final String key : keys) {
            block.append(escape(key, true))
                 .append('=')
                 .append(escape(properties.getProperty(key), false))
                 .append('\n');
        }
        return block.toString();
    }

    /**
     * Escapa un trozo del bloque. En una <b>clave</b> hay que escapar ademas
     * los tres separadores, porque ahi es donde termina la clave.
     */
    private static String escape(final String text, final boolean isKey) {
        final StringBuilder escaped = new StringBuilder(text.length());
        for (int i = 0; i < text.length(); i++) {
            final char c = text.charAt(i);
            switch (c) {
                case '\\' -> escaped.append("\\\\");
                case '\n' -> escaped.append("\\n");
                case '\r' -> escaped.append("\\r");
                case '\t' -> escaped.append("\\t");
                case '=', ':', ' ' -> {
                    if (isKey) {
                        escaped.append('\\');
                    }
                    escaped.append(c);
                }
                default -> escaped.append(c);
            }
        }
        return escaped.toString();
    }
}
