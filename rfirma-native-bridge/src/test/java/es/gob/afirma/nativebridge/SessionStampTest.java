package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.charset.StandardCharsets;
import java.util.Properties;
import java.util.TimeZone;

import org.junit.jupiter.api.Test;

/** Grada A: el sello de sesion no necesita nada para probarse. */
class SessionStampTest {

    private static final TimeZone MADRID = TimeZone.getTimeZone("Europe/Madrid");

    private static final byte[] PDF = "%PDF-1.7 un documento".getBytes(StandardCharsets.UTF_8);

    private static Properties params(final String... keysAndValues) {
        final Properties p = new Properties();
        for (int i = 0; i < keysAndValues.length; i += 2) {
            p.setProperty(keysAndValues[i], keysAndValues[i + 1]);
        }
        return p;
    }

    @Test
    void round_trips_algorithm_time_zone_and_effective_params() {
        final SessionStamp stamp = SessionStamp.of("SHA256withRSA", "1788186680218", MADRID,
                params("signatureSubFilter", "ETSI.CAdES.detached", "profile", "baseline"), PDF);

        final SessionStamp decoded = SessionStamp.decode(stamp.encode());

        assertEquals("SHA256withRSA", decoded.algorithm());
        assertEquals("1788186680218", decoded.time());
        assertEquals("Europe/Madrid", decoded.timeZone().getID());
        assertEquals("ETSI.CAdES.detached", decoded.extraParams().getProperty("signatureSubFilter"));
        assertEquals("baseline", decoded.extraParams().getProperty("profile"));
    }

    @Test
    void encodes_the_same_content_to_the_same_bytes() {
        // Un Properties no tiene orden, asi que sin ordenar las claves dos
        // sellos del mismo contenido saldrian distintos y la comparacion de
        // bytes del ADR-0016 daria falsos negativos.
        final SessionStamp first = SessionStamp.of("SHA256withRSA", "42", MADRID,
                params("a", "1", "b", "2", "c", "3"), PDF);
        final SessionStamp second = SessionStamp.of("SHA256withRSA", "42", MADRID,
                params("c", "3", "b", "2", "a", "1"), PDF);

        assertEquals(first.encode(), second.encode());
    }

    @Test
    void survives_values_with_line_breaks() {
        // El texto de una rubrica lleva saltos de linea, y sin escaparlos
        // partirian el bloque en campos inventados.
        final String layer2Text = "Firmado por\nEIDAS CERTIFICADO PRUEBAS\r\nel 2026-08-31";
        final SessionStamp stamp = SessionStamp.of("SHA256withRSA", "42", MADRID,
                params("layer2Text", layer2Text), PDF);

        assertEquals(layer2Text,
                SessionStamp.decode(stamp.encode()).extraParams().getProperty("layer2Text"));
    }

    @Test
    void detects_a_time_that_does_not_match_the_session() {
        final SessionStamp stamp = SessionStamp.of("SHA256withRSA", "1788186680218", MADRID,
                new Properties(), PDF);

        assertTrue(stamp.matchesSessionTime("1788186680218"));
        assertFalse(stamp.matchesSessionTime("1788186740218"));
        assertFalse(stamp.matchesSessionTime(null));
    }

    @Test
    void detects_a_pdf_that_is_not_the_one_that_was_presigned() {
        // El PDF viaja aparte hasta la postfirma igual que el TIME, y la postfirma
        // PAdES lo regenera entero: si no es el mismo, completa sin error y el PDF
        // sale con "Digest Mismatch".
        final SessionStamp stamp = SessionStamp.of("SHA256withRSA", "42", MADRID,
                new Properties(), PDF);

        assertTrue(stamp.matchesPdf(PDF));
        assertTrue(stamp.matchesPdf("%PDF-1.7 un documento".getBytes(StandardCharsets.UTF_8)),
                "el sello guarda el contenido, no la identidad del array");
        assertFalse(stamp.matchesPdf("%PDF-1.7 otro documento".getBytes(StandardCharsets.UTF_8)));
        assertFalse(stamp.matchesPdf(new byte[0]));
        assertFalse(stamp.matchesPdf(null));
    }

