package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Properties;

import org.junit.jupiter.api.Test;

import es.gob.afirma.signvalidation.SignValidity;

/**
 * El ciclo trifasico XAdES y FacturaE con {@code SHA1withRSA}, con las dos
 * puertas de validez de {@link XadesCycle}: el puente no rechaza SHA-1 en XML.
 */
class XadesSha1FullCycleTest {

    private static final String SHA1_RSA = "SHA1withRSA";
    private static final String SHA1_SIGNATURE_METHOD =
            "http://www.w3.org/2000/09/xmldsig#rsa-sha1";

    private static void assertValidWithSha1(final byte[] signed) throws Exception {
        final List<SignValidity> verdicts = XadesCycle.validate(signed);
        assertTrue(XadesCycle.isValid(verdicts),
                "el validador del original no da la firma por valida: " + verdicts);
        assertTrue(XadesCycle.xmlsecVerifies(signed), "xmlsec no da la firma por valida");
        assertTrue(new String(signed, StandardCharsets.UTF_8).contains(SHA1_SIGNATURE_METHOD),
                "la firma declara rsa-sha1 como metodo");
    }

    @Test
    void signs_xades_with_sha1withrsa_and_both_validators_accept_it() throws Exception {
        assertValidWithSha1(
                XadesCycle.signWith(SHA1_RSA, XadesCycle.referenceXml(), new Properties()));
    }

    @Test
    void signs_facturae_with_sha1withrsa_and_both_validators_accept_it() throws Exception {
        final Properties params = new Properties();
        params.setProperty("format", "FacturaE");
        final byte[] invoice =
                Files.readAllBytes(Path.of("..", "testdata", "reference", "invoice.xml"));

        assertValidWithSha1(XadesCycle.signWith(SHA1_RSA, invoice, params));
    }
}
