package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.Properties;

import org.junit.jupiter.api.Test;
import org.w3c.dom.NodeList;

import es.gob.afirma.signvalidation.SignValidity;

/**
 * El ciclo trifasico XAdES completo sobre el XML de referencia, con dos puertas
 * automaticas de validez: el validador del original y xmlsec por su cuenta. Es
 * de grada A: no hace falta ninguna herramienta del sistema.
 *
 * <p>La fase 2 la hace {@link XadesCycle} con la clave del kit FNMT, que es
 * donde queda escrito el contrato de la prefirma.
 */
class XadesFullCycleTest {

    private static final String XMLDSIG_NS = "http://www.w3.org/2000/09/xmldsig#";

    @Test
    void signs_the_reference_xml_and_both_validators_accept_it() throws Exception {
        final byte[] signed = XadesCycle.sign(XadesCycle.referenceXml(), new Properties());

        final NodeList signatures = XadesCycle.parse(signed)
                .getElementsByTagNameNS(XMLDSIG_NS, "Signature");
        assertEquals(1, signatures.getLength(), "el XML firmado lleva una ds:Signature");

        final List<SignValidity> verdicts = XadesCycle.validate(signed);
        assertTrue(XadesCycle.isValid(verdicts),
                "el validador del original no da la firma XAdES por valida: " + verdicts);
        assertTrue(XadesCycle.xmlsecVerifies(signed), "xmlsec no da la firma XAdES por valida");
    }

    @Test
    void wraps_the_signed_data_because_the_default_variant_is_enveloping() throws Exception {
        final byte[] signed = XadesCycle.sign(XadesCycle.referenceXml(), new Properties());

        assertEquals("Signature",
                XadesCycle.parse(signed).getDocumentElement().getLocalName(),
                "en Enveloping la firma es la raiz y los datos van dentro de ella");
    }

    @Test
    void the_signature_covers_the_document_that_was_presigned() throws Exception {
        final byte[] document = XadesCycle.referenceXml();
        final byte[] signed = XadesCycle.sign(document, new Properties());

        assertTrue(new String(signed, StandardCharsets.UTF_8)
                        .contains("Documento de prueba rfirma"),
                "el XML firmado tiene que llevar dentro el documento de referencia");
    }
}