    @Test
    void a_different_pdf_produces_a_different_stamp() {
        final SessionStamp one = SessionStamp.of("SHA256withRSA", "42", MADRID,
                new Properties(), PDF);
        final SessionStamp other = SessionStamp.of("SHA256withRSA", "42", MADRID,
                new Properties(), "%PDF-1.7 otro".getBytes(StandardCharsets.UTF_8));

        assertNotEquals(one.encode(), other.encode());
    }

    @Test
    void a_different_time_zone_produces_a_different_stamp() {
        // El desfase horario entra dentro del rango firmado (#23): dos sellos que
        // solo se diferencian en la zona TIENEN que ser distintos.
        final SessionStamp madrid = SessionStamp.of("SHA256withRSA", "42", MADRID, new Properties(), PDF);
        final SessionStamp utc = SessionStamp.of("SHA256withRSA", "42",
                TimeZone.getTimeZone("UTC"), new Properties(), PDF);

        assertNotEquals(madrid.encode(), utc.encode());
    }

    @Test
    void does_not_let_the_caller_mutate_what_was_sealed() {
        final Properties sent = params("profile", "baseline");
        final SessionStamp stamp = SessionStamp.of("SHA256withRSA", "42", MADRID, sent, PDF);

        sent.setProperty("profile", "otro");
        stamp.extraParams().setProperty("profile", "otro-mas");

        assertEquals("baseline", stamp.extraParams().getProperty("profile"));
    }

    @Test
    void rejects_a_stamp_it_did_not_write() {
        assertThrows(IllegalArgumentException.class, () -> SessionStamp.decode(null));
        assertThrows(IllegalArgumentException.class, () -> SessionStamp.decode(""));
        assertThrows(IllegalArgumentException.class, () -> SessionStamp.decode("no es base64 ***"));
        assertThrows(IllegalArgumentException.class, () -> SessionStamp.decode(
                java.util.Base64.getEncoder().encodeToString("otra-cosa/9\nALG=x\n".getBytes(
                        java.nio.charset.StandardCharsets.UTF_8))));
    }

    @Test
    void rejects_a_stamp_missing_a_mandatory_field() {
        // Sin TZ ni PDF: un sello de antes de que el PDF se sellara no vale.
        final String block = "rfirma-session-stamp/1\nALG=SHA256withRSA\nTIME=42\n";
        assertThrows(IllegalArgumentException.class, () -> SessionStamp.decode(
                java.util.Base64.getEncoder().encodeToString(
                        block.getBytes(java.nio.charset.StandardCharsets.UTF_8))));
    }

    @Test
    void rejects_a_stamp_that_does_not_seal_the_pdf() {
        // Un sello sin PDF es exactamente el de antes de esta comprobacion: se
        // rechaza en vez de dejar pasar la postfirma sin atar el documento.
        final String block = "rfirma-session-stamp/1\nALG=SHA256withRSA\nTIME=42\nTZ=UTC\n";
        assertThrows(IllegalArgumentException.class, () -> SessionStamp.decode(
                java.util.Base64.getEncoder().encodeToString(
                        block.getBytes(StandardCharsets.UTF_8))));
    }

    @Test
    void reads_the_extra_params_the_caller_sends_as_a_properties_block() {
        final Properties parsed = SessionStamp.parseParams("profile=baseline\nsignReason=prueba\n");

        assertEquals("baseline", parsed.getProperty("profile"));
        assertEquals("prueba", parsed.getProperty("signReason"));
        assertTrue(SessionStamp.parseParams("").isEmpty());
        assertTrue(SessionStamp.parseParams(null).isEmpty());
    }
}
