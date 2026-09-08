package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.security.cert.X509Certificate;
import java.util.Arrays;
import java.util.List;
import java.util.Properties;

import org.junit.jupiter.api.Test;

/**
 * Grada A: la postfirma CAdES ensambla el CMS sin necesitar nada externo. Que
 * ese CMS <b>valide</b> es otra pregunta, y la contesta {@link CadesFullCycleTest}.
 */
class CadesPostSignTest {

    /** Prefirma + fase 2 con la clave del kit, que es lo que hara Rust por PKCS#11. */
    private static SignedSession sign(final Properties extraParams) throws Exception {
        final byte[] document = TestFixtures.challenge();
        final CadesBridge.PreSignResult pre = CadesCycle.preSign(document, extraParams, "sign");

        return new SignedSession(document, TestFixtures.certificateChain(), pre,
                CadesCycle.pkcs1For(pre));
    }

    private record SignedSession(byte[] document, X509Certificate[] chain,
            CadesBridge.PreSignResult pre, List<CadesBridge.SignatureValue> pkcs1) { }

    /**
     * Dos prefirmas seguidas solo se distinguen en el instante, asi que la
     * segunda espera al tic del reloj: sin el, los dos sellos serian iguales y la
     * prueba de abajo no estaria comprobando nada.
     */
    private static void waitForTheClockToTick() {
        final long start = System.currentTimeMillis();
        while (System.currentTimeMillis() == start) {
            Thread.onSpinWait();
        }
    }

    @Test
    void assembles_a_cms_when_the_stamp_matches_the_session() throws Exception {
        final SignedSession s = sign(new Properties());

        final byte[] signature = CadesBridge.postSign(
                s.document(), s.chain(), s.pre().stamp(), s.pre().session(), s.pkcs1());

        assertTrue(signature.length > 0, "la postfirma no ha devuelto ningun CMS");
    }

    @Test
    void rejects_a_session_whose_time_does_not_match_the_stamp() throws Exception {
        final SignedSession s = sign(new Properties());
        final SessionStamp stamp = SessionStamp.decode(s.pre().stamp());
        final String tampered = s.pre().session().replace(
                "<param n=\"TIME\">" + stamp.time() + "</param>",
                "<param n=\"TIME\">" + (Long.parseLong(stamp.time()) + 60000L) + "</param>");
        assertTrue(!tampered.equals(s.pre().session()), "la sesion no se ha llegado a alterar");

        final SessionStampMismatchException failure = assertThrows(
                SessionStampMismatchException.class,
                () -> CadesBridge.postSign(s.document(), s.chain(), s.pre().stamp(),
                        tampered, s.pkcs1()));

        assertTrue(failure.getMessage().contains("TIME"),
                "el mensaje tiene que decir que se estaba evitando: " + failure.getMessage());
    }

    @Test
    void rejects_a_stamp_from_another_presign() throws Exception {
        final SignedSession first = sign(new Properties());
        waitForTheClockToTick();
        final SignedSession second = sign(new Properties());

        assertThrows(SessionStampMismatchException.class,
                () -> CadesBridge.postSign(first.document(), first.chain(),
                        second.pre().stamp(), first.pre().session(), first.pkcs1()));
    }

    @Test
    void rejects_a_document_that_is_not_the_one_that_was_presigned() throws Exception {
        // El documento viaja aparte, igual que en PAdES. En modo implicito acaba
        // dentro del CMS, asi que postfirmar otro devuelve una firma cuyo
        // message-digest no es el del contenido que lleva dentro.
        final SignedSession s = sign(new Properties());
        final byte[] other = Arrays.copyOf(s.document(), s.document().length);
        other[other.length - 1] ^= 0x01;

        final SessionStampMismatchException failure = assertThrows(
                SessionStampMismatchException.class,
                () -> CadesBridge.postSign(other, s.chain(),
                        s.pre().stamp(), s.pre().session(), s.pkcs1()));

        assertTrue(failure.getMessage().contains("prefirmo"),
                "el mensaje tiene que decir que se estaba evitando: " + failure.getMessage());
    }

    @Test
    void rejects_a_chain_that_is_not_the_one_that_presigned() throws Exception {
        final SignedSession s = sign(new Properties());
        final X509Certificate[] other = TestFixtures.otherCertificateChain();

        final SessionStampMismatchException failure = assertThrows(
                SessionStampMismatchException.class,
                () -> CadesBridge.postSign(s.document(), other,
                        s.pre().stamp(), s.pre().session(), s.pkcs1()));

        assertTrue(failure.getMessage().contains("quien no lo firmo"),
                "el mensaje tiene que decir que se estaba evitando: " + failure.getMessage());
    }

    @Test
    void refuses_to_sign_without_the_pkcs1_of_phase_two() throws Exception {
        final SignedSession s = sign(new Properties());

        assertThrows(IllegalArgumentException.class,
                () -> CadesBridge.postSign(s.document(), s.chain(),
                        s.pre().stamp(), s.pre().session(), List.of()));
    }

    @Test
    void reads_the_pkcs1_list_that_rust_sends() {
        final List<CadesBridge.SignatureValue> values =
                CadesBridge.parsePkcs1List("[{\"id\":\"a==\",\"pk1\":\"MTIz\"},"
                        + " {\"id\":\"b/c+d\",\"pk1\":\"NDU2\"}]");

        assertEquals(2, values.size());
        assertEquals("a==", values.get(0).id());
        assertEquals("MTIz", values.get(0).pkcs1B64());
        assertEquals("b/c+d", values.get(1).id());
    }

    @Test
    void refuses_a_pkcs1_list_without_a_single_entry() {
        assertThrows(IllegalArgumentException.class, () -> CadesBridge.parsePkcs1List("[]"));
        assertThrows(IllegalArgumentException.class, () -> CadesBridge.parsePkcs1List(null));
        assertThrows(IllegalArgumentException.class, () -> CadesBridge.parsePkcs1List("MTIz"));
    }

    @Test
    void imposes_the_extra_params_of_the_stamp_instead_of_asking_for_them_again() throws Exception {
        // La firma del metodo no tiene extraParams ni algoritmo: no hay manera de
        // pasarle unos distintos de los de la prefirma, que es la mitad del
        // ADR-0016 que no se comprueba sino que se hace imposible. Y aqui pesa: el
        // mode decide si el contenido entra en el CMS.
        final Properties sent = new Properties();
        sent.setProperty("mode", "implicit");
        final SignedSession s = sign(sent);

        final byte[] signature = CadesBridge.postSign(
                s.document(), s.chain(), s.pre().stamp(), s.pre().session(), s.pkcs1());

        assertEquals("implicit",
                SessionStamp.decode(s.pre().stamp()).extraParams().getProperty("mode"));
        assertTrue(signature.length > 0, "la postfirma no ha devuelto ningun CMS");
    }
}
