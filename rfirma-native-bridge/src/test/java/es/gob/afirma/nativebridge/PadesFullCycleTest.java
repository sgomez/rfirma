package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.Signature;
import java.security.cert.X509Certificate;
import java.util.Base64;
import java.util.Properties;
import java.util.concurrent.TimeUnit;

import org.junit.jupiter.api.Tag;
import org.junit.jupiter.api.Test;

/**
 * Grada C (ADR-0014): el ciclo trifasico completo, con {@code pdfsig} de poppler
 * como <b>puerta automatica de validez</b>. El carril rapido la compila pero no
 * la ejecuta —no instala poppler—; la ejecuta el carril lento con
 * {@code just test-native}.
 *
 * <p>Sin rubrica: la firma visible se comprueba <b>rasterizando</b>, porque
 * {@code pdftotext} no la ve y da un falso negativo (TD-03), y eso llega con el
 * sub-issue de la rubrica.
 *
 * <p>La fase 2 la hace aqui la JCE con la clave del kit FNMT. En la aplicacion
 * la hara Rust contra el PKCS#11 del sistema y la clave privada no entrara nunca
 * en Java (ADR-0001); lo que esta prueba fija es el <b>contrato</b>: un PKCS#1
 * sobre los bytes DER de la prefirma, sin mas envoltorio.
 */
@Tag("gradaC")
class PadesFullCycleTest {

    private static final String ALGORITHM = "SHA256withRSA";

    @Test
    void signs_a_pdf_without_rubric_and_pdfsig_validates_it() throws Exception {
        final byte[] pdf = TestFixtures.samplePdf();
        final X509Certificate[] chain = TestFixtures.certificateChain();

        // Fase 1: prefirma en Java.
        final PadesBridge.PreSignResult pre =
                PadesBridge.preSign(pdf, ALGORITHM, chain, new Properties());

        // Fase 2: el PKCS#1 sobre los atributos firmados. Aqui, la JCE; en la
        // aplicacion, PKCS#11 desde Rust.
        final Signature signature = Signature.getInstance(ALGORITHM);
        signature.initSign(TestFixtures.privateKey());
        signature.update(Base64.getDecoder().decode(pre.preSignB64()));
        final String pkcs1 = Base64.getEncoder().encodeToString(signature.sign());

        // Fase 3: postfirma en Java.
        final byte[] signed = PadesBridge.postSign(pdf, chain, pre.stamp(), pre.session(), pkcs1);

        final Path out = Path.of("target", "grada-c-sin-rubrica.pdf");
        Files.createDirectories(out.getParent());
        Files.write(out, signed);

        final String report = pdfsig(out);
        assertTrue(report.contains("Signature is Valid"),
                "pdfsig no da la firma por valida:\n" + report);
        assertTrue(report.contains("Total document signed"),
                "el rango firmado no cubre el documento entero:\n" + report);
    }

    /**
     * {@code pdfsig} sobre el PDF firmado.
     *
     * <p>Solo se mira la validez de la <b>firma</b>. La del certificado no: la CA
     * de pruebas de la FNMT no esta en el almacen del sistema, asi que
     * {@code pdfsig} la dara siempre por no verificada y afirmar lo contrario
     * seria una prueba que no prueba nada.
     */
    private static String pdfsig(final Path pdf) throws Exception {
        final ProcessBuilder builder = new ProcessBuilder("pdfsig", pdf.toString());
        builder.redirectErrorStream(true);
        final Process process;
        try {
            process = builder.start();
        }
        catch (final java.io.IOException e) {
            throw new IllegalStateException(
                    "falta pdfsig: es la puerta de validez de la grada C (ADR-0014)."
                            + " Instalalo con 'apt-get install poppler-utils'.", e);
        }
        final String output = new String(process.getInputStream().readAllBytes(),
                StandardCharsets.UTF_8);
        assertTrue(process.waitFor(60, TimeUnit.SECONDS), "pdfsig se ha quedado colgado");
        assertEquals(0, process.exitValue(), "pdfsig ha fallado:\n" + output);
        return output;
    }
}
