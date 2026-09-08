package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.Properties;

import org.bouncycastle.cms.CMSProcessableByteArray;
import org.bouncycastle.cms.CMSSignedData;
import org.bouncycastle.cms.SignerInformation;
import org.bouncycastle.cms.jcajce.JcaSimpleSignerInfoVerifierBuilder;
import org.junit.jupiter.api.Test;

/**
 * El ciclo trifasico CAdES completo, con BouncyCastle como puerta automatica de
 * validez. Es de grada A: no hace falta ninguna herramienta del sistema, al
 * contrario que {@link PadesFullCycleTest}, que necesita {@code pdfsig}.
 *
 * <p>La fase 2 la hace {@link CadesCycle} con la clave del kit FNMT, que es
 * donde queda escrito el contrato de la prefirma.
 */
class CadesFullCycleTest {

    @Test
    void signs_a_challenge_in_explicit_mode_and_bouncycastle_validates_it() throws Exception {
        final byte[] document = TestFixtures.challenge();
        final byte[] signature = CadesCycle.sign(document, new Properties(), "sign");

        assertNull(new CMSSignedData(signature).getSignedContent(),
                "el modo explicito deja el contenido fuera del CMS");

        final CMSSignedData cms =
                new CMSSignedData(new CMSProcessableByteArray(document), signature);
        assertTrue(verifies(cms), "BouncyCastle no da la firma CAdES explicita por valida");
    }

    @Test
    void signs_a_challenge_in_implicit_mode_and_bouncycastle_validates_it() throws Exception {
        final byte[] document = TestFixtures.challenge();
        final Properties implicitMode = new Properties();
        implicitMode.setProperty("mode", "implicit");

        final CMSSignedData cms = new CMSSignedData(CadesCycle.sign(document, implicitMode, "sign"));

        assertNotNull(cms.getSignedContent(), "el modo implicito lleva el contenido dentro");
        assertArrayEquals(document, (byte[]) cms.getSignedContent().getContent());
        assertTrue(verifies(cms), "BouncyCastle no da la firma CAdES implicita por valida");
    }

    /**
     * Solo se comprueba la firma, no la cadena de confianza: la CA de pruebas de
     * la FNMT no esta en ningun almacen del sistema, y afirmar lo contrario seria
     * una prueba que no prueba nada.
     */
    private static boolean verifies(final CMSSignedData cms) throws Exception {
        final SignerInformation signer = cms.getSignerInfos().getSigners().iterator().next();
        return signer.verify(new JcaSimpleSignerInfoVerifierBuilder()
                .build(TestFixtures.activeCertificate()));
    }
}
