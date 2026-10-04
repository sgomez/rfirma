//! El veredicto de conjunto del validador del original sobre un documento (válido, inválido, sin firmas o pendiente de confirmar) y el texto de cada resultado que imprime `verify`; no dice de qué firmante es cada uno: eso es `PreviousSignaturesBridge`.
package es.gob.afirma.nativebridge;

import java.io.IOException;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Properties;

import org.spongycastle.cms.CMSException;
import org.spongycastle.cms.CMSSignedData;
import org.spongycastle.cms.SignerInformation;

import es.gob.afirma.core.RuntimeConfigNeededException;
import es.gob.afirma.core.RuntimeConfigNeededException.RequestType;
import es.gob.afirma.signvalidation.SignValider;
import es.gob.afirma.signvalidation.SignValidity;
import es.gob.afirma.signvalidation.SignValidity.SIGN_DETAIL_TYPE;
import es.gob.afirma.signvalidation.SignValidity.VALIDITY_ERROR;
import es.gob.afirma.signvalidation.ValidateBinarySignature;
import es.gob.afirma.signvalidation.ValidatePdfSignature;
import es.gob.afirma.signvalidation.ValidateXMLSignature;

/**
 * Las firmas que ya trae un documento, vistas por el validador del original.
 *
 * <p>El validador se instancia por formato y NO con {@code SignValiderFactory},
 * que resuelve la clase por {@code Class.forName} y obligaria a declararlas por
 * reflexion en la imagen nativa.
 *
 * <p>El modo <b>relajado</b> es lo que convierte en excepcion lo que fuera de el
 * seria un {@code KO} mudo: el original lanza una
 * {@link RuntimeConfigNeededException} que trae la clave de {@code extraParams}
 * con la que se repite sin volver a preguntar. Una peticion de contrasena
 * ({@link RequestType#PASSWORD}) no se puede atender aqui y sale como fallo.
 */
final class ValidationBridge {

    private ValidationBridge() { }

    static final String VALID = "valid";

    static final String INVALID = "invalid";

    static final String UNSIGNED = "unsigned";

    static final String CONFIRMATION_NEEDED = "confirmationNeeded";

    /** El veredicto, con la clave a fijar y el codigo de mensaje del original solo en el tercero. */
    record Verdict(String outcome, String reason, String param, String messageCode) { }

    /** Con {@code checkCertificates} mira tambien la caducidad del certificado firmante. */
    static Verdict validate(final byte[] document, final String format,
            final boolean checkCertificates) throws IOException {
        final SignValider valider = validerFor(format);
        valider.setRelaxed(true);
        try {
            return verdictOf(relaxedValidities(valider, document, checkCertificates));
        }
        catch (final RuntimeConfigNeededException e) {
            if (RequestType.CONFIRM != e.getRequestType()) {
                throw new UnsupportedOperationException(e.getRequestorText(), e);
            }
            return new Verdict(CONFIRMATION_NEEDED, null, e.getParam(), e.getRequestorText());
        }
    }

    /**
     * Lo que imprime de cada firma la orden {@code verify} del original: sin modo
     * relajado y con la caducidad del certificado firmante.
     */
    static List<String> results(final byte[] document, final String format) throws IOException {
        final List<String> results = new ArrayList<>();
        try {
            for (final SignValidity validity : validities(validerFor(format), document, true)) {
                results.add(plainText(validity.toString()));
            }
        }
        catch (final RuntimeConfigNeededException e) {
            throw new IllegalStateException("el validador sin modo relajado ha pedido confirmacion", e);
        }
        if (results.isEmpty()) {
            results.add(plainText(new SignValidity(SIGN_DETAIL_TYPE.KO,
                    VALIDITY_ERROR.UNKOWN_SIGNATURE_FORMAT).toString()));
        }
        return results;
    }

    /** El original escribe sus mensajes con las entidades HTML de sus dialogos. */
    private static final Map<String, String> HTML_ENTITIES = Map.of(
            "&aacute;", "á", "&eacute;", "é", "&iacute;", "í",
            "&oacute;", "ó", "&uacute;", "ú", "&ntilde;", "ñ");

    static String plainText(final String original) {
        String text = original;
        for (final Map.Entry<String, String> entity : HTML_ENTITIES.entrySet()) {
            text = text.replace(entity.getKey(), entity.getValue());
        }
        return text;
    }

