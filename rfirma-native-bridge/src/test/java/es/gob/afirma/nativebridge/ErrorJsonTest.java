package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;

import org.junit.jupiter.api.Test;

import es.gob.afirma.signers.pades.common.PdfHasUnregisteredSignaturesException;

/**
 * Grada A: <b>el puente distingue el PDF con firmas no registradas de un fallo
 * cualquiera</b> (ID-296).
 *
 * <p>Toda excepcion salia de aqui como el mismo {@code {"ok":false}}, asi que
 * al otro lado de la frontera todas eran {@code bridgeFailed} y no habia forma
 * de decidir el codigo que la sede recibe. La clase de fallo viaja ahora en
 * {@code kind}, y una sola se distingue: la que no es un fallo sino algo que
 * hay que confirmar.
 */
class ErrorJsonTest {

    @Test
    void a_pdf_with_unregistered_signatures_travels_with_its_own_kind() {
        final String json = NativeBridge.errorJson(
                new PdfHasUnregisteredSignaturesException("el PDF trae firmas no registradas"));

        assertTrue(json.contains("\"kind\":\"pdfHasUnregisteredSignatures\""), json);
        assertTrue(json.startsWith("{\"ok\":false"), json);
    }

    /** AutoFirma envuelve sus excepciones antes de que lleguen al puente. */
    @Test
    void the_kind_is_found_through_the_causes() {
        final Throwable wrapped = new IllegalStateException("no se ha podido prefirmar",
                new PdfHasUnregisteredSignaturesException("firmas no registradas"));

        assertEquals(NativeBridge.UNREGISTERED_SIGNATURES_KIND, NativeBridge.kindOf(wrapped));
    }

    @Test
    void anything_else_is_a_plain_failure() {
        assertEquals(NativeBridge.GENERIC_FAILURE_KIND,
                NativeBridge.kindOf(new IOException("el fichero no es un PDF")));
    }

    /** Y el mensaje de Java sigue viajando crudo, sin traducir ni recortar. */
    @Test
    void the_raw_java_message_still_travels_next_to_the_kind() {
        final String json = NativeBridge.errorJson(new IOException("no es un PDF"));

        assertTrue(json.contains("\"kind\":\"failed\""), json);
        assertTrue(json.contains("java.io.IOException: no es un PDF"), json);
    }
}
