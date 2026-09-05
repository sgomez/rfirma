package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.security.cert.X509Certificate;
import java.util.Base64;
import java.util.List;
import java.util.Properties;

import org.junit.jupiter.api.Test;

/**
 * Grada A: el motor prestado, con certificados de verdad del kit FNMT y sin
 * imagen nativa.
 *
 * <p>Lo que se comprueba aqui <b>no</b> es que los filtros de AutoFirma
 * funcionen —eso es codigo del original y ya esta probado alli—, sino las tres
 * cosas que rFirma se juega al pedirselos prestados (ID-253): que la
 * composicion <b>Y</b> de dentro de una expresion y la <b>O</b> de entre
 * expresiones numeradas sigan siendo las suyas, y que el {@code nonexpired}
 * implicito de la ETSI se herede tal cual.
 */
class FilterBridgeTest {

    /** Un trozo del subject de los tres certificados del kit. */
    private static final String IN_THE_SUBJECT = "EIDAS CERTIFICADO PRUEBAS";

    private static Properties filters(final String... lines) {
        final Properties properties = new Properties();
        for (final String line : lines) {
            final int equals = line.indexOf('=');
            properties.setProperty(line.substring(0, equals), line.substring(equals + 1));
        }
        return properties;
    }

    @Test
    void the_expression_reaches_the_engine_and_bounds_the_listing() throws Exception {
        final List<X509Certificate> listing = List.of(TestFixtures.activeCertificate());

        assertArrayEquals(new int[] { 0 },
                FilterBridge.select(filters("filters=subject.contains:" + IN_THE_SUBJECT), listing));
        assertArrayEquals(new int[] {},
                FilterBridge.select(filters("filters=subject.contains:NO ESTA EN EL SUBJECT"), listing),
                "un listado vacio significa «la sede los excluyo» (ID-258)");
    }

    /**
     * Las tres formas del original tienen la misma precedencia que alli
     * ({@code CertFilterManager:165}-{@code 182}): {@code filter}, luego
     * {@code filters}, luego {@code filters.N}.
     */
    @Test
    void the_three_spellings_of_the_key_are_all_understood() throws Exception {
        final List<X509Certificate> listing = List.of(TestFixtures.activeCertificate());

        for (final String key : new String[] { "filter", "filters", "filters.1" }) {
            assertArrayEquals(new int[] { 0 },
                    FilterBridge.select(
                            filters(key + "=subject.contains:" + IN_THE_SUBJECT), listing),
                    "con la clave " + key);
        }
    }

    @Test
    void a_semicolon_inside_one_expression_is_an_AND() throws Exception {
        final List<X509Certificate> listing = List.of(TestFixtures.activeCertificate());

        assertArrayEquals(new int[] {},
                FilterBridge.select(
                        filters("filters=subject.contains:" + IN_THE_SUBJECT
                                + ";subject.contains:NO ESTA"),
                        listing),
                "dentro de una expresion los criterios se cumplen TODOS");
    }

    @Test
    void the_numbered_expressions_are_an_OR() throws Exception {
        final List<X509Certificate> listing = List.of(TestFixtures.activeCertificate());

        assertArrayEquals(new int[] { 0 },
                FilterBridge.select(
                        filters("filters.1=subject.contains:NO ESTA",
                                "filters.2=subject.contains:" + IN_THE_SUBJECT),
                        listing),
                "entre expresiones numeradas basta con que se cumpla UNA");
    }

    /**
     * ID-254, la mitad que si se hereda: cuando la sede no declara ningun
     * filtro, el motor añade el {@code nonexpired} de la ETSI TS 119 102-1
     * ({@code CertFilterManager:129}-{@code 136}). La otra mitad —que el listado
     * local siga enseñando el caducado— no se puede comprobar aqui, porque el
     * listado local no llama a este puente; la vigila
     * {@code app/certificates.rs} en Rust.
     */
    @Test
    void with_no_filter_at_all_the_engine_hides_the_expired_one() throws Exception {
        final List<X509Certificate> listing =
                List.of(TestFixtures.activeCertificate(), TestFixtures.expiredCertificate());

        assertArrayEquals(new int[] { 0 }, FilterBridge.select(new Properties(), listing));
    }

    /**
     * Y una sede que pide ver los caducados los ve. <b>El valor va al reves de
     * lo que el nombre sugiere</b>: {@code nonexpired:false} los <b>muestra</b>
     * y {@code nonexpired:true} los oculta, porque el original construye
     * {@code new ExpiredCertificateFilter(!parseBoolean(valor))}
     * ({@code CertFilterManager:216}-{@code 219}). Es exactamente la clase de
     * regla que una reimplementacion en Rust habria invertido (ID-253).
     */
    @Test
    void a_site_that_asks_for_the_expired_ones_gets_them() throws Exception {
        final List<X509Certificate> listing =
                List.of(TestFixtures.activeCertificate(), TestFixtures.expiredCertificate());

        assertArrayEquals(new int[] { 0, 1 },
                FilterBridge.select(filters("filters=nonexpired:false"), listing));
        assertArrayEquals(new int[] { 0 },
                FilterBridge.select(filters("filters=nonexpired:true"), listing));
    }

