package es.gob.afirma.nativebridge;

import java.io.ByteArrayInputStream;
import java.security.cert.CertificateFactory;
import java.security.cert.X509Certificate;
import java.util.ArrayList;
import java.util.Base64;
import java.util.List;
import java.util.Properties;

import es.gob.afirma.keystores.CertificateFilter;
import es.gob.afirma.keystores.filters.CertFilterManager;

/**
 * El motor de filtros de certificado del original, prestado (ID-252).
 *
 * <p><b>Aqui no se decide nada</b>, igual que en {@link PadesBridge}: quien
 * decide que certificados pasan es {@code CertFilterManager}, que es el codigo
 * de AutoFirma sin tocar. Reescribirlo en Rust quedo descartado (ID-253) porque
 * rompe las dos reglas de composicion que se reimplementan mal —dentro de una
 * expresion {@code ;} es <b>Y</b>, entre {@code filters.N=} es <b>O</b>— y el
 * {@code nonexpired} implicito de la ETSI cuando la sede no manda ningun filtro.
 *
 * <h2>Sin estado y sin sello (ADR-0016)</h2>
 *
 * La llamada no abre ninguna sesion trifasica: entra la expresion tal cual la
 * mando la sede y el DER de cada certificado, y sale que indices pasan. No hay
 * nada que atar entre dos llamadas, asi que <b>no lleva sello</b>; el sello
 * existe para que la postfirma no se desvie de la prefirma, y aqui no hay dos
 * fases.
 *
 * <h2>El {@code nonexpired} implicito, y donde no se hereda</h2>
 *
 * {@code CertFilterManager} anade un filtro que oculta los caducados cuando la
 * sede no declara ninguno, citando la ETSI TS 119 102-1. Ese comportamiento se
 * hereda tal cual, y <b>solo llega hasta donde llega esta llamada</b> (ID-254):
 * el listado local de rFirma no pasa por aqui y sigue enseñando el certificado
 * caducado con su estado.
 */
public final class FilterBridge {

    private FilterBridge() { }

    /**
     * Los certificados que pasan el filtro, por su indice en la lista de
     * entrada.
     *
     * <p>La relacion entre los filtros que devuelve el manager es
     * <b>disyuntiva</b> ({@code CertFilterManager:120}-{@code 127}: «los
     * distintos filtros disyuntivos declarados»), asi que un certificado pasa si
     * lo acepta <b>alguno</b>. La conjuncion de dentro de cada expresion ya la
     * resuelve el propio motor, en el {@code MultipleCertificateFilter} que
     * construye por expresion.
     *
     * @param filterProperties las claves {@code filter=} / {@code filters=} /
     *                         {@code filters.N=} tal y como vinieron, sin
     *                         reinterpretar (ID-256).
     * @param certificates     los certificados a acotar, en su orden.
     * @return los indices que pasan, en orden ascendente.
     */
    public static int[] select(final Properties filterProperties,
            final List<X509Certificate> certificates) {
        final List<CertificateFilter> engine =
                new CertFilterManager(filterProperties).getFilters();

        final List<Integer> selected = new ArrayList<>();
        for (int i = 0; i < certificates.size(); i++) {
            final X509Certificate certificate = certificates.get(i);
            for (final CertificateFilter filter : engine) {
                if (filter.matches(certificate)) {
                    selected.add(Integer.valueOf(i));
                    break;
                }
            }
        }

        final int[] indexes = new int[selected.size()];
        for (int i = 0; i < indexes.length; i++) {
            indexes[i] = selected.get(i).intValue();
        }
        return indexes;
    }

    /**
     * Los certificados de la peticion: Base64 del DER separados por {@code ';'},
     * la misma convencion que la cadena de la prefirma.
     *
     * <p>A diferencia de {@link PadesBridge#parseCertificates(String)}, una
     * lista <b>vacia</b> es una entrada valida: filtrar cero certificados da
     * cero certificados, no un error.
     */
    public static List<X509Certificate> parseCertificates(final String certificatesB64)
            throws Exception {
        final CertificateFactory cf = CertificateFactory.getInstance("X.509");
        final List<X509Certificate> certificates = new ArrayList<>();
        if (certificatesB64 == null) {
            return certificates;
        }
        for (final String b64 : certificatesB64.split(";")) {
            if (b64.isBlank()) {
                continue;
            }
            certificates.add((X509Certificate) cf.generateCertificate(
                    new ByteArrayInputStream(Base64.getDecoder().decode(b64.trim()))));
        }
        return certificates;
    }
}
