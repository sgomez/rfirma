package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.security.Security;

import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

/**
 * La politica de validacion segura de XMLDSig que lee el proveedor del JDK, el
 * unico que hay en la imagen nativa: en la JVM firma el Santuario externo, que
 * no la consulta, y por eso el ciclo SHA-1 de la JVM no la ve.
 */
class XmlSha1PolicyTest {

    private static final String JDK_POLICY = String.join(",",
            "disallowAlg http://www.w3.org/TR/1999/REC-xslt-19991116",
            "    disallowAlg http://www.w3.org/2001/04/xmldsig-more#rsa-md5",
            "    disallowAlg http://www.w3.org/2001/04/xmldsig-more#md5",
            "    disallowAlg http://www.w3.org/2000/09/xmldsig#sha1",
            "    disallowAlg http://www.w3.org/2000/09/xmldsig#dsa-sha1",
            "    disallowAlg  http://www.w3.org/2000/09/xmldsig#rsa-sha1",
            "    disallowAlg http://www.w3.org/2007/05/xmldsig-more#sha1-rsa-MGF1",
            "    disallowAlg http://www.w3.org/2001/04/xmldsig-more#ecdsa-sha1",
            "    maxTransforms 5",
            "    minKeySize RSA 1024",
            "    noDuplicateIds");

    @Test
    void only_the_sha1_digest_rsa_and_ecdsa_prohibitions_are_lifted() {
        assertEquals(String.join(",",
                "disallowAlg http://www.w3.org/TR/1999/REC-xslt-19991116",
                "disallowAlg http://www.w3.org/2001/04/xmldsig-more#rsa-md5",
                "disallowAlg http://www.w3.org/2001/04/xmldsig-more#md5",
                "disallowAlg http://www.w3.org/2000/09/xmldsig#dsa-sha1",
                "disallowAlg http://www.w3.org/2007/05/xmldsig-more#sha1-rsa-MGF1",
                "maxTransforms 5",
                "minKeySize RSA 1024",
                "noDuplicateIds"),
                XmlSha1Policy.withoutSha1(JDK_POLICY));
    }

    private String originalPolicy;

    @BeforeEach
    void rememberThePolicy() {
        originalPolicy = Security.getProperty(XmlSha1Policy.PROPERTY);
    }

    @AfterEach
    void restoreThePolicy() {
        Security.setProperty(XmlSha1Policy.PROPERTY, originalPolicy == null ? "" : originalPolicy);
    }

    @Test
    void the_policy_of_the_running_jdk_loses_rsa_sha1_and_keeps_rsa_md5() {
        XmlSha1Policy.allowSha1();

        final String policy = Security.getProperty(XmlSha1Policy.PROPERTY);
        assertFalse(policy.contains("xmldsig#rsa-sha1"), policy);
        assertTrue(policy.contains("xmldsig-more#rsa-md5"), policy);
    }
}
