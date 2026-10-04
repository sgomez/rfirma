//! Las dos comprobaciones de lo añadido a un PDF tras firmarlo que el validador del original hace antes de validar cada firma, con la última firma buscada por revisión.
package es.gob.afirma.nativebridge;

import java.io.IOException;
import java.io.InputStream;
import java.util.Map;
import java.util.Properties;

import com.aowagie.text.pdf.AcroFields;
import com.aowagie.text.pdf.PdfReader;

import es.gob.afirma.core.RuntimeConfigNeededException;
import es.gob.afirma.signers.pades.PdfUtil;
import es.gob.afirma.signers.pades.common.PdfExtraParams;
import es.gob.afirma.signers.pades.common.PdfFormModifiedException;
import es.gob.afirma.signers.pades.common.SuspectedPSAException;
import es.gob.afirma.signvalidation.DataAnalizerUtil;

/**
 * El original compara con la primera firma de {@code getSignatureNames}, que sale de un
 * {@code HashMap}; aqui, con la de revision mas alta (ADR-0023).
 */
final class LastSignedRevision {

    private LastSignedRevision() { }

    private static final String PAGES_TO_CHECK = "10";

    /** Pide confirmacion como la pediria el original; sin firmas o ilegible, lo decide el original. */
    static void confirmNothingChangedAfterSigning(final byte[] pdf, final Properties options)
            throws IOException, RuntimeConfigNeededException {
        final PdfReader reader;
        try {
            reader = PdfUtil.getPdfReader(pdf, options, true);
        }
        catch (final Exception unreadable) {
            return;
        }
        final AcroFields fields = reader.getAcroFields();
        final String last = PreviousSignaturesBridge.latestRevisionName(fields);
        if (last == null || fields.getTotalRevisions() <= 1) {
            return;
        }
        final Map<String, String> changedFields = DataAnalizerUtil.checkPDFForm(reader);
        if (changedFields != null && !changedFields.isEmpty()) {
            throw new PdfFormModifiedException("formulario cambiado tras la primera firma");
        }
        if (fields.getRevision(last) >= fields.getTotalRevisions()) {
            return;
        }
        try (InputStream lastSigned = fields.extractRevision(last)) {
            if (DataAnalizerUtil.checkPdfShadowAttack(pdf, lastSigned, PAGES_TO_CHECK) != null) {
                throw new SuspectedPSAException("pagina cambiada tras la ultima firma");
            }
        }
    }

    /** Las opciones con las que el original ya no repite ninguna de las dos comprobaciones. */
    static Properties withoutRepeatingThem(final Properties options) {
        final Properties without = new Properties();
        without.putAll(options);
        without.setProperty(PdfExtraParams.ALLOW_SIGN_MODIFIED_FORM, Boolean.TRUE.toString());
        return without;
    }
}
