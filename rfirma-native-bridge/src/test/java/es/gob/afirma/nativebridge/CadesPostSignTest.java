package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.security.Signature;
import java.security.cert.X509Certificate;
import java.util.Arrays;
import java.util.Base64;
import java.util.Properties;

import org.junit.jupiter.api.Test;

/**
 * Grada A: la postfirma CAdES ensambla el CMS sin necesitar nada externo. Que
 * ese CMS <b>valide</b> es otra pregunta, y la contesta {@link CadesFullCycleTest}.
 */
class CadesPostSignTest {

    private static final String ALGORITHM = "SHA256withRSA";

    /** Prefirma + fase 2 con la clave del kit, que es lo que hara Rust por PKCS#11. */
    private static SignedSession sign(final Properties extraParams) throws Exception {
        final byte[] document = TestFixtures.challenge();
        final X509Certificate[] chain = TestFixtures.certificateChain();
        final CadesBridge.PreSignResult pre =
                CadesBridge.preSign(document, ALGORITHM, chain, extraParams, "sign");

        final Signature signature = Signature.getInstance(ALGORITHM);
        signature.initSign(TestFixtures.privateKey());
        signature.update(Base64.getDecoder().decode(pre.preSignB64()));

        return new SignedSession(document, chain, pre,
                Base64.getEncoder().encodeToString(signature.sign()));
    }

    private record SignedSession(byte[] document, X509Certificate[] chain,
            CadesBridge.PreSignResult pre, String pkcs1) { }

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
                        s.pre().stamp(), s.pre().session(), ""));
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
