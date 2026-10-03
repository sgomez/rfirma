//! El fallo con el que no sale una firma cuya sede pidió sello de tiempo y no se pudo sellar (ADR-0030).
package es.gob.afirma.nativebridge;

public final class TimestampFailedException extends Exception {

    private static final long serialVersionUID = 1L;

    TimestampFailedException(final String message, final Throwable cause) {
        super(message, cause);
    }
}
