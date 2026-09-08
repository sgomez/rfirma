package es.gob.afirma.nativebridge;

import java.io.ByteArrayInputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.Signature;
import java.security.cert.X509Certificate;
import java.util.ArrayList;
import java.util.Base64;
import java.util.List;
import java.util.Properties;

import javax.xml.parsers.DocumentBuilderFactory;

import org.w3c.dom.Document;
import org.w3c.dom.Element;
import org.w3c.dom.Node;
import org.w3c.dom.NodeList;

import es.gob.afirma.signvalidation.SignValidity;
import es.gob.afirma.signvalidation.SignValidity.SIGN_DETAIL_TYPE;
import es.gob.afirma.signvalidation.ValidateXMLSignature;

/**
 * El ciclo trifasico XAdES de las pruebas: la fase 2 la hace la JCE con la clave
 * del kit FNMT, y el veredicto lo dan dos validadores independientes.
 *
 * <p>En la aplicacion la fase 2 la hara Rust contra el PKCS#11 del sistema y la
 * clave privada no entrara nunca en Java (ADR-0001); lo que fija esto es el
 * <b>contrato</b>: un PKCS#1 sobre los bytes del {@code SignedInfo}
 * canonicalizado, sin mas envoltorio.
 */
final class XadesCycle {

    static final String ALGORITHM = "SHA256withRSA";

    private static final Path REFERENCE_XML =
            Path.of("..", "testdata", "reference", "document.xml");

    private XadesCycle() { }

    static byte[] referenceXml() throws Exception {
        return Files.readAllBytes(REFERENCE_XML);
    }

    static XadesBridge.PreSignResult preSign(final byte[] document, final Properties extraParams,
            final String operation) throws Exception {
        return XadesBridge.preSign(document, ALGORITHM, TestFixtures.certificateChain(),
                extraParams, operation);
    }

    static byte[] sign(final byte[] document, final Properties extraParams) throws Exception {
        final X509Certificate[] chain = TestFixtures.certificateChain();
        final XadesBridge.PreSignResult pre =
                XadesBridge.preSign(document, ALGORITHM, chain, extraParams, "sign");
        return XadesBridge.postSign(document, chain, pre.stamp(), pre.session(), pkcs1For(pre));
    }

    static List<XadesBridge.SignatureValue> pkcs1For(final XadesBridge.PreSignResult pre)
            throws Exception {
        final List<XadesBridge.SignatureValue> values = new ArrayList<>();
        for (final XadesBridge.PreSign each : pre.pres()) {
            values.add(new XadesBridge.SignatureValue(each.id(), pkcs1Of(each.pre())));
        }
        return values;
    }

    static String pkcs1Of(final String preB64) throws Exception {
        final Signature signature = Signature.getInstance(ALGORITHM);
        signature.initSign(TestFixtures.privateKey());
        signature.update(Base64.getDecoder().decode(preB64));
        return Base64.getEncoder().encodeToString(signature.sign());
    }

    /** El veredicto del validador del original. */
    static List<SignValidity> validate(final byte[] signature) throws Exception {
        return new ValidateXMLSignature().validate(signature);
    }

    static boolean isValid(final List<SignValidity> verdicts) {
        return !verdicts.isEmpty()
                && verdicts.stream().allMatch(v -> v.getValidity() == SIGN_DETAIL_TYPE.OK);
    }

    /**
     * El segundo veredicto, con xmlsec directamente y no con el validador del
     * original: dos comprobaciones que comparten libreria no son dos.
     */
    static boolean xmlsecVerifies(final byte[] signature) throws Exception {
        org.apache.xml.security.Init.init();
        final Element root = parse(signature).getDocumentElement();
        markIdAttributes(root);
        final NodeList signatures = root.getOwnerDocument().getElementsByTagNameNS(
                javax.xml.crypto.dsig.XMLSignature.XMLNS, "Signature");
        if (signatures.getLength() != 1) {
            throw new IllegalStateException(
                    "el XML firmado tiene " + signatures.getLength() + " ds:Signature");
        }
        final org.apache.xml.security.signature.XMLSignature xmlSignature =
                new org.apache.xml.security.signature.XMLSignature(
                        (Element) signatures.item(0), "");
        return xmlSignature.checkSignatureValue(TestFixtures.activeCertificate());
    }

    static Document parse(final byte[] xml) throws Exception {
        final DocumentBuilderFactory factory = DocumentBuilderFactory.newInstance();
        factory.setFeature("http://apache.org/xml/features/disallow-doctype-decl", true);
        factory.setNamespaceAware(true);
        return factory.newDocumentBuilder().parse(new ByteArrayInputStream(xml));
    }

    /**
     * Sin esto la referencia a {@code #…-SignedProperties} no resuelve: el DOM no
     * trae DTD, asi que ningun atributo {@code Id} es de tipo ID (JDK-8134575).
     */
    private static void markIdAttributes(final Element element) {
        if (element.hasAttribute("Id")) {
            element.setIdAttribute("Id", true);
        }
        final NodeList children = element.getChildNodes();
        for (int i = 0; i < children.getLength(); i++) {
            if (children.item(i).getNodeType() == Node.ELEMENT_NODE) {
                markIdAttributes((Element) children.item(i));
            }
        }
    }
}
