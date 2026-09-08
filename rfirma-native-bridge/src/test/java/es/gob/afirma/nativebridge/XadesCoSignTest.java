package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Properties;

import org.junit.jupiter.api.Test;
import org.w3c.dom.NodeList;

/**
 * La cofirma XAdES por los mismos dos puntos de entrada que la firma: la
 * entrada es la firma existente y el XML que sale lleva dos {@code ds:Signature}.
 */
class XadesCoSignTest {

    private static final Path REFERENCE =
            Path.of("..", "testdata", "reference", "xades-enveloping.xml");

    private static byte[] reference() throws Exception {
        return Files.readAllBytes(REFERENCE);
    }

    private static int signatureCount(final byte[] xml) throws Exception {
        final NodeList signatures = XadesCycle.parse(xml).getElementsByTagNameNS(
                javax.xml.crypto.dsig.XMLSignature.XMLNS, "Signature");
        return signatures.getLength();
    }

    @Test
    void cosigns_the_reference_signature_and_the_validator_accepts_it() throws Exception {
        final byte[] cosigned = XadesCycle.sign(reference(), new Properties(), "cosign");

        assertEquals(2, signatureCount(cosigned),
                "la cofirma tiene que dejar dos ds:Signature en el XML");
        assertTrue(XadesCycle.isValid(XadesCycle.validate(cosigned)),
                "el validador del original no da la cofirma por valida");
    }

    @Test
    void returns_a_single_presign_with_an_identifier() throws Exception {
        final XadesBridge.PreSignResult pre =
                XadesCycle.preSign(reference(), new Properties(), "cosign");

        assertEquals(1, pre.pres().size(), "una cofirma prefirma una sola vez");
        assertNotNull(pre.pres().get(0).id(), "cada prefirma viaja con su identificador");
    }

    @Test
    void seals_the_operation_so_the_postsign_cannot_assemble_another_one() throws Exception {
        final byte[] document = reference();
        final XadesBridge.PreSignResult pre =
                XadesCycle.preSign(document, new Properties(), "cosign");
        final String tampered = pre.session().replace(
                "<param n=\"OP\">cosign</param>", "<param n=\"OP\">sign</param>");
        assertTrue(tampered.contains("<param n=\"OP\">sign</param>"),
                "la sesion trifasica tiene que llevar la operacion");

        final SessionStampMismatchException failure =
                assertThrows(SessionStampMismatchException.class,
                        () -> XadesBridge.postSign(document, TestFixtures.certificateChain(),
                                pre.stamp(), tampered, XadesCycle.pkcs1For(pre)));

        assertTrue(failure.getMessage().contains("cosign"),
                "el fallo tiene que nombrar la operacion sellada: " + failure.getMessage());
    }

    @Test
    void refuses_to_assemble_with_a_pkcs1_short() throws Exception {
        final byte[] document = reference();
        final XadesBridge.PreSignResult pre =
                XadesCycle.preSign(document, new Properties(), "cosign");

        assertThrows(IllegalArgumentException.class,
                () -> XadesBridge.postSign(document, TestFixtures.certificateChain(), pre.stamp(),
                        pre.session(), List.of()));
    }
}
