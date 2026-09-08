package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.charset.StandardCharsets;
import java.util.Base64;
import java.util.List;
import java.util.Properties;

import org.junit.jupiter.api.Test;

/**
 * Grada A: lo que la postfirma XAdES <b>rechaza</b>.
 *
 * <p>Cada caso de aqui es uno que sin el sello no fallaria: la postfirma
 * completaria y devolveria un XML firmado que dice cubrir algo que no cubre
 * (ADR-0016).
 */
class XadesPostSignTest {

    private static XadesBridge.PreSignResult preSign() throws Exception {
        return XadesCycle.preSign(XadesCycle.referenceXml(), new Properties(), "sign");
    }

    private static byte[] postSign(final XadesBridge.PreSignResult pre, final byte[] document,
            final String session) throws Exception {
        return XadesBridge.postSign(document, TestFixtures.certificateChain(), pre.stamp(),
                session, XadesCycle.pkcs1For(pre));
    }

    @Test
    void refuses_a_base_changed_between_the_two_phases() throws Exception {
        final XadesBridge.PreSignResult pre = preSign();
        final String base = XadesPreSignTest.baseOf(pre.session());
        final String tamperedBase = Base64.getEncoder().encodeToString(
                (new String(Base64.getDecoder().decode(base), StandardCharsets.UTF_8) + "<!--x-->")
                        .getBytes(StandardCharsets.UTF_8));

        final Exception failure = assertThrows(SessionStampMismatchException.class,
                () -> postSign(pre, XadesCycle.referenceXml(),
                        pre.session().replace(base, tamperedBase)));

        assertTrue(failure.getMessage().contains("BASE"), failure.getMessage());
    }

    @Test
    void refuses_a_document_that_is_not_the_one_that_was_presigned() throws Exception {
        final XadesBridge.PreSignResult pre = preSign();
        final byte[] other = new String(XadesCycle.referenceXml(), StandardCharsets.UTF_8)
                .replace("Contenido", "Otro contenido").getBytes(StandardCharsets.UTF_8);

        assertThrows(SessionStampMismatchException.class,
                () -> postSign(pre, other, pre.session()));
    }

    @Test
    void refuses_a_certificate_chain_that_is_not_the_one_that_presigned() throws Exception {
        final XadesBridge.PreSignResult pre = preSign();

        assertThrows(SessionStampMismatchException.class,
                () -> XadesBridge.postSign(XadesCycle.referenceXml(),
                        TestFixtures.otherCertificateChain(), pre.stamp(), pre.session(),
                        XadesCycle.pkcs1For(pre)));
    }

    @Test
    void refuses_a_session_whose_time_is_not_the_sealed_one() throws Exception {
        final XadesBridge.PreSignResult pre = preSign();
        final SessionStamp stamp = SessionStamp.decode(pre.stamp());

        assertThrows(SessionStampMismatchException.class,
                () -> postSign(pre, XadesCycle.referenceXml(), pre.session().replace(
                        "<param n=\"TIME\">" + stamp.time(), "<param n=\"TIME\">0")));
    }

    @Test
    void refuses_a_session_whose_operation_is_not_the_sealed_one() throws Exception {
        final XadesBridge.PreSignResult pre = preSign();

        assertThrows(SessionStampMismatchException.class,
                () -> postSign(pre, XadesCycle.referenceXml(),
                        pre.session().replace("<param n=\"OP\">sign", "<param n=\"OP\">cosign")));
    }

    @Test
    void refuses_a_pkcs1_that_belongs_to_another_presignature() throws Exception {
        final XadesBridge.PreSignResult pre = preSign();
        final String pkcs1 = XadesCycle.pkcs1For(pre).get(0).pkcs1B64();

        assertThrows(IllegalArgumentException.class,
                () -> XadesBridge.postSign(XadesCycle.referenceXml(),
                        TestFixtures.certificateChain(), pre.stamp(), pre.session(),
                        List.of(new XadesBridge.SignatureValue("otra", pkcs1))));
    }

    @Test
    void refuses_a_missing_pkcs1() throws Exception {
        final XadesBridge.PreSignResult pre = preSign();

        assertThrows(IllegalArgumentException.class,
                () -> XadesBridge.postSign(XadesCycle.referenceXml(),
                        TestFixtures.certificateChain(), pre.stamp(), pre.session(), List.of()));
    }

    @Test
    void refuses_a_stamp_that_does_not_seal_any_base() throws Exception {
        final XadesBridge.PreSignResult pre = preSign();
        final byte[] document = XadesCycle.referenceXml();
        final SessionStamp cadesLike = SessionStamp.of(XadesCycle.ALGORITHM,
                SessionStamp.decode(pre.stamp()).time(), java.util.TimeZone.getDefault(),
                SessionStamp.decode(pre.stamp()).extraParams(), document,
                TestFixtures.certificateChain(), "sign", null);

        assertThrows(SessionStampMismatchException.class,
                () -> XadesBridge.postSign(document, TestFixtures.certificateChain(),
                        cadesLike.encode(), pre.session(), XadesCycle.pkcs1For(pre)));
    }
}
