package es.gob.afirma.nativebridge;

/** La sede pidio sello de tiempo y no se pudo sellar: la firma no sale sin el (ADR-0030). */
public final class TimestampFailedException extends Exception {

    private static final long serialVersionUID = 1L;

    TimestampFailedException(final String message, final Throwable cause) {
        super(message, cause);
    }
}
