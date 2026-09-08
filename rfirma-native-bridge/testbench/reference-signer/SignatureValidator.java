import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;

import es.gob.afirma.signvalidation.SignValider;
import es.gob.afirma.signvalidation.SignValiderFactory;
import es.gob.afirma.signvalidation.SignValidity;
import es.gob.afirma.signvalidation.SignValidity.SIGN_DETAIL_TYPE;
import es.gob.afirma.signvalidation.SignValidity.VALIDITY_ERROR;

/** Oraculo de la grada C: valida una firma con el validador del 1.9.2. */
public final class SignatureValidator {

    public static void main(String[] args) throws Exception {
        if (args.length != 1) {
            System.err.println("Uso: SignatureValidator <fichero>");
            System.exit(2);
        }
        byte[] data = Files.readAllBytes(Path.of(args[0]));
        List<SignValidity> resultados;
        try {
            SignValider validador = SignValiderFactory.getSignValider(data);
            resultados = validador.validate(data);
        } catch (IllegalArgumentException e) {
            System.out.println("INVALID no se reconoce como firma: " + e.getMessage());
            System.exit(1);
            return;
        }
        boolean valida = !resultados.isEmpty()
                && resultados.stream().allMatch(SignatureValidator::isTolerable);
        if (valida) {
            System.out.println("VALID" + unknownSuffix(resultados));
            System.exit(0);
        }
        String motivo = resultados.stream()
                .map(SignValidity::toString)
                .reduce((a, b) -> a + "; " + b)
                .orElse("sin detalle");
        System.out.println("INVALID " + motivo);
        System.exit(1);
    }

    // Una firma detached/explicita sin el fichero original, o con referencias
    // externas, produce UNKNOWN sin ningun OK: solo estos dos errores son
    // "no comprobado", no "no comprobable". Cualquier otro UNKNOWN (por
    // ejemplo, un catch generico) no debe pasar por valida.
    private static boolean isTolerable(SignValidity v) {
        return v.getValidity() != SIGN_DETAIL_TYPE.UNKNOWN
                || v.getError() == VALIDITY_ERROR.NO_DATA
                || v.getError() == VALIDITY_ERROR.SIGN_PROFILE_NOT_CHECKED;
    }

    private static String unknownSuffix(List<SignValidity> resultados) {
        String motivos = resultados.stream()
                .filter(v -> v.getValidity() == SIGN_DETAIL_TYPE.UNKNOWN)
                .map(v -> v.getError().toString())
                .reduce((a, b) -> a + ", " + b)
                .orElse(null);
        return motivos == null ? "" : " (con: " + motivos + ")";
    }
}
