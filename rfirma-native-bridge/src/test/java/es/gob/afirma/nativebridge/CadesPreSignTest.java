package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.Base64;
import java.util.HashMap;
import java.util.Map;
import java.util.Properties;

import org.junit.jupiter.api.Test;
import org.spongycastle.asn1.ASN1Encodable;
import org.spongycastle.asn1.ASN1Encoding;
import org.spongycastle.asn1.ASN1ObjectIdentifier;
import org.spongycastle.asn1.ASN1OctetString;
import org.spongycastle.asn1.ASN1Sequence;
import org.spongycastle.asn1.ASN1Set;

/**
 * Grada A: la prefirma CAdES no necesita nada mas que el kit de pruebas
 * versionado.
 *
 * <p>Del campo {@code PRE} se comprueba su <b>estructura ASN.1</b>, no su
 * longitud, por lo mismo que en {@link PadesPreSignTest}: comparar longitudes
 * pasa igual de bien con un hash envuelto en cualquier cosa.
 */
class CadesPreSignTest {

    private static final String ALGORITHM = "SHA256withRSA";

    /** PKCS#9 content-type. */
    private static final String OID_CONTENT_TYPE = "1.2.840.113549.1.9.3";
    /** PKCS#9 message-digest. */
    private static final String OID_MESSAGE_DIGEST = "1.2.840.113549.1.9.4";
    /** PKCS#9 signing-time. */
    private static final String OID_SIGNING_TIME = "1.2.840.113549.1.9.5";
    /** id-aa-signingCertificateV2. */
    private static final String OID_SIGNING_CERTIFICATE_V2 = "1.2.840.113549.1.9.16.2.47";

    private static CadesBridge.PreSignResult preSign(final Properties params) throws Exception {
        return CadesBridge.preSign(TestFixtures.challenge(), ALGORITHM,
                TestFixtures.certificateChain(), params, "sign");
    }

    /** Los atributos firmados de la unica prefirma que devuelve una firma. */
    private static String soleValueOf(final CadesBridge.PreSignResult result) {
        assertEquals(1, result.pres().size(), "una firma prefirma una sola vez");
        return result.pres().get(0).pre();
    }

    /** El {@code SET OF Attribute} de la prefirma, indexado por OID. */
    private static Map<String, ASN1Encodable> signedAttributes(final byte[] der) {
        final ASN1Set attributes = ASN1Set.getInstance(der);
        final Map<String, ASN1Encodable> byOid = new HashMap<>();
        for (int i = 0; i < attributes.size(); i++) {
            final ASN1Sequence attribute = ASN1Sequence.getInstance(attributes.getObjectAt(i));
            final ASN1ObjectIdentifier oid =
                    ASN1ObjectIdentifier.getInstance(attribute.getObjectAt(0));
            final ASN1Set values = ASN1Set.getInstance(attribute.getObjectAt(1));
            byOid.put(oid.getId(), values.getObjectAt(0));
        }
        return byOid;
    }

    @Test
    void returns_the_cades_signed_attributes_in_asn1_der() throws Exception {
        final byte[] der = Base64.getDecoder().decode(soleValueOf(preSign(new Properties())));

        final ASN1Set attributes = ASN1Set.getInstance(der);
        assertArrayEquals(der, attributes.getEncoded(ASN1Encoding.DER));

        final Map<String, ASN1Encodable> byOid = signedAttributes(der);
        assertNotNull(byOid.get(OID_CONTENT_TYPE), "falta el atributo content-type");
        assertNotNull(byOid.get(OID_SIGNING_CERTIFICATE_V2),
                "falta el atributo signing-certificate-v2");

        final ASN1OctetString messageDigest =
                ASN1OctetString.getInstance(byOid.get(OID_MESSAGE_DIGEST));
        assertEquals(32, messageDigest.getOctets().length,
                "el message-digest de SHA-256 son 32 bytes");
    }

    @Test
    void keeps_the_signing_time_inside_the_signed_attributes() throws Exception {
        // Al reves que PAdES, que lo deja fuera y lo reconstruye en la postfirma:
        // en CAdES el instante va DENTRO de lo que se firma, y por eso la postfirma
        // no tiene ninguna fecha que rehacer ni ninguna zona horaria que imponer.
        final byte[] der = Base64.getDecoder().decode(soleValueOf(preSign(new Properties())));

        assertNotNull(signedAttributes(der).get(OID_SIGNING_TIME),
                "los atributos firmados de CAdES llevan signing-time");
    }

    @Test
    void seals_the_time_that_the_session_carries() throws Exception {
        final CadesBridge.PreSignResult result = preSign(new Properties());

        final SessionStamp stamp = SessionStamp.decode(result.stamp());
        assertTrue(result.session().contains("<param n=\"TIME\">" + stamp.time() + "</param>"),
                "el TIME del sello tiene que ser el mismo que el de la sesion");
        assertEquals(ALGORITHM, stamp.algorithm());
    }

    @Test
    void seals_the_mode_and_the_policy_without_translating_them() throws Exception {
        // mode y politica viajan en los extraParams tal cual: CAdESParameters.load
        // los lee, y el sello es lo que impide que la postfirma reciba otros.
        final Properties sent = new Properties();
        sent.setProperty("mode", "implicit");
        sent.setProperty("policyIdentifier", "urn:oid:2.16.724.1.3.1.1.2.1.9");
        sent.setProperty("policyIdentifierHash", "G7roucf600+f03r/o0bAOQ6WAs0=");
        sent.setProperty("policyIdentifierHashAlgorithm",
                "http://www.w3.org/2000/09/xmldsig#sha1");

        final SessionStamp stamp = SessionStamp.decode(preSign(sent).stamp());

        assertEquals("implicit", stamp.extraParams().getProperty("mode"));
        assertEquals("urn:oid:2.16.724.1.3.1.1.2.1.9",
                stamp.extraParams().getProperty("policyIdentifier"));
        assertEquals("G7roucf600+f03r/o0bAOQ6WAs0=",
                stamp.extraParams().getProperty("policyIdentifierHash"));
    }

    @Test
    void seals_the_operation_that_the_session_carries() throws Exception {
        final CadesBridge.PreSignResult result = preSign(new Properties());

        final SessionStamp stamp = SessionStamp.decode(result.stamp());
        assertEquals("sign", stamp.operation());
        assertNull(stamp.target(), "una firma no tiene objetivo de contrafirma");
        assertTrue(result.session().contains("<param n=\"OP\">sign</param>"),
                "la operacion sellada tiene que viajar tambien en la sesion");
    }

    @Test
    void refuses_an_operation_it_does_not_know() {
        final Exception failure = assertThrows(IllegalArgumentException.class,
                () -> CadesBridge.preSign(TestFixtures.challenge(), ALGORITHM,
                        TestFixtures.certificateChain(), new Properties(), "encrypt"));

        assertTrue(failure.getMessage().contains("encrypt"),
                "el mensaje tiene que nombrar la operacion: " + failure.getMessage());
    }

    @Test
    void rejects_an_empty_document() {
        assertThrows(Exception.class, () -> CadesBridge.preSign(new byte[0], ALGORITHM,
                TestFixtures.certificateChain(), new Properties(), "sign"));
    }
}
