package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.ByteArrayInputStream;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.Properties;
import java.util.zip.ZipEntry;
import java.util.zip.ZipInputStream;

import javax.xml.crypto.OctetStreamData;
import javax.xml.crypto.URIDereferencer;
import javax.xml.crypto.dsig.XMLSignatureFactory;
import javax.xml.crypto.dsig.dom.DOMValidateContext;

import org.junit.jupiter.api.Test;
import org.w3c.dom.NodeList;

import es.gob.afirma.signers.xades.asic.AOXAdESASiCSSigner;
import es.gob.afirma.signvalidation.SignValidity;

/**
 * El ciclo trifasico completo de las variantes XAdES que no son la de por
 * defecto: Detached, Enveloped y el contenedor ASiC-S.
 *
 * <p>Detached y Enveloped se validan con el validador del original y con xmlsec,
 * igual que la Enveloping de {@link XadesFullCycleTest}. La ASiC-S se abre como
 * ZIP y se verifica resolviendole la referencia externa, porque
 * {@code afirma-crypto-validation} no valida firmas externally detached.
 */
class XadesVariantsTest {

    private static final String XMLDSIG_NS = "http://www.w3.org/2000/09/xmldsig#";
    private static final String ASIC_SIGNATURE_ENTRY = "META-INF/signatures.xml";
    private static final String ASIC_DATA_ENTRY = "dataobject.xml";

    @Test
    void signs_detached_leaving_the_data_beside_the_signature() throws Exception {
        final byte[] signed = XadesCycle.sign(XadesCycle.referenceXml(), variant("XAdES Detached"));

        assertEquals(1, signaturesIn(signed), "el XML firmado lleva una ds:Signature");
        assertTrue(new String(signed, StandardCharsets.UTF_8).contains("Documento de prueba"),
                "en Detached los datos viajan al lado de la firma, no dentro de un Object");
        assertBothValidatorsAccept(signed);
    }

    @Test
    void signs_enveloped_keeping_the_original_root() throws Exception {
        final byte[] signed = XadesCycle.sign(XadesCycle.referenceXml(), variant("XAdES Enveloped"));

        assertEquals("documento", XadesCycle.parse(signed).getDocumentElement().getLocalName(),
                "en Enveloped la firma se cuelga del documento y la raiz sigue siendo la suya");
        assertEquals(1, signaturesIn(signed), "el XML firmado lleva una ds:Signature");
        assertBothValidatorsAccept(signed);
    }

    @Test
    void signs_asic_s_into_a_zip_container() throws Exception {
        final byte[] document = XadesCycle.referenceXml();
        final byte[] container = XadesCycle.sign(document, variant("XAdES-ASiC-S"));

        final byte[] signature = entryOf(container, ASIC_SIGNATURE_ENTRY);
        assertNotNull(signature, "el contenedor ASiC-S tiene que llevar " + ASIC_SIGNATURE_ENTRY);
        assertEquals(1, signaturesIn(signature), "la firma del contenedor lleva una ds:Signature");
        assertTrue(new AOXAdESASiCSSigner().isSign(container),
                "el firmador ASiC-S del original no reconoce el contenedor como firma suya");
    }

    @Test
    void the_asic_s_signature_covers_the_document_that_travels_in_the_container() throws Exception {
        final byte[] document = XadesCycle.referenceXml();
        final byte[] container = XadesCycle.sign(document, variant("XAdES-ASiC-S"));

        assertArrayEquals(document, entryOf(container, ASIC_DATA_ENTRY),
                "el ASiC-S lleva el documento tal cual en " + ASIC_DATA_ENTRY);
        assertTrue(verifiesAgainstThePackagedData(entryOf(container, ASIC_SIGNATURE_ENTRY),
                        document),
                "la firma del contenedor no verifica contra el documento empaquetado");
    }

    @Test
    void seals_the_variant_that_was_asked_for() throws Exception {
        final SessionStamp stamp = SessionStamp.decode(
                XadesCycle.preSign(XadesCycle.referenceXml(), variant("XAdES-ASiC-S"), "sign")
                        .stamp());

        assertEquals("XAdES-ASiC-S", stamp.extraParams().getProperty("format"),
                "la variante que se firmo se sella, no se deduce en la postfirma");
    }

    @Test
    void refuses_an_enveloped_signature_over_something_that_is_not_xml() throws Exception {
        final byte[] notXml = "esto no es un XML".getBytes(StandardCharsets.UTF_8);

        final Exception failure = assertThrows(Exception.class,
                () -> XadesCycle.sign(notXml, variant("XAdES Enveloped")));

        final String json = NativeBridge.errorJson(failure);
        assertTrue(json.startsWith("{\"ok\":false"), json);
    }

    /**
     * La verificacion criptografica del ASiC-S, que no puede dar
     * {@code afirma-crypto-validation}: su firma es externally detached y hay que
     * resolverle la referencia con el fichero que viaja en el ZIP.
     */
    private static boolean verifiesAgainstThePackagedData(final byte[] signaturesXml,
            final byte[] data) throws Exception {
        final XMLSignatureFactory factory = XMLSignatureFactory.getInstance("DOM");
        final DOMValidateContext context = new DOMValidateContext(
                TestFixtures.activeCertificate().getPublicKey(),
                XadesCycle.signatureElementOf(signaturesXml));
        context.setURIDereferencer(dereferencerOf(factory.getURIDereferencer(), data));

        return factory.unmarshalXMLSignature(context).validate(context);
    }

    private static URIDereferencer dereferencerOf(final URIDereferencer fallback,
            final byte[] data) {
        return (reference, context) -> ASIC_DATA_ENTRY.equals(reference.getURI())
                ? new OctetStreamData(new ByteArrayInputStream(data))
                : fallback.dereference(reference, context);
    }

    private static void assertBothValidatorsAccept(final byte[] signature) throws Exception {
        final List<SignValidity> verdicts = XadesCycle.validate(signature);
        assertTrue(XadesCycle.isValid(verdicts),
                "el validador del original no da la firma XAdES por valida: " + verdicts);
        assertTrue(XadesCycle.xmlsecVerifies(signature), "xmlsec no da la firma XAdES por valida");
    }

    private static int signaturesIn(final byte[] xml) throws Exception {
        final NodeList signatures =
                XadesCycle.parse(xml).getElementsByTagNameNS(XMLDSIG_NS, "Signature");
        return signatures.getLength();
    }

    private static byte[] entryOf(final byte[] container, final String name) throws Exception {
        try (ZipInputStream zip = new ZipInputStream(new ByteArrayInputStream(container))) {
            ZipEntry entry;
            while ((entry = zip.getNextEntry()) != null) {
                if (name.equals(entry.getName())) {
                    return zip.readAllBytes();
                }
            }
        }
        return null;
    }

    private static Properties variant(final String format) {
        final Properties params = new Properties();
        params.setProperty("format", format);
        return params;
    }
}
