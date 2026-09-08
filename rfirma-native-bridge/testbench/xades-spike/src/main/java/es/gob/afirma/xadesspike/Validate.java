package es.gob.afirma.xadesspike;

import java.io.FileInputStream;
import java.security.KeyStore;
import java.security.cert.X509Certificate;
import javax.xml.crypto.dsig.Reference;
import javax.xml.crypto.dsig.XMLSignature;
import javax.xml.crypto.dsig.XMLSignatureFactory;
import javax.xml.crypto.dsig.dom.DOMValidateContext;
import javax.xml.parsers.DocumentBuilderFactory;
import org.w3c.dom.Document;
import org.w3c.dom.Element;
import org.w3c.dom.NodeList;

/**
 * Valida criptograficamente el {@code XMLSignature} que produjo {@link Main},
 * con la API de JAXP ({@code javax.xml.crypto.dsig}), no con Apache Santuario
 * directamente: es el control independiente de la firma que ensambla el spike.
 */
public final class Validate {

    private Validate() { }

    /**
     * El DOM que produce el spike no trae DTD ni esquema: sin esto, la
     * resolucion de referencias {@code #Id} de {@code XMLSignatureFactory}
     * falla aunque la firma sea valida (JDK-8134575).
     */
    private static void markIdAttributes(final Document doc) {
        final NodeList all = doc.getElementsByTagName("*");
        for (int i = 0; i < all.getLength(); i++) {
            final Element el = (Element) all.item(i);
            if (el.hasAttribute("Id")) {
                el.setIdAttribute("Id", true);
            }
        }
    }

    public static void main(final String[] args) throws Exception {
        final String xmlPath = args[0];
        final String p12Path = args[1];
        final String alias = args[2];
        final String pass = args[3];

        final KeyStore ks = KeyStore.getInstance("PKCS12");
        try (var in = new FileInputStream(p12Path)) {
            ks.load(in, pass.toCharArray());
        }
        final X509Certificate cert = (X509Certificate) ks.getCertificate(alias);

        final DocumentBuilderFactory dbf = DocumentBuilderFactory.newInstance();
        dbf.setNamespaceAware(true);
        final Document doc = dbf.newDocumentBuilder().parse(new FileInputStream(xmlPath));
        markIdAttributes(doc);
        final NodeList nl =
                doc.getElementsByTagNameNS("http://www.w3.org/2000/09/xmldsig#", "Signature");
        if (nl.getLength() == 0) {
            System.out.println("SIN FIRMA");
            System.exit(1);
        }
        final Element sigElement = (Element) nl.item(0);
        final DOMValidateContext valContext = new DOMValidateContext(cert.getPublicKey(), sigElement);
        final XMLSignatureFactory fac = XMLSignatureFactory.getInstance("DOM");
        final XMLSignature signature = fac.unmarshalXMLSignature(valContext);
        final boolean coreValidity = signature.validate(valContext);
        System.out.println("CORE_VALIDITY=" + coreValidity);
        if (!coreValidity) {
            System.out.println("SIGNATURE_VALUE=" + signature.getSignatureValue().validate(valContext));
            int i = 0;
            for (final Object ref : signature.getSignedInfo().getReferences()) {
                System.out.println("REF[" + i++ + "]=" + ((Reference) ref).validate(valContext));
            }
        }
        System.exit(coreValidity ? 0 : 1);
    }
}
