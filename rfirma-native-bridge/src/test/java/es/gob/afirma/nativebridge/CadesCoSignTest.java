package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Properties;

import org.bouncycastle.cms.CMSSignedData;
import org.junit.jupiter.api.Test;

import es.gob.afirma.signvalidation.SignValidity;

/**
 * La cofirma CAdES por los mismos dos puntos de entrada que la firma: la entrada
 * es la firma existente y el CMS que sale lleva dos firmantes.
 */
class CadesCoSignTest {

    private static final Path REFERENCE =
            Path.of("..", "testdata", "reference", "cades-implicit.p7s");

    private static byte[] reference() throws Exception {
        return Files.readAllBytes(REFERENCE);
    }

    @Test
    void cosigns_the_reference_signature_and_the_validator_accepts_it() throws Exception {
        final byte[] cosigned = CadesCycle.sign(reference(), new Properties(), "cosign");

        assertEquals(2, new CMSSignedData(cosigned).getSignerInfos().size(),
                "la cofirma tiene que dejar dos SignerInfo en el CMS");
        final List<SignValidity> verdicts = CadesCycle.validate(cosigned);
        assertTrue(CadesCycle.isValid(verdicts),
                "el validador del original no da la cofirma por valida: " + verdicts);
    }

    @Test
    void returns_a_single_presign_with_an_identifier() throws Exception {
        final CadesBridge.PreSignResult pre =
                CadesCycle.preSign(reference(), new Properties(), "cosign");

        assertEquals(1, pre.pres().size(), "una cofirma prefirma una sola vez");
        assertNotNull(pre.pres().get(0).id(), "cada prefirma viaja con su identificador");
    }

    @Test
    void seals_the_operation_so_the_postsign_cannot_assemble_another_one() throws Exception {
        final byte[] document = reference();
        final CadesBridge.PreSignResult pre =
                CadesCycle.preSign(document, new Properties(), "cosign");
        final String tampered = pre.session().replace(
                "<param n=\"OP\">cosign</param>", "<param n=\"OP\">sign</param>");
        assertTrue(tampered.contains("<param n=\"OP\">sign</param>"),
                "la sesion trifasica tiene que llevar la operacion");

        final SessionStampMismatchException failure =
                assertThrows(SessionStampMismatchException.class,
                        () -> CadesBridge.postSign(document, TestFixtures.certificateChain(),
                                pre.stamp(), tampered, CadesCycle.pkcs1For(pre)));

        assertTrue(failure.getMessage().contains("cosign"),
                "el fallo tiene que nombrar la operacion sellada: " + failure.getMessage());
    }
}
