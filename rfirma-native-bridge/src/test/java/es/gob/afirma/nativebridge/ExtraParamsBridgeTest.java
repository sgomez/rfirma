package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.Properties;

import org.junit.jupiter.api.Test;

import es.gob.afirma.core.signers.ExtraParamsProcessor;

/**
 * Grada A: el expansor de politica, sin token ni imagen nativa.
 *
 * <p>Lo que se comprueba no es <b>en que</b> expande —eso es del original y
 * cambiaria con el— sino que se expande con el original y que el fallo llega
 * entero.
 */
class ExtraParamsBridgeTest {

    private static Properties params(final String... pairs) {
        final Properties properties = new Properties();
        for (int i = 0; i < pairs.length; i += 2) {
            properties.setProperty(pairs[i], pairs[i + 1]);
        }
        return properties;
    }

    @Test
    void expands_the_age_policy_into_the_keys_the_original_writes() throws Exception {
        final String block = ExtraParamsBridge.expand(params("expPolicy", "FirmaAGE"), "PAdES");

        assertFalse(block.contains("expPolicy="),
                "la clave expandible se consume: " + block);
        assertTrue(block.contains("policyIdentifier="),
                "la politica de la AGE trae identificador: " + block);
        assertTrue(block.contains("signatureSubFilter=ETSI.CAdES.detached"),
                "en PAdES con politica de la AGE el subfiltro es el de la ETSI: " + block);
    }

    @Test
    void leaves_alone_what_declares_no_policy() throws Exception {
        final String block = ExtraParamsBridge.expand(params("signReason", "Conforme"), "PAdES");

        assertEquals("signReason=Conforme\n", block);
    }

    /**
     * Una politica que no se puede aplicar no se ignora: firmar sin ella seria
     * firmar algo distinto de lo que la sede declaro.
     */
    @Test
    void refuses_a_policy_that_does_not_fit_the_format() {
        assertThrows(ExtraParamsProcessor.IncompatiblePolicyException.class,
                () -> ExtraParamsBridge.expand(params("expPolicy", "PoliticaInventada"), "PAdES"));
    }

    /** Y un subfiltro que la politica de la AGE no admite tampoco pasa. */
    @Test
    void refuses_a_subfilter_the_age_policy_does_not_allow() {
        assertThrows(ExtraParamsProcessor.IncompatiblePolicyException.class,
                () -> ExtraParamsBridge.expand(
                        params("expPolicy", "FirmaAGE", "signatureSubFilter", "adbe.pkcs7.detached"),
                        "PAdES"));
    }

    /**
     * El bloque que sale lo lee el lado de Rust con el mismo lector que el
     * {@code properties} de la sede, asi que los separadores de una clave van
     * escapados y el orden es estable.
     */
    @Test
    void writes_a_block_the_rust_side_can_read_back() {
        final String block = ExtraParamsBridge.write(
                params("b", "dos", "a", "uno", "con=igual", "tres"));

        assertEquals("a=uno\nb=dos\ncon\\=igual=tres\n", block);
    }
}
