package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.charset.StandardCharsets;
import java.security.Signature;
import java.security.cert.X509Certificate;
import java.util.Arrays;
import java.util.Base64;
import java.util.List;
import java.util.Properties;
import java.util.TimeZone;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;

import org.junit.jupiter.api.Test;

/**
 * Grada A: la postfirma ensambla el PDF sin necesitar nada externo. Que el PDF
 * resultante sea <b>valido</b> es otra pregunta, y la contesta {@code pdfsig} en
 * la grada C ({@link PadesFullCycleTest}).
 */
class PadesPostSignTest {

    private static final String ALGORITHM = "SHA256withRSA";

    /** Prefirma + fase 2 con la clave del kit, que es lo que hara Rust por PKCS#11. */
    private static SignedSession sign(final Properties extraParams) throws Exception {
        final byte[] pdf = TestFixtures.samplePdf();
        final X509Certificate[] chain = TestFixtures.certificateChain();
        final PadesBridge.PreSignResult pre =
                PadesBridge.preSign(pdf, ALGORITHM, chain, extraParams);

        final Signature signature = Signature.getInstance(ALGORITHM);
        signature.initSign(TestFixtures.privateKey());
        signature.update(Base64.getDecoder().decode(pre.preSignB64()));

        return new SignedSession(pdf, chain, pre,
                Base64.getEncoder().encodeToString(signature.sign()));
    }

    private record SignedSession(byte[] pdf, X509Certificate[] chain,
            PadesBridge.PreSignResult pre, String pkcs1) { }

    @Test
    void assembles_a_pdf_when_the_stamp_matches_the_session() throws Exception {
        final SignedSession s = sign(new Properties());

        final byte[] signed = PadesBridge.postSign(
                s.pdf(), s.chain(), s.pre().stamp(), s.pre().session(), s.pkcs1());

        assertArrayEquals("%PDF".getBytes(StandardCharsets.US_ASCII),
                Arrays.copyOf(signed, 4));
        assertTrue(signed.length > s.pdf().length, "el PDF firmado tiene que crecer");
    }

    @Test
    void rejects_a_session_whose_time_does_not_match_the_stamp() throws Exception {
        // El escenario que midio el #14: la postfirma recibe un instante distinto
        // del de la prefirma. AutoFirma NO falla —completa, y el PDF sale con
        // "Digest Mismatch"—, asi que el rechazo tiene que ponerlo el puente.
        final SignedSession s = sign(new Properties());
        final SessionStamp stamp = SessionStamp.decode(s.pre().stamp());
        final String tampered = s.pre().session().replace(
                "<param n=\"TIME\">" + stamp.time() + "</param>",
                "<param n=\"TIME\">" + (Long.parseLong(stamp.time()) + 60000L) + "</param>");
        assertTrue(!tampered.equals(s.pre().session()), "la sesion no se ha llegado a alterar");

        final PadesBridge.SessionStampMismatchException failure = assertThrows(
                PadesBridge.SessionStampMismatchException.class,
                () -> PadesBridge.postSign(s.pdf(), s.chain(), s.pre().stamp(), tampered, s.pkcs1()));

        assertTrue(failure.getMessage().contains("Digest Mismatch"),
                "el mensaje tiene que decir que se estaba evitando: " + failure.getMessage());
    }

    @Test
    void rejects_a_stamp_from_another_presign() throws Exception {
        final SignedSession first = sign(new Properties());
        final SignedSession second = sign(new Properties());

        // Dos prefirmas del mismo PDF solo se distinguen en el instante, que es
        // justo lo que el sello ata.
        assertThrows(PadesBridge.SessionStampMismatchException.class,
                () -> PadesBridge.postSign(first.pdf(), first.chain(),
                        second.pre().stamp(), first.pre().session(), first.pkcs1()));
    }

    @Test
    void rejects_a_pdf_that_is_not_the_one_that_was_presigned() throws Exception {
        // El PDF viaja aparte igual que el TIME y la postfirma PAdES lo regenera
        // entero: postfirmar otro completa sin error y devuelve {"ok":true,...}
        // con una firma que da "Digest Mismatch".
        final SignedSession s = sign(new Properties());
        final byte[] other = Arrays.copyOf(s.pdf(), s.pdf().length);
        // Un byte de la zona de metadatos: sigue siendo un PDF, ya no es EL PDF.
        other[other.length - 1] ^= 0x01;

        final PadesBridge.SessionStampMismatchException failure = assertThrows(
                PadesBridge.SessionStampMismatchException.class,
                () -> PadesBridge.postSign(other, s.chain(),
                        s.pre().stamp(), s.pre().session(), s.pkcs1()));

        assertTrue(failure.getMessage().contains("Digest Mismatch"),
                "el mensaje tiene que decir que se estaba evitando: " + failure.getMessage());
    }

    @Test
    void restores_the_default_time_zone_even_when_postsigns_overlap() throws Exception {
        // TimeZone.setDefault es estado global del proceso: sin serializar, dos
        // postfirmas solapadas se pisan la zona y el finally de una restaura la de
        // la otra. El sintoma seria un PDF con "Digest Mismatch" e irreproducible.
        final SignedSession first = sign(new Properties());
        final SignedSession second = sign(new Properties());
        final TimeZone before = TimeZone.getDefault();

        final ExecutorService pool = Executors.newFixedThreadPool(2);
        try {
            final List<Future<byte[]>> results = pool.invokeAll(List.of(
                    () -> PadesBridge.postSign(first.pdf(), first.chain(),
                            first.pre().stamp(), first.pre().session(), first.pkcs1()),
                    () -> PadesBridge.postSign(second.pdf(), second.chain(),
                            second.pre().stamp(), second.pre().session(), second.pkcs1())));
            for (final Future<byte[]> result : results) {
                assertArrayEquals("%PDF".getBytes(StandardCharsets.US_ASCII),
                        Arrays.copyOf(result.get(), 4));
            }
        }
        finally {
            pool.shutdownNow();
        }

        assertEquals(before.getID(), TimeZone.getDefault().getID(),
                "el proceso se ha quedado con la zona horaria de una postfirma");
    }

    @Test
    void refuses_to_sign_without_the_pkcs1_of_phase_two() throws Exception {
        final SignedSession s = sign(new Properties());

        assertThrows(IllegalArgumentException.class,
                () -> PadesBridge.postSign(s.pdf(), s.chain(),
                        s.pre().stamp(), s.pre().session(), ""));
    }

    @Test
    void imposes_the_extra_params_of_the_stamp_instead_of_asking_for_them_again() throws Exception {
        // La firma del metodo no tiene extraParams: no hay manera de pasarle unos
        // distintos de los de la prefirma, que es la mitad del ADR-0016 que no se
        // comprueba sino que se hace imposible.
        final Properties sent = new Properties();
        sent.setProperty("profile", "baseline");
        final SignedSession s = sign(sent);

        final byte[] signed = PadesBridge.postSign(
                s.pdf(), s.chain(), s.pre().stamp(), s.pre().session(), s.pkcs1());

        assertEquals("ETSI.CAdES.detached",
                SessionStamp.decode(s.pre().stamp()).extraParams()
                        .getProperty("signatureSubFilter"));
        assertArrayEquals("%PDF".getBytes(StandardCharsets.US_ASCII),
                Arrays.copyOf(signed, 4));
    }
}
