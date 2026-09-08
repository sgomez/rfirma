package es.gob.afirma.nativebridge;

import java.security.Signature;
import java.security.cert.X509Certificate;
import java.util.ArrayList;
import java.util.Base64;
import java.util.List;
import java.util.Properties;

import es.gob.afirma.signvalidation.SignValidity;
import es.gob.afirma.signvalidation.SignValidity.SIGN_DETAIL_TYPE;
import es.gob.afirma.signvalidation.SignValidity.VALIDITY_ERROR;
import es.gob.afirma.signvalidation.SignValiderFactory;

/**
 * El ciclo trifasico CAdES de las pruebas: la fase 2 la hace la JCE con la clave
 * del kit FNMT, y el veredicto lo da el validador del original.
 *
 * <p>En la aplicacion la fase 2 la hara Rust contra el PKCS#11 del sistema y la
 * clave privada no entrara nunca en Java (ADR-0001); lo que fija esto es el
 * <b>contrato</b>: un PKCS#1 sobre los bytes DER de cada prefirma, sin mas
 * envoltorio.
 */
final class CadesCycle {

    private static final String ALGORITHM = "SHA256withRSA";

    private CadesCycle() { }

    static byte[] sign(final byte[] document, final Properties extraParams, final String operation)
            throws Exception {
        final X509Certificate[] chain = TestFixtures.certificateChain();
        final CadesBridge.PreSignResult pre =
                CadesBridge.preSign(document, ALGORITHM, chain, extraParams, operation);
        return CadesBridge.postSign(document, chain, pre.stamp(), pre.session(), pkcs1For(pre));
    }

    static CadesBridge.PreSignResult preSign(final byte[] document, final Properties extraParams,
            final String operation) throws Exception {
        return CadesBridge.preSign(document, ALGORITHM, TestFixtures.certificateChain(),
                extraParams, operation);
    }

    static List<CadesBridge.SignatureValue> pkcs1For(final CadesBridge.PreSignResult pre)
            throws Exception {
        final List<CadesBridge.SignatureValue> values = new ArrayList<>();
        for (final CadesBridge.PreSign each : pre.pres()) {
            values.add(new CadesBridge.SignatureValue(each.id(), pkcs1Of(each.pre())));
        }
        return values;
    }

    static String pkcs1Of(final String preB64) throws Exception {
        final Signature signature = Signature.getInstance(ALGORITHM);
        signature.initSign(TestFixtures.privateKey());
        signature.update(Base64.getDecoder().decode(preB64));
        return Base64.getEncoder().encodeToString(signature.sign());
    }

    static List<SignValidity> validate(final byte[] signature) throws Exception {
        return SignValiderFactory.getSignValider(signature).validate(signature);
    }

    /**
     * Un {@code UNKNOWN} por falta de datos o por perfil no comprobado es «no
     * comprobado», no «no valido»: una firma explicita sin el original a mano no
     * se puede comprobar entera. Cualquier otro veredicto que no sea OK cuenta
     * como invalido.
     */
    static boolean isValid(final List<SignValidity> verdicts) {
        return !verdicts.isEmpty() && verdicts.stream().allMatch(CadesCycle::isTolerable);
    }

    private static boolean isTolerable(final SignValidity verdict) {
        if (verdict.getValidity() == SIGN_DETAIL_TYPE.OK) {
            return true;
        }
        return verdict.getValidity() == SIGN_DETAIL_TYPE.UNKNOWN
                && (verdict.getError() == VALIDITY_ERROR.NO_DATA
                        || verdict.getError() == VALIDITY_ERROR.SIGN_PROFILE_NOT_CHECKED);
    }
}
