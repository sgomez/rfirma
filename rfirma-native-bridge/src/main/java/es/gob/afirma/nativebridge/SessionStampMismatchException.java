//! El fallo con el que una postfirma rechaza un sello que no es el de su prefirma (ADR-0016).
package es.gob.afirma.nativebridge;

public final class SessionStampMismatchException extends IllegalStateException {

    private static final long serialVersionUID = 1L;

    SessionStampMismatchException(final String message) {
        super(message);
    }
}
