package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Properties;

import org.junit.jupiter.api.Test;
import org.w3c.dom.NodeList;

import es.gob.afirma.signvalidation.SignValidity;

/**
 * El ciclo trifasico completo sobre la factura de referencia, con las dos
 * puertas de validez de {@link XadesCycle}, y la multifirma que el original no
 * tiene.
 */
class FacturaEFullCycleTest {

    private static final String XMLDSIG_NS = "http://www.w3.org/2000/09/xmldsig#";
    private static final String FACTURAE_POLICY =
            "http://www.facturae.es/politica_de_firma_formato_facturae/"
                    + "politica_de_firma_formato_facturae_v3_1.pdf";

    private static final Path REFERENCE_INVOICE =
            Path.of("..", "testdata", "reference", "invoice.xml");

    private static byte[] referenceInvoice() throws Exception {
        return Files.readAllBytes(REFERENCE_INVOICE);
    }

    private static Properties asFacturaE() {
        final Properties params = new Properties();
        params.setProperty("format", "FacturaE");
        return params;
    }

    @Test
    void signs_the_reference_invoice_and_both_validators_accept_it() throws Exception {
        final byte[] signed = XadesCycle.sign(referenceInvoice(), asFacturaE());

        final NodeList signatures = XadesCycle.parse(signed)
                .getElementsByTagNameNS(XMLDSIG_NS, "Signature");
        assertEquals(1, signatures.getLength(), "la factura firmada lleva una ds:Signature");

        final List<SignValidity> verdicts = XadesCycle.validate(signed);
        assertTrue(XadesCycle.isValid(verdicts),
                "el validador del original no da la factura por valida: " + verdicts);
        assertTrue(XadesCycle.xmlsecVerifies(signed), "xmlsec no da la factura por valida");
    }

    @Test
    void keeps_the_invoice_as_the_root_because_facturae_is_always_enveloped() throws Exception {
        final byte[] signed = XadesCycle.sign(referenceInvoice(), asFacturaE());

        assertEquals("Facturae", XadesCycle.parse(signed).getDocumentElement().getLocalName(),
                "una factura firmada sigue siendo una factura");
    }

    @Test
    void the_signer_imposes_the_facturae_policy_even_when_the_site_asks_for_none()
            throws Exception {
        final byte[] signed = XadesCycle.sign(referenceInvoice(), asFacturaE());

        assertTrue(new String(signed, StandardCharsets.UTF_8).contains(FACTURAE_POLICY),
                "la firma de una factura declara la politica de FacturaE 3.1");
    }

    @Test
    void a_policy_that_is_not_the_one_of_an_invoice_is_refused() throws Exception {
        final Properties params = asFacturaE();
        params.setProperty("policyIdentifier", "urn:una:politica:cualquiera");

        assertThrows(IllegalArgumentException.class,
                () -> XadesCycle.sign(referenceInvoice(), params),
                "solo se admiten las politicas de FacturaE 3.0 y 3.1");
    }

    @Test
    void an_invoice_is_neither_cosigned_nor_countersigned() throws Exception {
        final byte[] signed = XadesCycle.sign(referenceInvoice(), asFacturaE());

        for (final String operation : new String[] { "cosign", "countersign" }) {
            final UnsupportedOperationException refused =
                    assertThrows(UnsupportedOperationException.class,
                            () -> XadesCycle.sign(signed, asFacturaE(), operation),
                            "el original no multifirma facturas");
            assertTrue(refused.getMessage().contains("FacturaE"), refused.getMessage());
        }
    }

    @Test
    void what_is_not_an_invoice_does_not_get_signed_as_one() throws Exception {
        assertThrows(Exception.class,
                () -> XadesCycle.sign(XadesCycle.referenceXml(), asFacturaE()),
                "el procesador de facturas solo firma facturas");
    }
}
