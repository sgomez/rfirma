//! Liga la firma XML al certificado que se enseña como firmante: su valor se comprueba con la clave publica de ese certificado y no con cualquier otra clave del KeyInfo.
package es.gob.afirma.nativebridge;

import java.security.PublicKey;
import java.security.cert.X509Certificate;
import java.util.ArrayList;
import java.util.List;

import javax.xml.crypto.dsig.XMLSignature;
import javax.xml.crypto.dsig.dom.DOMValidateContext;

import org.w3c.dom.Element;
import org.w3c.dom.Node;

import es.gob.afirma.signers.xml.Utils;
import es.gob.afirma.signers.xml.XMLConstants;

/** Comprueba que el {@code SignatureValue} se sostiene con la clave del certificado que se enseña. */
final class XmlSignerKeyBinding {

    private XmlSignerKeyBinding() { }

    /** La firma se sostiene con la clave del certificado que enseña su {@code KeyInfo}. */
    static boolean holds(final Element signature) {
        final PublicKey shownSignerKey = shownSignerKey(signature);
        return shownSignerKey == null || holds(signature, shownSignerKey);
    }

    /** La firma se sostiene con la clave dada. */
    static boolean holds(final Element signature, final PublicKey signerKey) {
        try {
            final DOMValidateContext context = new DOMValidateContext(signerKey, signature);
            final XMLSignature xmlSignature = Utils.getDOMFactory().unmarshalXMLSignature(context);
            return xmlSignature.getSignatureValue().validate(context);
        }
        catch (final Exception e) {
            return false;
        }
    }

    private static PublicKey shownSignerKey(final Element signature) {
        final Element keyInfo = firstChild(signature, "KeyInfo");
        if (keyInfo == null) {
            return null;
        }
        for (final Element data : children(keyInfo, "X509Data")) {
            for (final Element encoded : children(data, "X509Certificate")) {
                final X509Certificate certificate = Utils.getCertificate(encoded);
                if (certificate != null) {
                    return certificate.getPublicKey();
                }
            }
        }
        return null;
    }

    private static Element firstChild(final Element parent, final String localName) {
        final List<Element> found = children(parent, localName);
        return found.isEmpty() ? null : found.get(0);
    }

    private static List<Element> children(final Element parent, final String localName) {
        final List<Element> found = new ArrayList<>();
        for (Node child = parent.getFirstChild(); child != null; child = child.getNextSibling()) {
            if (child instanceof Element element
                    && XMLConstants.DSIGNNS.equals(element.getNamespaceURI())
                    && localName.equals(element.getLocalName())) {
                found.add(element);
            }
        }
        return found;
    }
}
