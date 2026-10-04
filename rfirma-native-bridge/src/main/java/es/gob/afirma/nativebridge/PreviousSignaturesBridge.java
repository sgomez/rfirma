//! Las firmas que ya trae un PDF, un CAdES o un XAdES, una a una: quién firmó, cuándo y si es válida con su motivo (ADR-0043); no es el veredicto de conjunto de `ValidationBridge`.
package es.gob.afirma.nativebridge;

import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.cert.CertificateException;
import java.security.cert.CertificateFactory;
import java.security.cert.CertificateExpiredException;
import java.security.cert.CertificateNotYetValidException;
import java.security.cert.X509Certificate;
import java.time.Instant;
import java.time.format.DateTimeFormatter;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collection;
import java.util.Comparator;
import java.util.Date;
import java.util.HashMap;
import java.util.HexFormat;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Properties;
import java.util.Set;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

import javax.security.auth.x500.X500Principal;

import org.spongycastle.asn1.cms.Attribute;
import org.spongycastle.asn1.cms.CMSAttributes;
import org.spongycastle.asn1.cms.Time;
import org.spongycastle.cert.X509CertificateHolder;
import org.spongycastle.cms.CMSException;
import org.spongycastle.cms.CMSSignedData;
import org.spongycastle.cms.CMSSignerDigestMismatchException;
import org.spongycastle.cms.DefaultCMSSignatureAlgorithmNameGenerator;
import org.spongycastle.cms.SignerInformation;
import org.spongycastle.cms.SignerInformationStore;
import org.spongycastle.cms.SignerInformationVerifier;
import org.spongycastle.cms.jcajce.JcaSimpleSignerInfoVerifierBuilder;
import org.spongycastle.jce.provider.BouncyCastleProvider;
import org.spongycastle.operator.DefaultSignatureAlgorithmIdentifierFinder;
import org.spongycastle.operator.OperatorCreationException;
import org.spongycastle.operator.bc.BcDigestCalculatorProvider;
import org.spongycastle.operator.jcajce.JcaContentVerifierProviderBuilder;
import org.spongycastle.tsp.TimeStampToken;
import org.spongycastle.tsp.TimeStampTokenInfo;
import org.spongycastle.util.Store;
import org.w3c.dom.Document;
import org.w3c.dom.Element;
import org.w3c.dom.Node;
import org.w3c.dom.NodeList;

import com.aowagie.text.pdf.AcroFields;
import com.aowagie.text.pdf.PdfArray;
import com.aowagie.text.pdf.PdfDictionary;
import com.aowagie.text.pdf.PdfIndirectReference;
import com.aowagie.text.pdf.PdfName;
import com.aowagie.text.pdf.PdfNumber;
import com.aowagie.text.pdf.PdfObject;
import com.aowagie.text.pdf.PdfPKCS7;
import com.aowagie.text.pdf.PdfReader;
import com.aowagie.text.pdf.PdfSignatureAppearance;

import es.gob.afirma.core.RuntimeConfigNeededException;
import es.gob.afirma.core.misc.Base64;
import es.gob.afirma.core.signers.AOSimpleSignInfo;
import es.gob.afirma.core.util.tree.AOTreeModel;
import es.gob.afirma.core.util.tree.AOTreeNode;
import es.gob.afirma.signers.cades.AOCAdESSigner;
import es.gob.afirma.signers.pades.PdfUtil;
import es.gob.afirma.signers.xades.AOXAdESSigner;
import es.gob.afirma.signers.xml.Utils;
import es.gob.afirma.signers.xml.XMLConstants;
import es.gob.afirma.signvalidation.CertHolderBySignerIdSelector;
import es.gob.afirma.signvalidation.DataAnalizerUtil;
import es.gob.afirma.signvalidation.SignValidity;
import es.gob.afirma.signvalidation.SignValidity.SIGN_DETAIL_TYPE;
import es.gob.afirma.signvalidation.SignValidity.VALIDITY_ERROR;
import es.gob.afirma.signvalidation.SignatureFormatDetectorPadesCades;
import es.gob.afirma.signvalidation.SignatureFormatDetectorXades;
import es.gob.afirma.signvalidation.ValidateBinarySignature;
import es.gob.afirma.signvalidation.ValidatePdfSignature;
import es.gob.afirma.signvalidation.ValidateXMLSignature;

/**
 * Las firmas que ya trae un PDF, un CAdES o un XAdES (FacturaE incluida), validadas
 * una a una con el validador por firma de AutoFirma 1.9.2, sin red y sin modo relajado.
 *
 * <p>En PDF salta los sellos de tiempo; una firma que iText no llega a leer, o
 * que no trae certificado de firma, sale no valida y danada (ADR-0043). Un PDF
 * ilegible o cifrado da un informe vacio en vez de un fallo. En CAdES juzga cada
 * SignerInfo y cada contrafirma, a cualquier profundidad, y comprueba su integridad
 * aunque el certificado haya caducado (ADR-0043). En XAdES cada
 * {@code ds:Signature}, contrafirmas incluidas, se juzga con su validador y con
 * la vigencia de todos los certificados de su KeyInfo; su estado viejo va nulo.
 */
final class PreviousSignaturesBridge {

    private static final Map<String, String> READABLE_KEYWORDS = Map.of(
            "2.5.4.5", "SERIALNUMBER",
            "2.5.4.97", "organizationIdentifier");

