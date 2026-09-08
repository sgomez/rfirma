package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.charset.StandardCharsets;
import java.util.Base64;
import java.util.Properties;

import org.junit.jupiter.api.Test;

/**
 * Grada A: la prefirma XAdES no necesita nada mas que el kit de pruebas
 * versionado y el XML de referencia.
 */
class XadesPreSignTest {

    private static XadesBridge.PreSignResult preSign(final Properties params) throws Exception {
        return XadesCycle.preSign(XadesCycle.referenceXml(), params, "sign");
    }

    private static String soleValueOf(final XadesBridge.PreSignResult result) {
        assertEquals(1, result.pres().size(), "una firma XAdES prefirma una sola vez");
        return result.pres().get(0).pre();
    }

    @Test
    void returns_the_canonicalized_signed_info_as_the_block_rust_signs() throws Exception {
        final String pre = new String(Base64.getDecoder().decode(soleValueOf(preSign(
                new Properties()))), StandardCharsets.UTF_8);

        assertTrue(pre.contains("SignedInfo"), "el PRE de XAdES es el SignedInfo: " + pre);
        assertTrue(pre.contains("SignatureMethod"), "al SignedInfo le falta el algoritmo: " + pre);
    }

    @Test
    void leaves_the_base_and_its_encoding_in_the_session() throws Exception {
        final String session = preSign(new Properties()).session();

        assertTrue(session.contains("<param n=\"BASE\">"),
                "la postfirma reinyecta el BASE, asi que la sesion tiene que llevarlo");
        assertEquals("UTF-8", encodingOf(session),
                "la postfirma descodifica el BASE con el ENCODING de la sesion");
        assertTrue(session.contains("<param n=\"NEED_PRE\">"),
                "la sesion XAdES lleva NEED_PRE");
    }

    @Test
    void seals_the_base_that_the_session_carries() throws Exception {
        final XadesBridge.PreSignResult result = preSign(new Properties());

        final SessionStamp stamp = SessionStamp.decode(result.stamp());
        assertNotNull(stamp.xmlBaseDigest(), "el sello de una prefirma XAdES cubre el BASE");
        assertTrue(stamp.matchesXmlBase(baseOf(result.session()), encodingOf(result.session())),
                "el BASE sellado tiene que ser el que viaja en la sesion");
    }

    @Test
    void seals_the_encoding_that_the_session_carries() throws Exception {
        final XadesBridge.PreSignResult result = preSign(new Properties());
        final String base = baseOf(result.session());

        final SessionStamp stamp = SessionStamp.decode(result.stamp());
        assertTrue(stamp.matchesXmlBase(base, encodingOf(result.session())),
                "el ENCODING sellado tiene que ser el que viaja en la sesion");
        assertFalse(stamp.matchesXmlBase(base, "ISO-8859-1"),
                "un ENCODING distinto no casa con el sellado");
        assertFalse(stamp.matchesXmlBase(base, null),
                "una sesion sin ENCODING no casa con un sello que lo lleva");
    }

    @Test
    void seals_the_time_and_the_operation_that_the_session_carries() throws Exception {
        final XadesBridge.PreSignResult result = preSign(new Properties());

        final SessionStamp stamp = SessionStamp.decode(result.stamp());
        assertEquals(XadesCycle.ALGORITHM, stamp.algorithm());
        assertEquals("sign", stamp.operation());
        assertNull(stamp.target(), "una firma no tiene objetivo de contrafirma");
        assertTrue(result.session().contains("<param n=\"TIME\">" + stamp.time() + "</param>"),
                "el TIME del sello tiene que ser el mismo que el de la sesion");
        assertTrue(result.session().contains("<param n=\"OP\">sign</param>"),
                "la operacion sellada tiene que viajar tambien en la sesion");
    }

    @Test
    void seals_the_enveloping_variant_even_when_nobody_asked_for_it() throws Exception {
        final SessionStamp stamp = SessionStamp.decode(preSign(new Properties()).stamp());

        assertEquals("XAdES Enveloping", stamp.extraParams().getProperty("format"),
                "la variante que se firmo se sella, no se deduce en la postfirma");
    }

    @Test
    void does_not_touch_the_properties_of_the_caller() throws Exception {
        final Properties sent = new Properties();

        preSign(sent);

        assertNull(sent.getProperty("format"), "los extraParams del llamante no se tocan");
    }

    @Test
    void refuses_a_cosign_naming_what_is_missing() throws Exception {
        final Exception failure = assertThrows(IllegalArgumentException.class,
                () -> preSign(new Properties(), "cosign"));

        assertTrue(failure.getMessage().contains("cosign"), failure.getMessage());
        final String json = NativeBridge.errorJson(failure);
        assertTrue(json.startsWith("{\"ok\":false"), json);
        assertTrue(json.contains("cosign"), json);
    }

    @Test
    void refuses_a_countersign_naming_what_is_missing() throws Exception {
        final Exception failure = assertThrows(IllegalArgumentException.class,
                () -> preSign(new Properties(), "countersign"));

        assertTrue(failure.getMessage().contains("countersign"), failure.getMessage());
    }

    @Test
    void refuses_an_operation_it_does_not_know() throws Exception {
        final Exception failure = assertThrows(IllegalArgumentException.class,
                () -> preSign(new Properties(), "encrypt"));

        assertTrue(failure.getMessage().contains("encrypt"), failure.getMessage());
    }

    @Test
    void refuses_a_xades_variant_that_is_not_enveloping_yet() throws Exception {
        final Properties detached = new Properties();
        detached.setProperty("format", "XAdES Detached");

        final Exception failure =
                assertThrows(IllegalArgumentException.class, () -> preSign(detached));

        assertTrue(failure.getMessage().contains("XAdES Detached"), failure.getMessage());
        final String json = NativeBridge.errorJson(failure);
        assertTrue(json.startsWith("{\"ok\":false"), json);
        assertTrue(json.contains("XAdES Detached"), json);
    }

    @Test
    void accepts_the_bare_xades_format_the_protocol_sends() throws Exception {
        final Properties bare = new Properties();
        bare.setProperty("format", "XAdES");

        assertEquals(1, preSign(bare).pres().size(), "format=XAdES es la variante por defecto");
    }

    private static XadesBridge.PreSignResult preSign(final Properties params,
            final String operation) throws Exception {
        return XadesCycle.preSign(XadesCycle.referenceXml(), params, operation);
    }

    static String baseOf(final String session) {
        return between(session, "<param n=\"BASE\">", "</param>");
    }

    static String encodingOf(final String session) {
        return between(session, "<param n=\"ENCODING\">", "</param>");
    }

    private static String between(final String session, final String open, final String close) {
        final int from = session.indexOf(open);
        if (from < 0) {
            return null;
        }
        return session.substring(from + open.length(), session.indexOf(close, from));
    }
}
