package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Properties;

import org.junit.jupiter.api.Test;

/**
 * La contrafirma XAdES por los mismos dos puntos de entrada que la firma: sobre
 * una firma cofirmada hay una hoja por firmante y el puente devuelve una
 * prefirma por cada una, con {@code BASE} solo en la primera.
 */
class XadesCounterSignTest {

    private static final Path REFERENCE =
            Path.of("..", "testdata", "reference", "xades-enveloping.xml");
    private static final Path COSIGNED =
            Path.of("..", "testdata", "reference", "xades-enveloping.cosign.xml");

    private static byte[] reference() throws Exception {
        return Files.readAllBytes(REFERENCE);
    }

    private static byte[] cosigned() throws Exception {
        return Files.readAllBytes(COSIGNED);
    }

    private static Properties target(final String target) {
        final Properties params = new Properties();
        params.setProperty("target", target);
        return params;
    }

    @Test
    void countersigns_the_tree_and_the_validator_accepts_it() throws Exception {
        final byte[] counterSigned = XadesCycle.sign(reference(), target("tree"), "countersign");

        assertTrue(XadesCycle.isValid(XadesCycle.validate(counterSigned)),
                "el validador del original no da la contrafirma por valida");
    }

    @Test
    void countersigns_the_leafs_and_the_validator_accepts_it() throws Exception {
        final byte[] counterSigned = XadesCycle.sign(reference(), target("leafs"), "countersign");

        assertTrue(XadesCycle.isValid(XadesCycle.validate(counterSigned)),
                "el validador del original no da la contrafirma por valida");
    }

    @Test
    void returns_one_presign_per_leaf_when_countersigning_a_cosigned_signature() throws Exception {
        final XadesBridge.PreSignResult pre =
                XadesCycle.preSign(cosigned(), target("leafs"), "countersign");

        assertEquals(2, pre.pres().size(),
                "la firma cofirmada tiene dos firmantes, y cada uno es una prefirma");
    }

    @Test
    void countersigns_every_leaf_of_a_cosigned_signature_and_the_validator_accepts_it()
            throws Exception {
        final byte[] counterSigned = XadesCycle.sign(cosigned(), target("leafs"), "countersign");

        assertTrue(XadesCycle.isValid(XadesCycle.validate(counterSigned)),
                "el validador del original no da la contrafirma por valida");
    }

    @Test
    void countersigns_the_tree_when_the_caller_does_not_say_which_target() throws Exception {
        final XadesBridge.PreSignResult pre =
                XadesCycle.preSign(reference(), new Properties(), "countersign");

        assertEquals("tree", SessionStamp.decode(pre.stamp()).target(),
                "el defecto de XAdES es tree, al reves que en CAdES");
    }

    @Test
    void refuses_a_target_it_does_not_know() throws Exception {
        final IllegalArgumentException failure = assertThrows(IllegalArgumentException.class,
                () -> XadesCycle.preSign(reference(), target("signers"), "countersign"));

        assertTrue(failure.getMessage().contains("signers"),
                "el fallo tiene que nombrar el objetivo pedido: " + failure.getMessage());
    }

    @Test
    void seals_the_target_so_the_postsign_cannot_change_it() throws Exception {
        final byte[] document = reference();
        final XadesBridge.PreSignResult pre =
                XadesCycle.preSign(document, target("tree"), "countersign");
        final String tampered = pre.session().replace(
                "<param n=\"TARGET\">tree</param>", "<param n=\"TARGET\">leafs</param>");
        assertTrue(tampered.contains("<param n=\"TARGET\">leafs</param>"),
                "la sesion trifasica tiene que llevar el objetivo de la contrafirma");

        final SessionStampMismatchException failure =
                assertThrows(SessionStampMismatchException.class,
                        () -> XadesBridge.postSign(document, TestFixtures.certificateChain(),
                                pre.stamp(), tampered, XadesCycle.pkcs1For(pre)));

        assertTrue(failure.getMessage().contains("tree"),
                "el fallo tiene que nombrar el objetivo sellado: " + failure.getMessage());
    }

    @Test
    void refuses_to_assemble_with_a_pkcs1_short() throws Exception {
        final byte[] document = cosigned();
        final XadesBridge.PreSignResult pre =
                XadesCycle.preSign(document, target("leafs"), "countersign");

        final IllegalArgumentException failure = assertThrows(IllegalArgumentException.class,
                () -> XadesBridge.postSign(document, TestFixtures.certificateChain(), pre.stamp(),
                        pre.session(), java.util.List.of(XadesCycle.pkcs1For(pre).get(0))));

        assertTrue(failure.getMessage().contains("PKCS#1"),
                "el fallo tiene que decir que falta un PKCS#1: " + failure.getMessage());
    }
}
