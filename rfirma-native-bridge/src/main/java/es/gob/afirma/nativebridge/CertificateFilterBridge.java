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
 * SONDEO #314 (rama desechable, no es codigo de produccion).
 *
 * <p>Este fichero existe solo para que el motor de filtros del original sea
 * <b>alcanzable</b> desde un punto de entrada de la imagen nativa. Anadir la
 * dependencia al {@code pom.xml} y no llamarla no mide nada: {@code
 * native-image} solo compila lo que alcanza.
 *
 * <p>La frontera que se prueba es la del enunciado del sondeo: entran N
 * certificados en DER (Base64, separados por {@code ';'}) mas la cadena
 * {@code filters=} tal cual la manda la sede, y sale que indices pasan.
 */
final class CertificateFilterBridge {

    private CertificateFilterBridge() { }

    /** Certificados que pasan el filtro, por indice en la lista de entrada. */
    static int[] select(final String certChainB64, final String filterParams) throws Exception {
        final List<X509Certificate> certificates = parseCertificates(certChainB64);

        final Properties properties = new Properties();
        properties.load(new java.io.StringReader(filterParams));

        final CertFilterManager manager = new CertFilterManager(properties);
        final List<CertificateFilter> filters = manager.getFilters();

        // La lista que devuelve getFilters() es DISYUNTIVA: pasa el certificado
        // que satisfaga CUALQUIERA de sus elementos. Lo conjuntivo esta dentro
        // de cada elemento, porque un `filters=a:x;b:y` se compila en un
        // MultipleCertificateFilter. Asi lo aplica el original en
        // KeyStoreUtilities.getAliasesByFriendlyName:266-276, que va uniendo en
        // una tabla los alias que devuelve cada filtro.
        final List<Integer> passing = new ArrayList<>();
        for (int i = 0; i < certificates.size(); i++) {
            boolean matches = true;
            if (filters != null && !filters.isEmpty()) {
                matches = false;
                for (final CertificateFilter filter : filters) {
                    if (filter.matches(certificates.get(i))) {
                        matches = true;
                        break;
                    }
                }
            }
            if (matches) {
                passing.add(Integer.valueOf(i));
            }
        }

        final int[] result = new int[passing.size()];
        for (int i = 0; i < result.length; i++) {
            result[i] = passing.get(i).intValue();
        }
        return result;
    }

    private static List<X509Certificate> parseCertificates(final String certChainB64) throws Exception {
        final CertificateFactory factory = CertificateFactory.getInstance("X.509");
        final List<X509Certificate> certificates = new ArrayList<>();
        if (certChainB64 == null || certChainB64.isEmpty()) {
            return certificates;
        }
        for (final String part : certChainB64.split(";")) {
            if (part.isEmpty()) {
                continue;
            }
            certificates.add((X509Certificate) factory.generateCertificate(
                    new ByteArrayInputStream(Base64.getDecoder().decode(part))));
        }
        return certificates;
    }
}