    @Test
    void filtering_nothing_gives_nothing_instead_of_failing() throws Exception {
        assertArrayEquals(new int[] {}, FilterBridge.select(new Properties(), List.of()));
        assertTrue(FilterBridge.parseCertificates("").isEmpty());
        assertTrue(FilterBridge.parseCertificates(null).isEmpty());
    }

    @Test
    void the_certificates_arrive_as_base64_der_separated_by_semicolons() throws Exception {
        final X509Certificate active = TestFixtures.activeCertificate();
        final X509Certificate expired = TestFixtures.expiredCertificate();
        final String payload = Base64.getEncoder().encodeToString(active.getEncoded())
                + ";" + Base64.getEncoder().encodeToString(expired.getEncoded());

        final List<X509Certificate> parsed = FilterBridge.parseCertificates(payload);

        assertEquals(2, parsed.size());
        assertEquals(active, parsed.get(0));
        assertEquals(expired, parsed.get(1));
    }

    /**
     * ID-257: {@code disableopeningexternalstores} no filtra nada —desactiva una
     * bandera del dialogo del original— y aqui queda satisfecha por
     * construccion, porque rFirma no abre almacenes desde la seleccion. Lo que
     * no puede hacer es dejar el listado vacio.
     */
    @Test
    void the_flag_that_is_not_a_criterion_does_not_empty_the_listing() throws Exception {
        final List<X509Certificate> listing = List.of(TestFixtures.activeCertificate());

        assertArrayEquals(new int[] { 0 },
                FilterBridge.select(filters("filters=disableopeningexternalstores"), listing));
    }

    /**
     * El valor de un criterio con eñe y con tildes —lo normal en un apellido
     * espanol— llega al motor <b>identico</b> a como lo escribio la sede. El
     * bloque viaja en UTF-8 y {@link SessionStamp#parseParams(String)} lo lee
     * con un {@code Reader}: la sobrecarga de {@code Properties.load} que toma
     * un flujo de bytes descodifica ISO-8859-1 por contrato, y convertiria
     * cada letra acentuada en dos caracteres, ninguno el bueno. El motor no
     * casaria con nada, devolveria un listado vacio, y la aplicacion lo
     * contaria como «la sede los excluyo» (ID-258) en vez de como un filtro
     * mutilado. Nada mas se pondria rojo.
     */
    @Test
    void a_value_with_accents_reaches_the_engine_unchanged() throws Exception {
        final String expression = "subject.contains:MU\u00d1OZ P\u00c9REZ";

        final Properties parsed = SessionStamp.parseParams("filters=" + expression + "\n");

        assertEquals(expression, parsed.getProperty("filters"),
                "la expresion cruza literal al motor (ID-256), tambien fuera del ASCII");
        // Y el motor la entiende: ninguno de los del kit lleva ese apellido, asi
        // que el listado sale vacio por el criterio, no por la codificacion.
        assertArrayEquals(new int[] {},
                FilterBridge.select(parsed, List.of(TestFixtures.activeCertificate())));
    }

    /**
     * ID-260: los cuatro criterios que nadie ha podido medir —hace falta un
     * DNIe, un certificado SSL, uno cualificado y uno de seudonimo emitidos de
     * verdad— <b>se aceptan</b>. Lo que se fija aqui es que el motor los
     * entiende y contesta sin reventar; que su veredicto sea el correcto es la
     * cobertura que falta, y esta anotada como tal.
     */
    @Test
    void the_four_unmeasured_criteria_are_accepted_even_without_coverage_of_their_verdict()
            throws Exception {
        final List<X509Certificate> listing =
                List.of(TestFixtures.activeCertificate(), TestFixtures.expiredCertificate());

        for (final String criterion : new String[] {
                "qualified:2.16.724.1.3.5.3.2", "pseudonym:true", "ssl:true", "dnie:true" }) {
            final int[] selected = FilterBridge.select(filters("filters=" + criterion), listing);

            // Con dos certificados el listado ya puede decir algo: lo que se fija
            // es que el motor *criba* —contesta con un subconjunto propio— y no
            // que cribe bien, que es la cobertura que falta.
            assertTrue(selected.length < listing.size(),
                    "el motor entiende " + criterion + " y contesta cribando");
        }
    }
}
