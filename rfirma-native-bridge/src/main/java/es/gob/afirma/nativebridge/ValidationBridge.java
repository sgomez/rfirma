package es.gob.afirma.nativebridge;

import java.io.IOException;
import java.util.List;
import java.util.Properties;

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

    static final String CONFIRMATION_NEEDED = "confirmationNeeded";

    /** El veredicto, con la clave a fijar y el codigo de mensaje del original solo en el tercero. */
    record Verdict(String outcome, String reason, String param, String messageCode) { }

    static Verdict validate(final byte[] document, final String format) throws IOException {
        final SignValider valider = validerFor(format);
        valider.setRelaxed(true);
        try {
            return verdictOf(withoutCheckingCertificates(valider, document));
        }
        catch (final RuntimeConfigNeededException e) {
            if (RequestType.CONFIRM != e.getRequestType()) {
                throw new UnsupportedOperationException(e.getRequestorText(), e);
            }
            return new Verdict(CONFIRMATION_NEEDED, null, e.getParam(), e.getRequestorText());
        }
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
    private static List<SignValidity> withoutCheckingCertificates(
            final SignValider valider, final byte[] document)
            throws IOException, RuntimeConfigNeededException {
        return switch (valider) {
            case ValidatePdfSignature pdf -> pdf.validate(document, headlessWithoutCertificates());
            case ValidateBinarySignature binary -> binary.validate(document, false);
            case ValidateXMLSignature xml -> xml.validate(document, false);
            default -> throw new IllegalStateException(
                    "validador sin trato propio: " + valider.getClass().getName());
        };
    }

    private static Properties headlessWithoutCertificates() {
        final Properties options = new Properties();
        options.setProperty("headless", Boolean.TRUE.toString());
        options.setProperty("checkCertificates", Boolean.FALSE.toString());
        return options;
    }

    private static Verdict verdictOf(final List<SignValidity> validities) {
        for (final SignValidity validity : validities) {
            if (isFine(validity) || withoutSignatures(validity.getError())) {
                continue;
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

    /** Un documento sin firmas es valido, y el original lo cuenta como un {@code KO}. */
    private static boolean withoutSignatures(final VALIDITY_ERROR error) {
        return VALIDITY_ERROR.NO_SIGN == error;
    }

    private static String nameOf(final VALIDITY_ERROR error) {
        return error == null ? VALIDITY_ERROR.UNKOWN_ERROR.name() : error.name();
    }
}