    static SignValider validerFor(final String format) {
        return switch (format) {
            case "PAdES" -> new ValidatePdfSignature();
            case "CAdES", "CMS/PKCS#7" -> new ValidateBinarySignature();
            case "XAdES Detached", "XAdES Enveloping", "XAdES Enveloped", "FacturaE" ->
                    new ValidateXMLSignature();
            default -> throw new IllegalArgumentException(
                    "no hay validador del original para el formato " + format);
        };
    }

    /** Solo PAdES lee las {@code Properties}; los otros dos solo obedecen a la sobrecarga booleana. */
    private static List<SignValidity> validities(final SignValider valider,
            final byte[] document, final boolean checkCertificates)
            throws IOException, RuntimeConfigNeededException {
        return switch (valider) {
            case ValidatePdfSignature pdf -> pdf.validate(document, headless(checkCertificates));
            case ValidateBinarySignature binary -> integrityOnlyUnless(checkCertificates, binary, document);
            case ValidateXMLSignature xml -> xml.validate(document, checkCertificates);
            default -> throw new IllegalStateException(
                    "validador sin trato propio: " + valider.getClass().getName());
        };
    }

    /** Sin {@code checkCertificates} la caducidad no cuenta pero la integridad si (ADR-0044). */
    private static List<SignValidity> integrityOnlyUnless(final boolean checkCertificates,
            final ValidateBinarySignature binary, final byte[] document) throws IOException {
        final List<SignValidity> validities = binary.validate(document, true);
        if (checkCertificates || validities.stream().noneMatch(PreviousSignaturesBridge::isOutOfDate)) {
            return validities;
        }
        final List<SignValidity> integrity = new ArrayList<>(validities);
        integrity.removeIf(PreviousSignaturesBridge::isOutOfDate);
        integrity.addAll(integrityOfEachSigner(document));
        return integrity;
    }

    private static List<SignValidity> integrityOfEachSigner(final byte[] signature) {
        final CMSSignedData signed;
        try {
            signed = new CMSSignedData(signature);
        }
        catch (final CMSException e) {
            return List.of(new SignValidity(SIGN_DETAIL_TYPE.KO, VALIDITY_ERROR.UNKOWN_ERROR));
        }
        final boolean withContent = signed.getSignedContent() != null;
        final List<SignValidity> integrity = new ArrayList<>();
        for (final SignerInformation signer : signed.getSignerInfos().getSigners()) {
            integrity.add(PreviousSignaturesBridge.integrityOf(signer,
                    PreviousSignaturesBridge.certificateOf(signer, signed.getCertificates()),
                    withContent));
        }
        return integrity;
    }

    private static List<SignValidity> relaxedValidities(final SignValider valider,
            final byte[] document, final boolean checkCertificates)
            throws IOException, RuntimeConfigNeededException {
        if (!(valider instanceof final ValidatePdfSignature pdf)) {
            return validities(valider, document, checkCertificates);
        }
        final Properties options = headless(checkCertificates);
        LastSignedRevision.confirmNothingChangedAfterSigning(document, options);
        return pdf.validate(document, LastSignedRevision.withoutRepeatingThem(options));
    }

    private static Properties headless(final boolean checkCertificates) {
        final Properties options = new Properties();
        options.setProperty("headless", Boolean.TRUE.toString());
        options.setProperty("checkCertificates", Boolean.toString(checkCertificates));
        return options;
    }

    private static Verdict verdictOf(final List<SignValidity> validities) {
        for (final SignValidity validity : validities) {
            if (isFine(validity)) {
                continue;
            }
            if (withoutSignatures(validity.getError())) {
                return new Verdict(UNSIGNED, null, null, null);
            }
            return new Verdict(INVALID, nameOf(validity.getError()), null, null);
        }
        return new Verdict(VALID, null, null, null);
    }

    /** {@code UNKNOWN} es «no he podido comprobarlo», y solo vale si lo no comprobado es el perfil. */
    private static boolean isFine(final SignValidity validity) {
        return SIGN_DETAIL_TYPE.OK == validity.getValidity()
                || SIGN_DETAIL_TYPE.GENERATED == validity.getValidity()
                || VALIDITY_ERROR.SIGN_PROFILE_NOT_CHECKED == validity.getError();
    }

    /** El original lo cuenta como un {@code KO} que solo deja pasar una primera firma. */
    private static boolean withoutSignatures(final VALIDITY_ERROR error) {
        return VALIDITY_ERROR.NO_SIGN == error;
    }

    private static String nameOf(final VALIDITY_ERROR error) {
        return error == null ? VALIDITY_ERROR.UNKOWN_ERROR.name() : error.name();
    }
}
