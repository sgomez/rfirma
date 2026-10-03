package es.gob.afirma.nativebridge;

import java.io.IOException;
import java.io.InputStream;
import java.security.cert.X509Certificate;
import java.time.Instant;
import java.time.format.DateTimeFormatter;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Comparator;
import java.util.Date;
import java.util.List;
import java.util.Map;
import java.util.Properties;

import javax.security.auth.x500.X500Principal;

import com.aowagie.text.pdf.AcroFields;
import com.aowagie.text.pdf.PdfArray;
import com.aowagie.text.pdf.PdfDictionary;
import com.aowagie.text.pdf.PdfName;
import com.aowagie.text.pdf.PdfNumber;
import com.aowagie.text.pdf.PdfObject;
import com.aowagie.text.pdf.PdfPKCS7;
import com.aowagie.text.pdf.PdfReader;
import com.aowagie.text.pdf.PdfSignatureAppearance;

import es.gob.afirma.core.AOException;
import es.gob.afirma.core.RuntimeConfigNeededException;
import es.gob.afirma.core.signers.AOSigner;
import es.gob.afirma.core.signers.AOSimpleSignInfo;
import es.gob.afirma.core.util.tree.AOTreeModel;
import es.gob.afirma.core.util.tree.AOTreeNode;
import es.gob.afirma.signers.cades.AOCAdESSigner;
import es.gob.afirma.signers.pades.PdfUtil;
import es.gob.afirma.signers.xades.AOXAdESSigner;
import es.gob.afirma.signvalidation.DataAnalizerUtil;
import es.gob.afirma.signvalidation.SignValidity;
import es.gob.afirma.signvalidation.SignValidity.SIGN_DETAIL_TYPE;
import es.gob.afirma.signvalidation.SignValidity.VALIDITY_ERROR;
import es.gob.afirma.signvalidation.SignatureFormatDetectorPadesCades;
import es.gob.afirma.signvalidation.ValidatePdfSignature;

/**
 * Las firmas que ya trae un PDF, un CAdES o un XAdES (FacturaE incluida), recorridas con el
 * {@code getSignersStructure} de AutoFirma 1.9.2; las de PDF, ademas,
 * validadas una a una con su validador, sin red y sin modo relajado.
 *
 * <p>En PDF salta los sellos de tiempo; una firma que iText no llega a leer, o
 * que no trae certificado de firma, sale no valida y danada (ADR-0043). Un PDF
 * ilegible o cifrado da un informe vacio en vez de un fallo. En CAdES y XAdES
 * solo lee la identidad de cada firmante y de sus contrafirmas, a cualquier
 * profundidad: su estado va nulo y su validez, valida, hasta que se juzguen.
 */
final class PreviousSignaturesBridge {

    private static final Map<String, String> READABLE_KEYWORDS = Map.of(
            "2.5.4.5", "SERIALNUMBER",
            "2.5.4.97", "organizationIdentifier");

    private static final PdfName ETSI_RFC3161 = new PdfName("ETSI.RFC3161");

    private static final PdfName DOC_TIMESTAMP = new PdfName("DocTimeStamp");

    /** Los {@code /SubFilter} que {@link SignatureFormatDetectorPadesCades#isPDF} reconoce como PAdES/CAdES. */
    private static final List<PdfName> RECOGNIZED_SUBFILTERS = List.of(
            new PdfName("adbe.pkcs7.detached"),
            new PdfName("adbe.pkcs7.sha1"),
            new PdfName("ETSI.CAdES.detached"));

    /** El mismo tope por defecto que trae el original en {@code pagesToCheckShadowAttack}. */
    private static final int PAGES_TO_COMPARE = 10;

    private PreviousSignaturesBridge() { }

    /** El estado de una firma previa, con el nombre con el que cruza a Rust. */
    enum Status {
        VALID("valid"),
        CERTIFICATE_EXPIRED("certificateExpired"),
        CERTIFICATE_NOT_YET_VALID("certificateNotYetValid"),
        BROKEN("broken"),
        UNVERIFIABLE("unverifiable"),
        NOT_FULLY_CHECKED("notFullyChecked");

        private final String wireName;

        Status(final String wireName) {
            this.wireName = wireName;
        }