    /** La cabecera de un PDF y cuantos bytes del principio se miran buscandola. */
    private static final byte[] PDF_HEADER = "%PDF-".getBytes(StandardCharsets.US_ASCII);
    private static final int PDF_HEADER_WINDOW = 1024;

    private static final PdfName ETSI_RFC3161 = new PdfName("ETSI.RFC3161");

    private static final PdfName DOC_TIMESTAMP = new PdfName("DocTimeStamp");

    private static final PdfName DOC_MDP = new PdfName("DocMDP");

    /** Los {@code /SubFilter} que {@link SignatureFormatDetectorPadesCades#isPDF} reconoce como PAdES/CAdES. */
    private static final List<PdfName> RECOGNIZED_SUBFILTERS = List.of(
            new PdfName("adbe.pkcs7.detached"),
            new PdfName("adbe.pkcs7.sha1"),
            new PdfName("ETSI.CAdES.detached"));

    /** El mismo tope por defecto que trae el original en {@code pagesToCheckShadowAttack}. */
    private static final int PAGES_TO_COMPARE = 10;

    private static final int HIDDEN = 1 << 1;

    private static final int NO_VIEW = 1 << 5;

    private PreviousSignaturesBridge() { }

    /** La validez de una firma (ADR-0043), con el nombre con el que cruza a Rust. */
    enum Validity {
        VALID("valid"),
        EXPIRED("expired"),
        INVALID("invalid");

        private final String wireName;

        Validity(final String wireName) {
            this.wireName = wireName;
        }

        String wireName() {
            return wireName;
        }
    }

    /** Los motivos de la validez, del mas grave al menos grave, con su nombre en Rust. */
    enum Problem {
        DAMAGED("damaged", Validity.INVALID),
        MODIFIED_AFTER_SIGNING("modifiedAfterSigning", Validity.INVALID),
        COSIGN_NOT_ADMITTED("cosignNotAdmitted", Validity.INVALID),
        UNSUPPORTED_ALGORITHM("unsupportedAlgorithm", Validity.INVALID),
        UNKNOWN_SIGNATURE_TYPE("unknownSignatureType", Validity.INVALID),
        CERTIFICATE_NOT_YET_VALID("certificateNotYetValid", Validity.INVALID),
        CERTIFICATE_EXPIRED("certificateExpired", Validity.EXPIRED);

        private final String wireName;

        private final Validity validity;

        Problem(final String wireName, final Validity validity) {
            this.wireName = wireName;
            this.validity = validity;
        }

        String wireName() {
            return wireName;
        }

        Validity validity() {
            return validity;
        }
    }

    /** El motivo de la validez: la fecha y el titular del certificado, o quien cerro el documento. */
    record Reason(Problem problem, String date, String holder, String closedBy) {

        static Reason of(final Problem problem) {
            return new Reason(problem, null, null, null);
        }
    }

    /** Lo que se ve en el documento entero y no se cuelga de ninguna firma (ADR-0043). */
    enum Finding {
        MODIFIED_AFTER_LAST_SIGNATURE("modifiedAfterLastSignature"),
        FORM_FILLED_AFTER_SIGNING("formFilledAfterSigning"),
        CONTENT_ADDED_ON_TOP("contentAddedOnTop");

        private final String wireName;

        Finding(final String wireName) {
            this.wireName = wireName;
        }

        String wireName() {
            return wireName;
        }
    }

    /** La fecha de una firma: sellada si trae la TSA que la sello, declarada si {@code tsa} es nulo. */
    record SigningDate(String at, String tsa) {

        static SigningDate declared(final Date at) {
            return at == null ? null : new SigningDate(instant(at), null);
        }
    }

    /**
     * Titular, emisor, numero de serie, vigencia del certificado, algoritmo y perfil de la firma,
     * fecha, validez, motivo, fecha declarada o sellada, si cierra el documento y contrafirmas.
     */
    record Signature(String subject, String issuer, String serialNumber, String validFrom,
            String validUntil, String signatureAlgorithm, String profile, String signingTime,
            Validity validity, Reason validityReason,
            SigningDate signingDate, boolean closesDocument, List<Signature> countersignatures) { }

    /** Las firmas en orden cronologico, si el documento cambio despues de la ultima, y sus hallazgos. */
    record Report(List<Signature> signatures, boolean changedAfterLastSignature,
            List<Finding> findings) { }

    static Report read(final byte[] document) {
        final AOCAdESSigner cades = new AOCAdESSigner();
        if (cades.isSign(document)) {
            return new Report(cadesSigners(document), false, List.of());
        }
        if (isPdf(document)) {
            return readPdf(document);
        }
        final AOXAdESSigner xades = new AOXAdESSigner();
        if (xades.isSign(document)) {
            return new Report(xadesSigners(document), false, List.of());
        }
        return readPdf(document);
    }

    private static boolean isPdf(final byte[] document) {
        final int window = Math.min(document.length, PDF_HEADER_WINDOW + PDF_HEADER.length);
        for (int at = 0; at + PDF_HEADER.length <= window; at++) {
            if (Arrays.equals(document, at, at + PDF_HEADER.length, PDF_HEADER, 0, PDF_HEADER.length)) {
                return true;
            }
        }
        return false;
    }

    private static List<Signature> cadesSigners(final byte[] signature) {
        final CMSSignedData signed;
        try {
            signed = new CMSSignedData(signature);
        }
        catch (final CMSException e) {
            throw new IllegalStateException(e);
        }
        return judgedSigners(signed.getSignerInfos(), signed, signed.getSignedContent() != null);
    }

