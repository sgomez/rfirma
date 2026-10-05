package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.List;
import java.util.Properties;

import org.junit.jupiter.api.Test;

import es.gob.afirma.signvalidation.SignValidity;

/**
 * La XAdES explicita: Rust entrega la huella SHA-1 de los datos con
 * {@code mimeType=hash/sha1}, como el lanzador del original (ADR-0023).
 */
class XadesExplicitTest {

    @Test
    void an_explicit_xades_over_the_sha1_of_the_data_is_valid_for_both_validators()
            throws Exception {
        final byte[] digest =
                MessageDigest.getInstance("SHA-1").digest(XadesCycle.referenceXml());
        final Properties params = new Properties();
        params.setProperty("mode", "explicit");
        params.setProperty("mimeType", "hash/sha1");

        final byte[] signed = XadesCycle.sign(digest, params);

        final List<SignValidity> verdicts = XadesCycle.validate(signed);
        assertTrue(XadesCycle.isValid(verdicts),
                "el validador del original no da la firma por valida: " + verdicts);
        assertTrue(XadesCycle.xmlsecVerifies(signed), "xmlsec no da la firma por valida");
        assertTrue(new String(signed, StandardCharsets.UTF_8).contains("MimeType=\"hash/sha1\""),
                "el ds:Object declara la huella SHA-1");
    }
}
