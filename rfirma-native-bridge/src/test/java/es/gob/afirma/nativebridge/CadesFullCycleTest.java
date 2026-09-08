package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.security.Signature;
import java.security.cert.X509Certificate;
import java.util.Base64;
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
 * <p>La fase 2 la hace aqui la JCE con la clave del kit FNMT. En la aplicacion
 * la hara Rust contra el PKCS#11 del sistema y la clave privada no entrara nunca
 * en Java (ADR-0001); lo que esta prueba fija es el <b>contrato</b>: un PKCS#1
 * sobre los bytes DER de la prefirma, sin mas envoltorio.
 */
class CadesFullCycleTest {

    private static final String ALGORITHM = "SHA256withRSA";

    @Test
    void signs_a_challenge_in_explicit_mode_and_bouncycastle_validates_it() throws Exception {
        final byte[] document = TestFixtures.challenge();
        final byte[] signature = sign(document, new Properties());

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

        final CMSSignedData cms = new CMSSignedData(sign(document, implicitMode));

        assertNotNull(cms.getSignedContent(), "el modo implicito lleva el contenido dentro");
        assertArrayEquals(document, (byte[]) cms.getSignedContent().getContent());
        assertTrue(verifies(cms), "BouncyCastle no da la firma CAdES implicita por valida");
    }

    /** Las tres fases, con la JCE en la de en medio. */
    private static byte[] sign(final byte[] document, final Properties extraParams)
            throws Exception {
        final X509Certificate[] chain = TestFixtures.certificateChain();
        final CadesBridge.PreSignResult pre =
                CadesBridge.preSign(document, ALGORITHM, chain, extraParams, "sign");

        final Signature signature = Signature.getInstance(ALGORITHM);
        signature.initSign(TestFixtures.privateKey());
        signature.update(Base64.getDecoder().decode(pre.preSignB64()));
        final String pkcs1 = Base64.getEncoder().encodeToString(signature.sign());

        return CadesBridge.postSign(document, chain, pre.stamp(), pre.session(), pkcs1);
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