    private static List<Signature> judgedSigners(final SignerInformationStore signers,
            final CMSSignedData signed, final boolean withContent) {
        final List<Signature> judged = new ArrayList<>();
        for (final SignerInformation signer : signers.getSigners()) {
            judged.add(judged(signer, signed, withContent,
                    judgedSigners(signer.getCounterSignatures(), signed, true)));
        }
        return judged;
    }

    private static Signature judged(final SignerInformation signer, final CMSSignedData signed,
            final boolean withContent, final List<Signature> countersignatures) {
        final Store<X509CertificateHolder> certificates = signed.getCertificates();
        final X509Certificate certificate = certificateOf(signer, certificates);
        if (certificate == null) {
            return damaged(countersignatures);
        }
        final String profile = SignatureFormatDetectorPadesCades.resolveASN1Format(signed, signer);
        final List<SignValidity> validities = new ArrayList<>(ValidateBinarySignature.verifySign(
                signer, certificates, x509(), true, profile, withContent));
        if (validities.stream().anyMatch(PreviousSignaturesBridge::isOutOfDate)) {
            validities.add(integrityOf(signer, certificate, withContent));
        }
        if (usesBrokenCmsAlgorithm(signer)) {
            validities.add(unsupportedAlgorithm());
        }
        return identityOf(certificate,
                algorithmName(signer.getDigestAlgOID(), signer.getEncryptionAlgOID()), profile,
                signingTimeOf(signer), worstReason(validities, certificate, false, null),
                countersignatures);
    }

    static X509Certificate certificateOf(final SignerInformation signer,
            final Store<X509CertificateHolder> certificates) {
        try {
            for (final X509CertificateHolder holder
                    : certificates.getMatches(new CertHolderBySignerIdSelector(signer.getSID()))) {
                return (X509Certificate) x509().generateCertificate(
                        new ByteArrayInputStream(holder.getEncoded()));
            }
            return null;
        }
        catch (final IOException | CertificateException | RuntimeException e) {
            return null;
        }
    }

    static boolean isOutOfDate(final SignValidity validity) {
        return validity.getError() == VALIDITY_ERROR.CERTIFICATE_EXPIRED
            || validity.getError() == VALIDITY_ERROR.CERTIFICATE_NOT_VALID_YET;
    }

    /**
     * La comprobacion criptografica que {@code verifySign} se salta con el certificado fuera de
     * vigencia (ADR-0043). Verifica con la clave publica y no con el certificado, porque
     * SpongyCastle rechaza un certificado que no estaba en vigor en el {@code signingTime}.
     */
    static SignValidity integrityOf(final SignerInformation signer,
            final X509Certificate certificate, final boolean withContent) {
        try {
            final boolean verified = signer.verify(new SignerInformationVerifier(
                    new DefaultCMSSignatureAlgorithmNameGenerator(),
                    new DefaultSignatureAlgorithmIdentifierFinder(),
                    new JcaContentVerifierProviderBuilder()
                            .setProvider(new BouncyCastleProvider())
                            .build(certificate.getPublicKey()),
                    new BcDigestCalculatorProvider()));
            return verified
                    ? new SignValidity(SIGN_DETAIL_TYPE.OK, null)
                    : new SignValidity(SIGN_DETAIL_TYPE.KO, VALIDITY_ERROR.CANT_VALIDATE_CERT);
        }
        catch (final CMSSignerDigestMismatchException e) {
            return withContent
                    ? new SignValidity(SIGN_DETAIL_TYPE.KO, VALIDITY_ERROR.NO_MATCH_DATA)
                    : new SignValidity(SIGN_DETAIL_TYPE.OK, null);
        }
        catch (final CMSException | OperatorCreationException | RuntimeException e) {
            return new SignValidity(SIGN_DETAIL_TYPE.KO, VALIDITY_ERROR.CANT_VALIDATE_CERT);
        }
    }

    private static Date signingTimeOf(final SignerInformation signer) {
        if (signer.getSignedAttributes() == null) {
            return null;
        }
        final Attribute signingTime = signer.getSignedAttributes().get(CMSAttributes.signingTime);
        if (signingTime == null || signingTime.getAttrValues().size() == 0) {
            return null;
        }
        try {
            return Time.getInstance(signingTime.getAttrValues().getObjectAt(0)).getDate();
        }
        catch (final RuntimeException e) {
            return null;
        }
    }

    private static CertificateFactory x509() {
        try {
            return CertificateFactory.getInstance("X.509");
        }
        catch (final CertificateException e) {
            throw new IllegalStateException(e);
        }
    }

    private static Signature identityOf(final X509Certificate signer, final String algorithm,
            final String profile, final Date signingTime, final Reason worst,
            final List<Signature> countersignatures) {
        return new Signature(
                readable(signer.getSubjectX500Principal()),
                readable(signer.getIssuerX500Principal()),
                signer.getSerialNumber().toString(),
                instant(signer.getNotBefore()),
                instant(signer.getNotAfter()),
                algorithm,
                profile,
                signingTime == null
                        ? null
                        : DateTimeFormatter.ISO_INSTANT.format(signingTime.toInstant()),
                worst == null ? Validity.VALID : worst.problem().validity(),
                worst,
                SigningDate.declared(signingTime),
                false,
                countersignatures);
    }