        String wireName() {
            return wireName;
        }
    }

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

    /** Titular, emisor, numero de serie, fecha, estado y motivo viejos, validez, motivo y contrafirmas. */
    record Signature(String subject, String issuer, String serialNumber, String signingTime,
            Status status, String reason, Validity validity, Reason validityReason,
            List<Signature> countersignatures) { }

    /** Las firmas en orden cronologico, si el documento cambio despues de la ultima, y sus hallazgos. */
    record Report(List<Signature> signatures, boolean changedAfterLastSignature,
            List<Finding> findings) { }

    static Report read(final byte[] document) {
        final AOCAdESSigner cades = new AOCAdESSigner();
        if (cades.isSign(document)) {
            return new Report(signersOf(cades, document, "CAdES"), false, List.of());
        }
        final AOXAdESSigner xades = new AOXAdESSigner();
        if (xades.isSign(document)) {
            return new Report(signersOf(xades, document, "XAdES"), false, List.of());
        }
        return readPdf(document);
    }

    private static List<Signature> signersOf(final AOSigner signer, final byte[] signature,
            final String format) {
        final AOTreeModel tree;
        try {
            tree = signer.getSignersStructure(signature, true);
        }
        catch (final AOException | IOException e) {
            throw new IllegalStateException(e);
        }
        if (tree == null) {
            throw new IllegalStateException(
                    "no se ha podido leer el arbol de firmantes del " + format);
        }
        return signersUnder((AOTreeNode) tree.getRoot());
    }

    private static List<Signature> signersUnder(final AOTreeNode parent) {
        final List<Signature> signers = new ArrayList<>();
        for (int i = 0; i < parent.getChildCount(); i++) {
            final AOTreeNode node = parent.getChildAt(i);
            final AOSimpleSignInfo info = (AOSimpleSignInfo) node.getUserObject();
            final X509Certificate[] chain = info.getCerts();
            if (chain == null || chain.length == 0) {
                continue;
            }
            signers.add(identityOf(chain[0], info.getSigningTime(), signersUnder(node)));
        }
        return signers;
    }

    private static Signature identityOf(final X509Certificate signer, final Date signingTime,
            final List<Signature> countersignatures) {
        return new Signature(
                readable(signer.getSubjectX500Principal()),
                readable(signer.getIssuerX500Principal()),
                signer.getSerialNumber().toString(),
                signingTime == null
                        ? null
                        : DateTimeFormatter.ISO_INSTANT.format(signingTime.toInstant()),
                null,
                null,
                Validity.VALID,
                null,
                countersignatures);
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

        final List<Dated> dated = new ArrayList<>();
        for (final String name : fields.getSignatureNames()) {
            if (isTimestamp(fields, name)) {
                continue;
            }
            final PdfPKCS7 pkcs7 = readableSignature(fields, name);
            if (pkcs7 == null || pkcs7.getSigningCertificate() == null) {
                dated.add(new Dated(null, damaged()));
                continue;
            }
            final X509Certificate signer = pkcs7.getSigningCertificate();
            final Instant signingTime =
                    pkcs7.getSignDate() == null ? null : pkcs7.getSignDate().toInstant();
            final List<SignValidity> validities = new ArrayList<>(validate(name, fields, profile));
            if (certification.revision() > 0
                    && fields.getRevision(name) > certification.revision()) {
                validities.add(new SignValidity(SIGN_DETAIL_TYPE.KO,
                        VALIDITY_ERROR.CERTIFIED_SIGN_REVISION));
            }
            final boolean unrecognizedSubFilter = hasUnrecognizedSubFilter(fields, name);
            final SignValidity validity = withUnrecognizedFormat(
                    unrecognizedSubFilter, decisive(validities));
            final Reason worst = worstReason(validities, signer, unrecognizedSubFilter,
                    certification.closedBy());
            dated.add(new Dated(signingTime, new Signature(
                    readable(signer.getSubjectX500Principal()),
                    readable(signer.getIssuerX500Principal()),
                    signer.getSerialNumber().toString(),
                    signingTime == null ? null : DateTimeFormatter.ISO_INSTANT.format(signingTime),
                    statusOf(validity),
                    reasonOf(validity),
                    worst == null ? Validity.VALID : worst.problem().validity(),
                    worst,
                    List.of())));
        }
        dated.sort(Comparator.comparing(Dated::signingTime,
                Comparator.nullsLast(Comparator.naturalOrder())));
        final Finding suspect = changedAfterLastSignature(reader, fields);
        return new Report(dated.stream().map(Dated::signature).toList(),
                suspect != null, findings(reader, fields, suspect));
    }

