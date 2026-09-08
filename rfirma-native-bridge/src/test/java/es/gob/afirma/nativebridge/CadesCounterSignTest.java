package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Properties;

import org.bouncycastle.cms.CMSSignedData;
import org.bouncycastle.cms.SignerInformation;
import org.junit.jupiter.api.Test;

import es.gob.afirma.signvalidation.SignValidity;

/**
 * La contrafirma CAdES, que es la operacion que rompe el «una prefirma por
 * ciclo»: sobre una firma cofirmada hay una hoja por firmante y el puente
 * devuelve una prefirma por cada una.
 */
class CadesCounterSignTest {

    private static final Path COSIGNED =
            Path.of("..", "testdata", "reference", "cades-implicit.cosign.p7s");

    private static byte[] cosigned() throws Exception {
        return Files.readAllBytes(COSIGNED);
    }

    private static Properties target(final String target) {
        final Properties params = new Properties();
        params.setProperty("target", target);
        return params;
    }

    private static void assertEveryLeafIsCounterSigned(final byte[] counterSigned)
            throws Exception {
        for (final SignerInformation signer
                : new CMSSignedData(counterSigned).getSignerInfos().getSigners()) {
            assertEquals(1, signer.getCounterSignatures().size(),
                    "cada firmante de la firma cofirmada tiene que quedar contrafirmado");
        }
        final List<SignValidity> verdicts = CadesCycle.validate(counterSigned);
        assertTrue(CadesCycle.isValid(verdicts),
                "el validador del original no da la contrafirma por valida: " + verdicts);
    }

    @Test
    void countersigns_every_leaf_and_the_validator_accepts_it() throws Exception {
        assertEveryLeafIsCounterSigned(CadesCycle.sign(cosigned(), target("leafs"), "countersign"));
    }

    @Test
    void countersigns_the_whole_tree_and_the_validator_accepts_it() throws Exception {
        assertEveryLeafIsCounterSigned(CadesCycle.sign(cosigned(), target("tree"), "countersign"));
    }

    @Test
    void returns_one_presign_per_leaf() throws Exception {
        final CadesBridge.PreSignResult pre =
                CadesCycle.preSign(cosigned(), target("leafs"), "countersign");

        assertEquals(2, pre.pres().size(),
                "la firma cofirmada tiene dos firmantes, y cada uno es una prefirma");
        assertEquals(2, pre.pres().stream().map(CadesBridge.PreSign::id).distinct().count(),
                "cada prefirma se identifica por su firma, y no se pueden confundir");
    }

    @Test
    void reaches_more_nodes_with_the_tree_target_than_with_the_leafs_one() throws Exception {
        // Sobre una firma que ya lleva contrafirmas: leafs se queda en los nodos
        // sin contrafirmar y tree recorre el arbol entero. Sin este segundo paso
        // los dos objetivos serian indistinguibles.
        final byte[] counterSigned =
                CadesCycle.sign(cosigned(), target("leafs"), "countersign");

        final int leafs = CadesCycle.preSign(counterSigned, target("leafs"), "countersign")
                .pres().size();
        final int tree = CadesCycle.preSign(counterSigned, target("tree"), "countersign")
                .pres().size();

        assertTrue(tree > leafs, "tree ha alcanzado " + tree + " nodos y leafs " + leafs);
    }

    @Test
    void countersigns_the_leafs_when_the_caller_does_not_say_which_target() throws Exception {
        final CadesBridge.PreSignResult pre =
                CadesCycle.preSign(cosigned(), new Properties(), "countersign");

        assertEquals("leafs", SessionStamp.decode(pre.stamp()).target());
    }

    @Test
    void refuses_a_target_it_does_not_know() throws Exception {
        final IllegalArgumentException failure = assertThrows(IllegalArgumentException.class,
                () -> CadesCycle.preSign(cosigned(), target("signers"), "countersign"));

        assertTrue(failure.getMessage().contains("signers"),
                "el fallo tiene que nombrar el objetivo pedido: " + failure.getMessage());
    }

    @Test
    void seals_the_target_so_the_postsign_cannot_change_it() throws Exception {
        final byte[] document = cosigned();
        final CadesBridge.PreSignResult pre =
                CadesCycle.preSign(document, target("tree"), "countersign");
        final String tampered = pre.session().replace(
                "<param n=\"TARGET\">tree</param>", "<param n=\"TARGET\">leafs</param>");
        assertTrue(tampered.contains("<param n=\"TARGET\">leafs</param>"),
                "la sesion trifasica tiene que llevar el objetivo de la contrafirma");

        final SessionStampMismatchException failure =
                assertThrows(SessionStampMismatchException.class,
                        () -> CadesBridge.postSign(document, TestFixtures.certificateChain(),
                                pre.stamp(), tampered, CadesCycle.pkcs1For(pre)));

        assertTrue(failure.getMessage().contains("tree"),
                "el fallo tiene que nombrar el objetivo sellado: " + failure.getMessage());
    }

    @Test
    void refuses_to_assemble_with_a_pkcs1_short() throws Exception {
        final byte[] document = cosigned();
        final CadesBridge.PreSignResult pre =
                CadesCycle.preSign(document, target("leafs"), "countersign");

        final IllegalArgumentException failure = assertThrows(IllegalArgumentException.class,
                () -> CadesBridge.postSign(document, TestFixtures.certificateChain(), pre.stamp(),
                        pre.session(), List.of(CadesCycle.pkcs1For(pre).get(0))));

        assertTrue(failure.getMessage().contains("PKCS#1"),
                "el fallo tiene que decir que falta un PKCS#1: " + failure.getMessage());
    }
}