    private static List<Signature> xadesSigners(final byte[] document) {
        final Document xml;
        final AOTreeModel tree;
        try {
            xml = Utils.getNewDocumentBuilder().parse(new ByteArrayInputStream(document));
            tree = AOXAdESSigner.getSignersStructure(xml, true);
        }
        catch (final Exception e) {
            throw new IllegalStateException(e);
        }
        final boolean externallyDetached =
                AOXAdESSigner.isExternallyDetached(xml.getDocumentElement());
        return xadesSignersUnder((AOTreeNode) tree.getRoot(), signatureElementsOf(xml),
                externallyDetached);
    }

    /** Cada {@code ds:Signature} por su {@code SignatureValue}, que es lo que guarda el arbol. */
    private static Map<String, Element> signatureElementsOf(final Document xml) {
        final Map<String, Element> elements = new HashMap<>();
        final NodeList signatures =
                xml.getElementsByTagNameNS(XMLConstants.DSIGNNS, "Signature");
        for (int i = 0; i < signatures.getLength(); i++) {
            final Element signature = (Element) signatures.item(i);
            final Element value = firstChild(signature, "SignatureValue");
            final byte[] decoded = value == null ? null : decoded(value.getTextContent());
            if (decoded != null) {
                elements.put(HexFormat.of().formatHex(decoded), signature);
            }
        }
        return elements;
    }

    private static byte[] decoded(final String base64) {
        try {
            return Base64.decode(base64);
        }
        catch (final IOException | RuntimeException e) {
            return null;
        }
    }

    private static List<Signature> xadesSignersUnder(final AOTreeNode parent,
            final Map<String, Element> elements, final boolean externallyDetached) {
        final List<Signature> signers = new ArrayList<>();
        for (int i = 0; i < parent.getChildCount(); i++) {
            final AOTreeNode node = parent.getChildAt(i);
            final AOSimpleSignInfo info = (AOSimpleSignInfo) node.getUserObject();
            final List<Signature> countersignatures =
                    xadesSignersUnder(node, elements, externallyDetached);
            final X509Certificate[] chain = info.getCerts();
            final Element element = info.getPkcs1() == null
                    ? null
                    : elements.get(HexFormat.of().formatHex(info.getPkcs1()));
            if (chain == null || chain.length == 0 || element == null) {
                signers.add(new Signature("", "", "", null, null, null, null, null,
                        Validity.INVALID, Reason.of(Problem.DAMAGED), null, false,
                        countersignatures));
                continue;
            }
            signers.add(identityOf(chain[0], xadesAlgorithmOf(element),
                    SignatureFormatDetectorXades.resolveSignerXAdESFormat(element),
                    info.getSigningTime(),
                    xadesWorstReason(element, chain[0], externallyDetached), countersignatures));
        }
        return signers;
    }

    /**
     * {@code ValidateXMLSignature.validateSign} no mira la vigencia de ningun certificado: se
     * comprueba aqui la de todos los del KeyInfo, como en su {@code validate} de documento.
     */
    private static Reason xadesWorstReason(final Element signature,
            final X509Certificate signer, final boolean externallyDetached) {
        final List<Reason> reasons = new ArrayList<>();
        for (final SignValidity validity : ValidateXMLSignature.validateSign(signature,
                SignatureFormatDetectorXades.resolveSignerXAdESFormat(signature),
                externallyDetached)) {
            final Reason reason = reasonOf(validity, signer, false, null);
            if (reason != null) {
                reasons.add(reason);
            }
        }
        reasons.addAll(keyInfoReasons(signature, signer));
        if (!XmlSignerKeyBinding.holds(signature, signer.getPublicKey())) {
            reasons.add(Reason.of(Problem.DAMAGED));
        }
        if (usesBrokenXmlDigest(signature)) {
            reasons.add(Reason.of(Problem.UNSUPPORTED_ALGORITHM));
        }
        return worstOf(reasons);
    }

    private static List<Reason> keyInfoReasons(final Element signature,
            final X509Certificate signer) {
        final List<Reason> reasons = new ArrayList<>();
        final Element keyInfo = firstChild(signature, "KeyInfo");
        if (keyInfo == null) {
            return reasons;
        }
        for (final Element data : children(keyInfo, "X509Data")) {
            for (final Element encoded : children(data, "X509Certificate")) {
                final X509Certificate certificate = Utils.getCertificate(encoded);
                final Reason reason = certificate == null
                        ? Reason.of(Problem.DAMAGED)
                        : validityOf(certificate, signer);
                if (reason != null) {
                    reasons.add(reason);
                }
            }
        }
        return reasons;
    }

