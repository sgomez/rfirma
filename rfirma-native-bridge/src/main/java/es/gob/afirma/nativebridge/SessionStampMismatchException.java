package es.gob.afirma.nativebridge;

/** El sello recibido no es el de esta sesion trifasica (ADR-0016). */
public final class SessionStampMismatchException extends IllegalStateException {

    private static final long serialVersionUID = 1L;

    SessionStampMismatchException(final String message) {
        super(message);
    }
}