    private static PdfPKCS7 readableSignature(final AcroFields fields, final String name) {
        try {
            return fields.verifySignature(name);
        }
        catch (final RuntimeException e) {
            return null;
        }
    }

    private static Signature damaged() {
        return new Signature("", "", "", null, Status.BROKEN,
                VALIDITY_ERROR.CORRUPTED_SIGN.name(), Validity.INVALID,
                Reason.of(Problem.DAMAGED), List.of());
    }

    /** El problema mas grave de una firma, o {@code null} si no tiene ninguno (ADR-0043). */
    static Reason worstReason(final List<SignValidity> validities, final X509Certificate signer,
            final boolean unrecognizedSubFilter, final String closedBy) {
        Reason worst = null;
        for (final SignValidity validity : validities) {
            final Reason reason = reasonOf(validity, signer, unrecognizedSubFilter, closedBy);
            if (reason != null
                    && (worst == null || reason.problem().ordinal() < worst.problem().ordinal())) {
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
            case ALGORITHM_NOT_SUPPORTED, UNKOWN_SIGNATURE_FORMAT ->
                    Reason.of(Problem.UNKNOWN_SIGNATURE_TYPE);
            default -> Reason.of(Problem.DAMAGED);
        };
    }

    private static String instant(final Date date) {
        return DateTimeFormatter.ISO_INSTANT.format(date.toInstant());
    }

    /**
     * El PDF Shadow Attack del original sin pintar las paginas: su {@code checkPdfShadowAttack}
     * las rasteriza con AWT, que no entra en la imagen nativa (ADR-0004). Aqui, pagina a pagina,
     * dos anotaciones visibles que se solapan son contenido encima, y un flujo de contenido
     * distinto del de la ultima revision firmada es una modificacion.
     */
    private static Finding changedAfterLastSignature(final PdfReader current,
            final AcroFields fields) {
        final List<String> names = fields.getSignatureNames();
        if (names.isEmpty() || fields.getRevision(names.get(0)) >= fields.getTotalRevisions()) {
            return null;
        }
        try (InputStream lastSignedRevision = fields.extractRevision(names.get(0))) {
            final PdfReader signed = new PdfReader(lastSignedRevision);
            final int pages = Math.min(current.getNumberOfPages(), PAGES_TO_COMPARE);
            for (int page = 1; page <= pages; page++) {
                if (hasOverlappingAnnotations(current, page)) {
                    return Finding.CONTENT_ADDED_ON_TOP;
                }
                if (page > signed.getNumberOfPages() || !Arrays.equals(
                        signed.getPageContent(page, signed.getSafeFile()),
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

    private static boolean hasOverlappingAnnotations(final PdfReader reader, final int page) {
        final PdfArray annotations = reader.getPageN(page).getAsArray(PdfName.ANNOTS);
        if (annotations == null) {
            return false;
        }
        final List<float[]> visible = new ArrayList<>();
        for (int i = 0; i < annotations.size(); i++) {
            final PdfObject annotation = PdfReader.getPdfObject(annotations.getPdfObject(i));
            if (!(annotation instanceof PdfDictionary)) {
                continue;
            }
            final float[] box = boxOf(((PdfDictionary) annotation).getAsArray(PdfName.RECT));
            if (box == null) {
                continue;
            }
            for (final float[] other : visible) {
                if (box[0] <= other[2] && other[0] <= box[2]
                        && box[1] <= other[3] && other[1] <= box[3]) {
                    return true;
                }
            }
            visible.add(box);
        }
        return false;
    }

    /** El recuadro normalizado de una anotacion, o {@code null} si no tiene area (invisible). */
    private static float[] boxOf(final PdfArray rect) {
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
        final float[] box = {
            Math.min(corners[0], corners[2]), Math.min(corners[1], corners[3]),
            Math.max(corners[0], corners[2]), Math.max(corners[1], corners[3])};
        return box[2] - box[0] == 0 || box[3] - box[1] == 0 ? null : box;
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

    static Status statusOf(final SignValidity validity) {
        if (validity.getError() == null) {
            return Status.VALID;
        }
        return switch (validity.getError()) {
            case CERTIFICATE_EXPIRED -> Status.CERTIFICATE_EXPIRED;
            case CERTIFICATE_NOT_VALID_YET -> Status.CERTIFICATE_NOT_YET_VALID;
            case NO_MATCH_DATA, CORRUPTED_SIGN, CERTIFIED_SIGN_REVISION -> Status.BROKEN;
            case SIGN_PROFILE_NOT_CHECKED -> Status.NOT_FULLY_CHECKED;
            default -> Status.UNVERIFIABLE;
        };
    }

    private record Dated(Instant signingTime, Signature signature) { }

    private static Properties headless() {
        final Properties options = new Properties();
        options.setProperty("headless", Boolean.TRUE.toString());
        return options;
    }

    /** La revision que certifico el PDF «sin cambios permitidos» y quien la firmo, o 0 y nadie. */
    private record Certification(int revision, String closedBy) { }

    private static Certification certification(final PdfReader reader, final AcroFields fields) {
        if (reader.getCertificationLevel()
                != PdfSignatureAppearance.CERTIFIED_NO_CHANGES_ALLOWED) {
            return new Certification(0, null);
        }
        for (final String name : fields.getSignatureNames()) {
            final PdfDictionary signature = fields.getSignatureDictionary(name);
            final Object reference = signature.get(PdfName.REFERENCE);
            if (!(reference instanceof PdfArray)) {
                continue;
            }
            final Object first = ((PdfArray) reference).getArrayList().get(0);
            if (first instanceof PdfDictionary
                    && ((PdfDictionary) first).get(PdfName.TRANSFORMMETHOD) != null) {
                final PdfPKCS7 closer = readableSignature(fields, name);
                return new Certification(fields.getRevision(name),
                        closer == null || closer.getSigningCertificate() == null
                                ? null
                                : readable(closer.getSigningCertificate().getSubjectX500Principal()));
            }
        }
        return new Certification(0, null);
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

    /**
     * Un {@code KO} pesa mas que un {@code UNKNOWN}, como en el validador del original, salvo
     * cuando el unico {@code KO} es de certificado caducado y hay un aviso de perfil longevo:
     * ese aviso pesa mas ({@code SignValider#checkLongStandingValiditySign} del original).
     */
    private static SignValidity decisive(final List<SignValidity> validities) {
        SignValidity decisive = new SignValidity(SIGN_DETAIL_TYPE.OK, null);
        SignValidity expiredCertificateKo = null;
        SignValidity longStandingWarning = null;
        for (final SignValidity validity : validities) {
            if (SIGN_DETAIL_TYPE.KO == validity.getValidity()) {
                if (VALIDITY_ERROR.CERTIFICATE_EXPIRED != validity.getError()) {
                    return validity;
                }
                expiredCertificateKo = validity;
            }
            else if (SIGN_DETAIL_TYPE.UNKNOWN == validity.getValidity()) {
                if (VALIDITY_ERROR.SIGN_PROFILE_NOT_CHECKED == validity.getError()) {
                    longStandingWarning = validity;
                }
                else {
                    decisive = validity;
                }
            }
        }
        if (expiredCertificateKo != null) {
            return longStandingWarning != null ? longStandingWarning : expiredCertificateKo;
        }
        return longStandingWarning != null ? longStandingWarning : decisive;
    }

    /** El original confunde el {@code /SubFilter} no reconocido con una firma longeva sin comprobar. */
    private static SignValidity withUnrecognizedFormat(final boolean unrecognizedSubFilter,
            final SignValidity validity) {
        if (unrecognizedSubFilter && validity.getError() == VALIDITY_ERROR.SIGN_PROFILE_NOT_CHECKED) {
            return new SignValidity(SIGN_DETAIL_TYPE.KO, VALIDITY_ERROR.UNKOWN_SIGNATURE_FORMAT);
        }
        return validity;
    }

    private static String reasonOf(final SignValidity validity) {
        return validity.getError() == null ? null : validity.getError().name();
    }
}
