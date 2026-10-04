package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.ByteArrayOutputStream;
import java.util.Base64;
import java.util.List;
import java.util.Properties;

import org.junit.jupiter.api.Test;
import org.w3c.dom.Document;
import org.w3c.dom.Element;
import org.w3c.dom.Node;
import org.w3c.dom.NodeList;
import org.w3c.dom.ls.DOMImplementationLS;
import org.w3c.dom.ls.LSOutput;

import es.gob.afirma.signvalidation.ValidateXMLSignature;

/**
 * El atacante firma con su clave, pone su {@code KeyValue} delante del {@code KeyInfo}
 * y deja en {@code X509Data} el certificado de otro titular. El validador del original
 * comprueba con el {@code KeyValue} y da la firma por buena, pero enseña al otro como
 * firmante. El puente liga la clave al certificado y la rechaza en los tres caminos.
 */
class XadesSignerKeyBindingTest {

    private static final String XMLDSIG_NS = "http://www.w3.org/2000/09/xmldsig#";

    private static final String VICTIM_IN_SUBJECT = "PUESTO TEST";

    @Test
    void the_original_validator_accepts_a_xades_whose_key_is_not_its_certificate_key()
            throws Exception {
        assertTrue(XadesCycle.isValid(new ValidateXMLSignature().validate(foreignKeyXades())),
                "el validador del original, con KeyValueKeySelector, da por buena la firma maliciosa");
    }

    @Test
    void the_shown_signer_is_the_foreign_certificate_not_the_signing_key() throws Exception {
        final PreviousSignaturesBridge.Signature signature =
                PreviousSignaturesBridge.read(foreignKeyXades()).signatures().get(0);

        assertTrue(signature.subject().contains(VICTIM_IN_SUBJECT),
                "el certificado que se enseña es el del otro titular: " + signature.subject());
    }

    @Test
    void the_site_verdict_rejects_a_xades_whose_key_is_not_its_certificate_key() throws Exception {
        final ValidationBridge.Verdict verdict =
                ValidationBridge.validate(foreignKeyXades(), "XAdES Enveloping", false);

        assertEquals(ValidationBridge.INVALID, verdict.outcome());
    }

    @Test
    void verify_rejects_a_xades_whose_key_is_not_its_certificate_key() throws Exception {
        final List<String> results = ValidationBridge.results(foreignKeyXades(), "XAdES Enveloping");

        assertFalse(results.stream().allMatch(text -> text.equals("Firma valida")),
                "verify no puede dar por valida una firma que no es del certificado que enseña: " + results);
    }

    @Test
    void the_previous_signatures_report_marks_it_invalid() throws Exception {
        final PreviousSignaturesBridge.Signature signature =
                PreviousSignaturesBridge.read(foreignKeyXades()).signatures().get(0);

        assertEquals(PreviousSignaturesBridge.Validity.INVALID, signature.validity());
        assertEquals(PreviousSignaturesBridge.Problem.DAMAGED,
                signature.validityReason().problem());
    }

    @Test
    void a_legitimate_xades_still_binds_its_key_to_its_certificate() throws Exception {
        final byte[] signed = XadesCycle.sign(XadesCycle.referenceXml(), new Properties());

        assertEquals(ValidationBridge.VALID,
                ValidationBridge.validate(signed, "XAdES Enveloping", false).outcome());
        assertEquals(PreviousSignaturesBridge.Validity.VALID,
                PreviousSignaturesBridge.read(signed).signatures().get(0).validity());
    }

    @Test
    void a_legitimate_facturae_still_binds_its_key_to_its_certificate() throws Exception {
        final Properties facturaE = new Properties();
        facturaE.setProperty("format", "FacturaE");
        final byte[] signed = XadesCycle.sign(invoice(), facturaE);

        assertEquals(ValidationBridge.VALID,
                ValidationBridge.validate(signed, "FacturaE", false).outcome());
    }

    /**
     * Una XAdES firmada por {@link TestFixtures#certificateChain()} con su {@code KeyValue}
     * delante del {@code X509Data}, en el que se deja el certificado de otro titular.
     */
    private static byte[] foreignKeyXades() throws Exception {
        final Properties params = new Properties();
        params.setProperty("format", "XAdES");
        params.setProperty("addKeyInfoKeyValue", "true");
        params.setProperty("keepKeyInfoUnsigned", "true");
        final byte[] signed = XadesCycle.sign(XadesCycle.referenceXml(), params);

        final Document doc = XadesCycle.parse(signed);
        final Element signature =
                (Element) doc.getElementsByTagNameNS(XMLDSIG_NS, "Signature").item(0);
        final Element keyInfo = firstChild(signature, "KeyInfo");
        final Element x509Data = firstChild(keyInfo, "X509Data");
        keyInfo.insertBefore(firstChild(keyInfo, "KeyValue"), x509Data);

        final Element certificate =
                (Element) x509Data.getElementsByTagNameNS(XMLDSIG_NS, "X509Certificate").item(0);
        certificate.setTextContent(Base64.getEncoder().encodeToString(
                TestFixtures.pseudonymCertificate().getEncoded()));

        return serialize(doc);
    }

    private static byte[] invoice() throws Exception {
        return java.nio.file.Files.readAllBytes(
                java.nio.file.Path.of("..", "testdata", "reference", "invoice.xml"));
    }

    private static Element firstChild(final Element parent, final String localName) {
        final NodeList children = parent.getChildNodes();
        for (int i = 0; i < children.getLength(); i++) {
            final Node child = children.item(i);
            if (child instanceof Element element
                    && XMLDSIG_NS.equals(element.getNamespaceURI())
                    && localName.equals(element.getLocalName())) {
                return element;
            }
        }
        return null;
    }

    private static byte[] serialize(final Document doc) {
        final DOMImplementationLS ls =
                (DOMImplementationLS) doc.getImplementation().getFeature("LS", "3.0");
        final ByteArrayOutputStream bytes = new ByteArrayOutputStream();
        final LSOutput output = ls.createLSOutput();
        output.setByteStream(bytes);
        output.setEncoding("UTF-8");
        ls.createLSSerializer().write(doc, output);
        return bytes.toByteArray();
    }
}
