package es.gob.afirma.nativebridge;

import java.io.IOException;
import java.security.cert.X509Certificate;
import java.time.Instant;
import java.time.format.DateTimeFormatter;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import java.util.Map;
import java.util.Properties;

import javax.security.auth.x500.X500Principal;

import com.aowagie.text.pdf.AcroFields;
import com.aowagie.text.pdf.PdfArray;
import com.aowagie.text.pdf.PdfDictionary;
import com.aowagie.text.pdf.PdfName;
import com.aowagie.text.pdf.PdfPKCS7;
import com.aowagie.text.pdf.PdfReader;
import com.aowagie.text.pdf.PdfSignatureAppearance;

import es.gob.afirma.core.RuntimeConfigNeededException;
import es.gob.afirma.signers.pades.PdfUtil;
import es.gob.afirma.signvalidation.SignValidity;
import es.gob.afirma.signvalidation.SignValidity.SIGN_DETAIL_TYPE;
import es.gob.afirma.signvalidation.SignValidity.VALIDITY_ERROR;
import es.gob.afirma.signvalidation.SignatureFormatDetectorPadesCades;
import es.gob.afirma.signvalidation.ValidatePdfSignature;

/**
 * Las firmas que ya trae un PDF, recorridas como el escritorio de AutoFirma
 * 1.9.2 y validadas una a una con su validador, sin red y sin modo relajado.
 *
 * <p>El recorrido es el de {@code AOPDFSigner.getSignersStructure}: salta los
 * sellos de tiempo y las firmas que iText no llega a leer, y un PDF ilegible o
 * cifrado da un informe vacio en vez de un fallo.
 */
final class PreviousSignaturesBridge {

    private static final Map<String, String> READABLE_KEYWORDS = Map.of(
            "2.5.4.5", "SERIALNUMBER",
            "2.5.4.97", "organizationIdentifier");

    private static final PdfName ETSI_RFC3161 = new PdfName("ETSI.RFC3161");

    private static final PdfName DOC_TIMESTAMP = new PdfName("DocTimeStamp");

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

    /** Titular, emisor, numero de serie, fecha, estado y motivo del original de una firma previa. */
    record Signature(String subject, String issuer, String serialNumber, String signingTime,
            Status status, String reason) { }

    /** Las firmas en orden cronologico y si el documento cambio despues de la ultima. */
    record Report(List<Signature> signatures, boolean changedAfterLastSignature) { }

    static Report read(final byte[] pdf) {
        final PdfReader reader;
        final AcroFields fields;
        try {
            reader = PdfUtil.getPdfReader(pdf, headless(), true);
            fields = reader.getAcroFields();
        }
        catch (final Exception e) {
            return new Report(List.of(), false);
        }
        final String profile = SignatureFormatDetectorPadesCades.resolvePDFFormat(pdf);
        final int certifyingRevision = certifyingRevision(reader, fields);

        final List<Dated> dated = new ArrayList<>();
        for (final String name : fields.getSignatureNames()) {
            if (isTimestamp(fields, name)) {
                continue;
            }
            final PdfPKCS7 pkcs7;
            try {
                pkcs7 = fields.verifySignature(name);
            }
            catch (final RuntimeException e) {
                continue;
            }
            final X509Certificate signer = pkcs7.getSigningCertificate();
            if (signer == null) {
                continue;
            }
            final Instant signingTime =
                    pkcs7.getSignDate() == null ? null : pkcs7.getSignDate().toInstant();
            final List<SignValidity> validities = new ArrayList<>(validate(name, fields, profile));
            if (certifyingRevision > 0 && fields.getRevision(name) > certifyingRevision) {
                validities.add(new SignValidity(SIGN_DETAIL_TYPE.KO,
                        VALIDITY_ERROR.CERTIFIED_SIGN_REVISION));
            }
            final SignValidity validity = decisive(validities);
            dated.add(new Dated(signingTime, new Signature(
                    readable(signer.getSubjectX500Principal()),
                    readable(signer.getIssuerX500Principal()),
                    signer.getSerialNumber().toString(),
                    signingTime == null ? null : DateTimeFormatter.ISO_INSTANT.format(signingTime),
                    statusOf(validity),
                    reasonOf(validity))));
        }
        dated.sort(Comparator.comparing(Dated::signingTime,
                Comparator.nullsLast(Comparator.naturalOrder())));
        return new Report(dated.stream().map(Dated::signature).toList(), false);
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

    /** La revision que certifico el PDF «sin cambios permitidos», o 0. */
    private static int certifyingRevision(final PdfReader reader, final AcroFields fields) {
        if (reader.getCertificationLevel()
                != PdfSignatureAppearance.CERTIFIED_NO_CHANGES_ALLOWED) {
            return 0;
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
                return fields.getRevision(name);
            }
        }
        return 0;
    }

    private static boolean isTimestamp(final AcroFields fields, final String name) {
        final Object subFilter = fields.getSignatureDictionary(name).get(PdfName.SUBFILTER);
        return ETSI_RFC3161.equals(subFilter) || DOC_TIMESTAMP.equals(subFilter);
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

    /** Un {@code KO} pesa mas que un {@code UNKNOWN}, como en el validador del original. */
    private static SignValidity decisive(final List<SignValidity> validities) {
        SignValidity decisive = new SignValidity(SIGN_DETAIL_TYPE.OK, null);
        for (final SignValidity validity : validities) {
            if (SIGN_DETAIL_TYPE.KO == validity.getValidity()) {
                return validity;
            }
            if (SIGN_DETAIL_TYPE.UNKNOWN == validity.getValidity()) {
                decisive = validity;
            }
        }
        return decisive;
    }

    private static String reasonOf(final SignValidity validity) {
        return validity.getError() == null ? null : validity.getError().name();
    }
}