    /** Si el certificado no esta en vigor, el motivo, que lo nombra cuando no es el del firmante. */
    private static Reason validityOf(final X509Certificate certificate, final X509Certificate signer) {
        try {
            certificate.checkValidity();
            return null;
        }
        catch (final CertificateExpiredException e) {
            return new Reason(Problem.CERTIFICATE_EXPIRED, instant(certificate.getNotAfter()),
                    certificate.equals(signer)
                            ? null
                            : readable(certificate.getSubjectX500Principal()),
                    null);
        }
        catch (final CertificateNotYetValidException e) {
            return new Reason(Problem.CERTIFICATE_NOT_YET_VALID,
                    instant(certificate.getNotBefore()), null, null);
        }
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

    private static Report readPdf(final byte[] pdf) {
        final PdfReader reader;
        final AcroFields fields;
        try {
            reader = PdfUtil.getPdfReader(pdf, headless(), true);
            fields = reader.getAcroFields();
        }
        catch (final Exception e) {
            return new Report(List.of(), false, List.of());
        }
        final String profile = SignatureFormatDetectorPadesCades.resolvePDFFormat(pdf);
        final Certification certification = certification(reader, fields);
        final String latestName = latestRevisionName(fields);

        final List<Dated> dated = new ArrayList<>();
        for (final String name : fields.getSignatureNames()) {
            if (isTimestamp(fields, name)) {
                continue;
            }
            final PdfPKCS7 pkcs7 = readableSignature(fields, name);
            if (pkcs7 == null || pkcs7.getSigningCertificate() == null) {
                dated.add(new Dated(null, damaged(List.of())));
                continue;
            }
            final X509Certificate signer = pkcs7.getSigningCertificate();
            final Instant signingTime =
                    pkcs7.getSignDate() == null ? null : pkcs7.getSignDate().toInstant();
            final Stamp stamp = stampOf(pkcs7);
            final List<SignValidity> validities = new ArrayList<>(validate(name, fields, profile));
            if (certification.forbids(fields.getRevision(name))) {
                validities.add(new SignValidity(SIGN_DETAIL_TYPE.KO,
                        VALIDITY_ERROR.CERTIFIED_SIGN_REVISION));
            }
            if (isBroken(pkcs7.getHashAlgorithm())) {
                validities.add(unsupportedAlgorithm());
            }
            final boolean unrecognizedSubFilter = hasUnrecognizedSubFilter(fields, name);
            final Reason worst = worstReason(
                    stamp == null ? validities : atStampTime(validities, signer, stamp.at()),
                    signer, unrecognizedSubFilter, certification.closedBy());
            dated.add(new Dated(signingTime, new Signature(
                    readable(signer.getSubjectX500Principal()),
                    readable(signer.getIssuerX500Principal()),
                    signer.getSerialNumber().toString(),
                    instant(signer.getNotBefore()),
                    instant(signer.getNotAfter()),
                    pkcs7.getDigestAlgorithm(),
                    name.equals(latestName) ? profile : null,
                    signingTime == null ? null : DateTimeFormatter.ISO_INSTANT.format(signingTime),
                    worst == null ? Validity.VALID : worst.problem().validity(),
                    worst,
                    stamp == null
                            ? SigningDate.declared(signingTime == null ? null : Date.from(signingTime))
                            : new SigningDate(instant(stamp.at()), stamp.tsa()),
                    name.equals(certification.name()),
                    List.of())));
        }
        dated.sort(Comparator.comparing(Dated::signingTime,
                Comparator.nullsLast(Comparator.naturalOrder())));
        final Finding suspect = changedAfterLastSignature(reader, fields);
        return new Report(dated.stream().map(Dated::signature).toList(),
                suspect != null, findings(reader, fields, suspect));
    }

    /** La firma de la revision mas alta, que es la ultima: {@code getSignatureNames} no tiene orden. */
    static String latestRevisionName(final AcroFields fields) {
        String latest = null;
        for (final String name : fields.getSignatureNames()) {
            if (latest == null || fields.getRevision(name) > fields.getRevision(latest)) {
                latest = name;
            }
        }
        return latest;
    }

    private static PdfPKCS7 readableSignature(final AcroFields fields, final String name) {
        try {
            return fields.verifySignature(name);
        }
        catch (final RuntimeException e) {
            return null;
        }
    }

    private static Signature damaged(final List<Signature> countersignatures) {
        return new Signature("", "", "", null, null, null, null, null, Validity.INVALID,
                Reason.of(Problem.DAMAGED), null, false, countersignatures);
    }

    /** El problema mas grave de una firma, o {@code null} si no tiene ninguno (ADR-0043). */
    static Reason worstReason(final List<SignValidity> validities, final X509Certificate signer,
            final boolean unrecognizedSubFilter, final String closedBy) {
        final List<Reason> reasons = new ArrayList<>();
        for (final SignValidity validity : validities) {
            final Reason reason = reasonOf(validity, signer, unrecognizedSubFilter, closedBy);
            if (reason != null) {
                reasons.add(reason);
            }
        }
        return worstOf(reasons);
    }

    private static Reason worstOf(final List<Reason> reasons) {
        Reason worst = null;
        for (final Reason reason : reasons) {
            if (worst == null || reason.problem().ordinal() < worst.problem().ordinal()) {
                worst = reason;
            }
        }
        return worst;
    }

    private static Reason reasonOf(final SignValidity validity, final X509Certificate signer,
            final boolean unrecognizedSubFilter, final String closedBy) {
        if (SIGN_DETAIL_TYPE.OK == validity.getValidity() || validity.getError() == null) {
            return null;
        }
        return switch (validity.getError()) {
            case CERTIFICATE_EXPIRED -> new Reason(Problem.CERTIFICATE_EXPIRED,
                    instant(signer.getNotAfter()), null, null);
            case CERTIFICATE_NOT_VALID_YET -> new Reason(Problem.CERTIFICATE_NOT_YET_VALID,
                    instant(signer.getNotBefore()), null, null);
            case NO_MATCH_DATA -> Reason.of(Problem.MODIFIED_AFTER_SIGNING);
            case CERTIFIED_SIGN_REVISION -> new Reason(Problem.COSIGN_NOT_ADMITTED,
                    null, null, closedBy);
            case SIGN_PROFILE_NOT_CHECKED -> unrecognizedSubFilter
                    ? Reason.of(Problem.UNKNOWN_SIGNATURE_TYPE)
                    : null;
            case ALGORITHM_NOT_SUPPORTED -> Reason.of(Problem.UNSUPPORTED_ALGORITHM);
            case UNKOWN_SIGNATURE_FORMAT -> Reason.of(Problem.UNKNOWN_SIGNATURE_TYPE);
            default -> Reason.of(Problem.DAMAGED);
        };
    }

    private static final Set<String> BROKEN_DIGEST_NAMES = Set.of("MD2", "MD5");

    private static final Set<String> BROKEN_DIGEST_OIDS =
            Set.of("1.2.840.113549.2.2", "1.2.840.113549.2.5",
                    "1.2.840.113549.1.1.2", "1.2.840.113549.1.1.4");

    private static final Pattern BROKEN_XML_DIGEST = Pattern.compile("(?i)[#/-](md2|md5)$");

    /** Si el resumen es MD5 o MD2, por su OID (tambien el compuesto md5WithRSA o md2WithRSA) o por su nombre. */
    static boolean isBroken(final String digest) {
        return digest != null && (BROKEN_DIGEST_OIDS.contains(digest)
                || BROKEN_DIGEST_NAMES.contains(digest.toUpperCase(Locale.ROOT).replace("-", "")));
    }

    /** Si el resumen o el algoritmo de firma del firmante es MD5 o MD2. */
    static boolean usesBrokenCmsAlgorithm(final SignerInformation signer) {
        return isBroken(signer.getDigestAlgOID()) || isBroken(signer.getEncryptionAlgOID());
    }

    private static SignValidity unsupportedAlgorithm() {
        return new SignValidity(SIGN_DETAIL_TYPE.KO, VALIDITY_ERROR.ALGORITHM_NOT_SUPPORTED);
    }

    /** Si el {@code ds:SignedInfo} propio de la firma declara MD5 o MD2, en su firma o en sus referencias. */
    static boolean usesBrokenXmlDigest(final Element signature) {
        final Element signedInfo = firstChild(signature, "SignedInfo");
        if (signedInfo == null) {
            return false;
        }
        final List<Element> methods = new ArrayList<>(children(signedInfo, "SignatureMethod"));
        for (final Element reference : children(signedInfo, "Reference")) {
            methods.addAll(children(reference, "DigestMethod"));
        }
        return methods.stream().anyMatch(method ->
                BROKEN_XML_DIGEST.matcher(method.getAttribute("Algorithm")).find());
    }

    /** Si alguna firma del documento, contrafirmas incluidas, resume con MD5 o MD2. */
    static boolean usesBrokenDigest(final byte[] document) {
        return read(document).signatures().stream().anyMatch(PreviousSignaturesBridge::isBrokenTree);
    }

    private static boolean isBrokenTree(final Signature signature) {
        return signature.validityReason() != null
                && signature.validityReason().problem() == Problem.UNSUPPORTED_ALGORITHM
                || signature.countersignatures().stream()
                        .anyMatch(PreviousSignaturesBridge::isBrokenTree);
    }

    private static final Map<String, String> DIGESTS = Map.of(
            "1.3.14.3.2.26", "SHA1",
            "2.16.840.1.101.3.4.2.4", "SHA224",
            "2.16.840.1.101.3.4.2.1", "SHA256",
            "2.16.840.1.101.3.4.2.2", "SHA384",
            "2.16.840.1.101.3.4.2.3", "SHA512");

    private static final String RSASSA_PSS_OID = "1.2.840.113549.1.1.10";

    private static final Pattern XADES_ALGORITHM = Pattern.compile("(rsa|ecdsa)-(sha\\d+)$");

    /** El nombre {@code SHA256withRSA} de un par digest y cifrado dados por su OID, o los OID si no se conocen. */
    static String algorithmName(final String digestOid, final String encryptionOid) {
        final String digest = DIGESTS.get(digestOid);
        final String encryption = encryptionOid.startsWith("1.2.840.113549.1.1.")
                && !RSASSA_PSS_OID.equals(encryptionOid) ? "RSA"
                : encryptionOid.startsWith("1.2.840.10045.") ? "ECDSA" : null;
        return digest == null || encryption == null
                ? digestOid + "/" + encryptionOid
                : digest + "with" + encryption;
    }

    /** El {@code rsa-sha256} de {@code ds:SignatureMethod}, dicho como {@code SHA256withRSA}. */
    static String xadesAlgorithmOf(final Element signature) {
        final NodeList methods = signature.getElementsByTagNameNS(
                XMLConstants.DSIGNNS, "SignatureMethod");
        if (methods.getLength() == 0) {
            return null;
        }
        final String uri = ((Element) methods.item(0)).getAttribute("Algorithm");
        final Matcher named = XADES_ALGORITHM.matcher(uri);
        return named.find()
                ? named.group(2).toUpperCase(Locale.ROOT) + "with"
                        + named.group(1).toUpperCase(Locale.ROOT)
                : uri;
    }

    private static String instant(final Date date) {
        return DateTimeFormatter.ISO_INSTANT.format(date.toInstant());
    }

    /** El sello de tiempo de una firma: cuando la sello la TSA y quien es la TSA. */
    private record Stamp(Date at, String tsa) { }

    /**
     * El sello de tiempo de los atributos sin firmar, o {@code null} si no trae ninguno, si no es
     * integro o si no sella esta firma (ADR-0043). No comprueba que la TSA sea de confianza.
     */
    private static Stamp stampOf(final PdfPKCS7 pkcs7) {
        final TimeStampToken token = pkcs7.getTimeStampToken();
        if (token == null) {
            return null;
        }
        try {
            final TimeStampTokenInfo info = token.getTimeStampInfo();
            final byte[] imprint = MessageDigest.getInstance(info.getMessageImprintAlgOID().getId())
                    .digest(pkcs7.getPkcs1());
            final X509CertificateHolder authority = authorityOf(token);
            if (authority == null || !MessageDigest.isEqual(imprint, info.getMessageImprintDigest())) {
                return null;
            }
            token.validate(new JcaSimpleSignerInfoVerifierBuilder().build(authority));
            return new Stamp(info.getGenTime(),
                    readable(new X500Principal(authority.getSubject().getEncoded())));
        }
        catch (final Exception e) {
            return null;
        }
    }

    @SuppressWarnings("unchecked")
    private static X509CertificateHolder authorityOf(final TimeStampToken token) {
        final Collection<X509CertificateHolder> matches =
                token.getCertificates().getMatches(token.getSID());
        return matches.isEmpty() ? null : matches.iterator().next();
    }

    /** Los veredictos con la vigencia del certificado medida en la fecha del sello (ADR-0043). */
    static List<SignValidity> atStampTime(final List<SignValidity> validities,
            final X509Certificate signer, final Date stampedAt) {
        final List<SignValidity> atStamp = new ArrayList<>();
        for (final SignValidity validity : validities) {
            if (validity.getError() != VALIDITY_ERROR.CERTIFICATE_EXPIRED
                    && validity.getError() != VALIDITY_ERROR.CERTIFICATE_NOT_VALID_YET) {
                atStamp.add(validity);
            }
        }
        if (stampedAt.after(signer.getNotAfter())) {
            atStamp.add(new SignValidity(SIGN_DETAIL_TYPE.KO, VALIDITY_ERROR.CERTIFICATE_EXPIRED));
        }
        else if (stampedAt.before(signer.getNotBefore())) {
            atStamp.add(new SignValidity(SIGN_DETAIL_TYPE.KO,
                    VALIDITY_ERROR.CERTIFICATE_NOT_VALID_YET));
        }
        return atStamp;
    }

    /**
     * El PDF Shadow Attack del original sin pintar las paginas: su {@code checkPdfShadowAttack}
     * las rasteriza con AWT, que no entra en la imagen nativa (ADR-0004). Aqui, pagina a pagina,
     * una anotacion nueva o movida que se solapa con otra visible es contenido encima, y un flujo
     * de contenido distinto del de la ultima revision firmada es una modificacion.
     */
    private static Finding changedAfterLastSignature(final PdfReader current,
            final AcroFields fields) {
        final String last = latestRevisionName(fields);
        if (last == null || fields.getRevision(last) >= fields.getTotalRevisions()) {
            return null;
        }
        try (InputStream lastSignedRevision = fields.extractRevision(last)) {
            final PdfReader signed = new PdfReader(lastSignedRevision);
            final int pages = Math.min(current.getNumberOfPages(), PAGES_TO_COMPARE);
            for (int page = 1; page <= pages; page++) {
                if (page > signed.getNumberOfPages()) {
                    return Finding.MODIFIED_AFTER_LAST_SIGNATURE;
                }
                if (laysNewAnnotationOverAnother(visibleAnnotations(signed, page),
                        visibleAnnotations(current, page))) {
                    return Finding.CONTENT_ADDED_ON_TOP;
                }
                if (!Arrays.equals(signed.getPageContent(page, signed.getSafeFile()),
                        current.getPageContent(page, current.getSafeFile()))) {
                    return Finding.MODIFIED_AFTER_LAST_SIGNATURE;
                }
            }
            return null;
        }
        catch (final IOException | RuntimeException e) {
            return null;
        }
    }

    private static boolean laysNewAnnotationOverAnother(final List<Annotation> signed,
            final List<Annotation> current) {
        for (int i = 0; i < current.size(); i++) {
            for (int j = i + 1; j < current.size(); j++) {
                final Annotation one = current.get(i);
                final Annotation other = current.get(j);
                final boolean involvesANewOne = !signed.contains(one) || !signed.contains(other);
                if (involvesANewOne && one.overlaps(other)) {
                    return true;
                }
            }
        }
        return false;
    }

    /** Una anotacion visible: su referencia (vacia si es directa) y su recuadro normalizado. */
    private record Annotation(String reference, float left, float bottom, float right,
            float top) {

        boolean overlaps(final Annotation other) {
            return left < other.right && other.left < right
                && bottom < other.top && other.bottom < top;
        }
    }

    private static List<Annotation> visibleAnnotations(final PdfReader reader, final int page) {
        final PdfArray annotations = reader.getPageN(page).getAsArray(PdfName.ANNOTS);
        final List<Annotation> visible = new ArrayList<>();
        if (annotations == null) {
            return visible;
        }
        for (int i = 0; i < annotations.size(); i++) {
            final PdfObject raw = annotations.getPdfObject(i);
            final PdfObject annotation = PdfReader.getPdfObject(raw);
            if (annotation instanceof PdfDictionary dictionary && !isHidden(dictionary)) {
                final Annotation box = annotationOf(referenceOf(raw),
                        dictionary.getAsArray(PdfName.RECT));
                if (box != null) {
                    visible.add(box);
                }
            }
        }
        return visible;
    }

    private static boolean isHidden(final PdfDictionary annotation) {
        final PdfNumber flags = annotation.getAsNumber(PdfName.F);
        return flags != null && (flags.intValue() & (HIDDEN | NO_VIEW)) != 0;
    }

    private static String referenceOf(final PdfObject raw) {
        return raw instanceof PdfIndirectReference reference
            ? reference.getNumber() + " " + reference.getGeneration()
            : "";
    }

    /** La anotacion con su recuadro normalizado, o {@code null} si no tiene area (invisible). */
    private static Annotation annotationOf(final String reference, final PdfArray rect) {
        if (rect == null || rect.size() != 4) {
            return null;
        }
        final float[] corners = new float[4];
        for (int i = 0; i < 4; i++) {
            final PdfNumber number = rect.getAsNumber(i);
            if (number == null) {
                return null;
            }
            corners[i] = number.floatValue();
        }
        final Annotation box = new Annotation(reference,
            Math.min(corners[0], corners[2]), Math.min(corners[1], corners[3]),
            Math.max(corners[0], corners[2]), Math.max(corners[1], corners[3]));
        return box.right() - box.left() == 0 || box.top() - box.bottom() == 0 ? null : box;
    }

    /** Como en el validador del original, el formulario cambiado tapa al PDF Shadow Attack. */
    private static List<Finding> findings(final PdfReader reader, final AcroFields fields,
            final Finding suspect) {
        if (formFilledAfterSigning(reader, fields)) {
            return List.of(Finding.FORM_FILLED_AFTER_SIGNING);
        }
        return suspect == null ? List.of() : List.of(suspect);
    }

    private static boolean formFilledAfterSigning(final PdfReader reader,
            final AcroFields fields) {
        if (fields.getSignatureNames().isEmpty() || fields.getTotalRevisions() <= 1) {
            return false;
        }
        try {
            final Map<String, String> changed = DataAnalizerUtil.checkPDFForm(reader);
            return changed != null && !changed.isEmpty();
        }
        catch (final IOException | RuntimeException e) {
            return false;
        }
    }

    static String readable(final X500Principal name) {
        return name.getName(X500Principal.RFC2253, READABLE_KEYWORDS);
    }

    private record Dated(Instant signingTime, Signature signature) { }

    private static Properties headless() {
        final Properties options = new Properties();
        options.setProperty("headless", Boolean.TRUE.toString());
        return options;
    }

    /**
     * La firma que certifico el PDF «sin cambios permitidos»: su campo, su revision y quien la
     * firmo; sin campo y con la revision 0 si el PDF no esta cerrado.
     */
    private record Certification(String name, int revision, String closedBy) {

        static final Certification NONE = new Certification(null, 0, null);

        /** Sin firma de certificacion localizada no se prohibe nada: el {@code rev <= 0} del original. */
        boolean forbids(final int signatureRevision) {
            return revision > 0 && signatureRevision > revision;
        }
    }

    /** Con varias firmas de certificacion cuenta la ultima, la de la revision mas alta (ADR-0043). */
    private static Certification certification(final PdfReader reader, final AcroFields fields) {
        if (reader.getCertificationLevel()
                != PdfSignatureAppearance.CERTIFIED_NO_CHANGES_ALLOWED) {
            return Certification.NONE;
        }
        String last = null;
        for (final String name : fields.getSignatureNames()) {
            if (isCertification(fields.getSignatureDictionary(name))
                    && (last == null || fields.getRevision(name) > fields.getRevision(last))) {
                last = name;
            }
        }
        if (last == null) {
            return Certification.NONE;
        }
        final PdfPKCS7 closer = readableSignature(fields, last);
        return new Certification(last, fields.getRevision(last),
                closer == null || closer.getSigningCertificate() == null
                        ? null
                        : readable(closer.getSigningCertificate().getSubjectX500Principal()));
    }

    private static boolean isCertification(final PdfDictionary signature) {
        final PdfArray references = signature.getAsArray(PdfName.REFERENCE);
        if (references == null) {
            return false;
        }
        for (int i = 0; i < references.size(); i++) {
            if (PdfReader.getPdfObject(references.getPdfObject(i)) instanceof PdfDictionary reference
                    && DOC_MDP.equals(reference.get(PdfName.TRANSFORMMETHOD))) {
                return true;
            }
        }
        return false;
    }

    private static boolean isTimestamp(final AcroFields fields, final String name) {
        final Object subFilter = fields.getSignatureDictionary(name).get(PdfName.SUBFILTER);
        return ETSI_RFC3161.equals(subFilter) || DOC_TIMESTAMP.equals(subFilter);
    }

    private static boolean hasUnrecognizedSubFilter(final AcroFields fields, final String name) {
        final Object subFilter = fields.getSignatureDictionary(name).get(PdfName.SUBFILTER);
        return !RECOGNIZED_SUBFILTERS.contains(subFilter);
    }

    private static List<SignValidity> validate(final String name, final AcroFields fields,
            final String profile) {
        try {
            return ValidatePdfSignature.validateSign(name, fields, profile, true);
        }
        catch (final IOException | RuntimeConfigNeededException e) {
            return List.of(new SignValidity(SIGN_DETAIL_TYPE.KO, VALIDITY_ERROR.UNKOWN_ERROR));
        }
    }
}
