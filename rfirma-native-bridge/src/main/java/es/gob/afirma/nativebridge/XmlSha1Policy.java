//! Quita de la política de validación segura de XMLDSig del JDK las tres prohibiciones de SHA-1 con las que la imagen nativa no firmaba ni validaba XAdES y FacturaE con SHA-1, que Rust solo pide con el permiso de la persona (ADR-0023).
package es.gob.afirma.nativebridge;

import java.security.Security;
import java.util.Arrays;
import java.util.List;
import java.util.stream.Collectors;

final class XmlSha1Policy {

    static final String PROPERTY = "jdk.xml.dsig.secureValidationPolicy";

    private static final List<String> SHA1_PROHIBITIONS = List.of(
            "disallowAlg http://www.w3.org/2000/09/xmldsig#sha1",
            "disallowAlg http://www.w3.org/2000/09/xmldsig#rsa-sha1",
            "disallowAlg http://www.w3.org/2001/04/xmldsig-more#ecdsa-sha1");

    private XmlSha1Policy() {
    }

    /** Aplica {@link #withoutSha1} a la propiedad; vale antes de que el JDK lea su {@code Policy}. */
    static void allowSha1() {
        final String policy = Security.getProperty(PROPERTY);
        if (policy != null) {
            Security.setProperty(PROPERTY, withoutSha1(policy));
        }
    }

    /** La política sin las prohibiciones de SHA-1 en huella, RSA y ECDSA; el resto, intacto. */
    static String withoutSha1(final String policy) {
        return Arrays.stream(policy.split(","))
                .map(entry -> entry.trim().replaceAll("\\s+", " "))
                .filter(entry -> !entry.isEmpty() && !SHA1_PROHIBITIONS.contains(entry))
                .collect(Collectors.joining(","));
    }
}
