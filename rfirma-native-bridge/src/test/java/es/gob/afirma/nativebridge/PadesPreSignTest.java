package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.charset.StandardCharsets;
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
 * Grada A: la prefirma no necesita nada mas que el kit de pruebas versionado.
 *
 * <p>Lo que se comprueba del campo {@code PRE} es su <b>estructura ASN.1</b>, no
 * su longitud: una prueba que compare longitudes pasa igual de bien con un hash
 * de 32 bytes envuelto en cualquier cosa, que es exactamente el error que el
 * ID-15 existe para descartar.
 */
class PadesPreSignTest {

    /** PKCS#9 content-type. */
    private static final String OID_CONTENT_TYPE = "1.2.840.113549.1.9.3";
    /** PKCS#9 message-digest. */
    private static final String OID_MESSAGE_DIGEST = "1.2.840.113549.1.9.4";
    /** PKCS#9 signing-time. */
    private static final String OID_SIGNING_TIME = "1.2.840.113549.1.9.5";
    /** id-aa-signingCertificateV2. */
    private static final String OID_SIGNING_CERTIFICATE_V2 = "1.2.840.113549.1.9.16.2.47";

    private static PadesBridge.PreSignResult preSign(final Properties params) throws Exception {
        return PadesBridge.preSign(TestFixtures.samplePdf(), "SHA256withRSA",
                TestFixtures.certificateChain(), params);
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
        final byte[] der = Base64.getDecoder().decode(preSign(new Properties()).preSignB64());

        // Parsea como SET OF Attribute, y reencodearlo en DER da los mismos
        // bytes: si viniera en BER —o fuera un hash suelto— esto no se cumple.
        final ASN1Set attributes = ASN1Set.getInstance(der);
        assertArrayEquals(der, attributes.getEncoded(ASN1Encoding.DER));

        final Map<String, ASN1Encodable> byOid = signedAttributes(der);
        assertNotNull(byOid.get(OID_CONTENT_TYPE), "falta el atributo content-type");
        assertNotNull(byOid.get(OID_SIGNING_CERTIFICATE_V2),
                "falta el atributo signing-certificate-v2");

        // El message-digest lleva el hash del rango de bytes del PDF, no el PDF.
        final ASN1OctetString messageDigest =
                ASN1OctetString.getInstance(byOid.get(OID_MESSAGE_DIGEST));
        assertEquals(32, messageDigest.getOctets().length,
                "el message-digest de SHA-256 son 32 bytes");
    }

    @Test
    void leaves_the_cades_signing_time_out_of_the_signed_attributes() throws Exception {
        // PAdESTriPhaseSigner:194-203 fuerza setSigningTime(null): la hora va
        // aparte, en el TIME del TriphaseData, y la reconstruye la postfirma.
        // Si algun dia entra, el sello de sesion deja de bastar.
        final byte[] der = Base64.getDecoder().decode(preSign(new Properties()).preSignB64());

        assertNull(signedAttributes(der).get(OID_SIGNING_TIME),
                "los atributos firmados de PAdES no llevan signing-time");
    }

    @Test
    void seals_the_time_that_the_session_carries() throws Exception {
        final PadesBridge.PreSignResult result = preSign(new Properties());

        final SessionStamp stamp = SessionStamp.decode(result.stamp());
        assertTrue(result.session().contains("<param n=\"TIME\">" + stamp.time() + "</param>"),
                "el TIME del sello tiene que ser el mismo que el de la sesion");
        assertEquals("SHA256withRSA", stamp.algorithm());
    }

    @Test
    void seals_the_effective_extra_params_not_the_ones_it_was_sent() throws Exception {
        // Con perfil baseline, PdfSessionManager:154 ANADE signatureSubFilter al
        // Properties que recibe, y PAdESTriPhaseSigner:174 no lo clona. Sellar lo
        // enviado en vez de lo efectivo reintroduciria el fallo por otra puerta
        // (ADR-0016).
        final Properties sent = new Properties();
        sent.setProperty("profile", "baseline");

        final SessionStamp stamp = SessionStamp.decode(preSign(sent).stamp());

        assertEquals("ETSI.CAdES.detached", stamp.extraParams().getProperty("signatureSubFilter"),
                "el sello tiene que llevar el subfiltro que anadio la prefirma");
        assertEquals("baseline", stamp.extraParams().getProperty("profile"));
    }

    @Test
    void rejects_something_that_is_not_a_pdf() {
        assertThrows(Exception.class, () -> PadesBridge.preSign(
                "no soy un PDF".getBytes(StandardCharsets.UTF_8),
                "SHA256withRSA", TestFixtures.certificateChain(), new Properties()));
    }
}
