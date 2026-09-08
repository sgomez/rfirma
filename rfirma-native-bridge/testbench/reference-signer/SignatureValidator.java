import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;

import es.gob.afirma.signvalidation.SignValider;
import es.gob.afirma.signvalidation.SignValiderFactory;
import es.gob.afirma.signvalidation.SignValidity;
import es.gob.afirma.signvalidation.SignValidity.SIGN_DETAIL_TYPE;

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
        // Una firma detached/explicita sin el fichero original produce ademas
        // un SignValidity UNKNOWN ("no se puede comprobar... por no tener los
        // datos firmados") junto al OK de la firma criptografica: UNKNOWN no
        // tumba el veredicto, solo KO lo hace.
        boolean valida = !resultados.isEmpty()
                && resultados.stream().noneMatch(v -> v.getValidity() == SIGN_DETAIL_TYPE.KO);
        if (valida) {
            System.out.println("VALID");
            System.exit(0);
        }
        String motivo = resultados.stream()
                .map(SignValidity::toString)
                .reduce((a, b) -> a + "; " + b)
                .orElse("sin detalle");
        System.out.println("INVALID " + motivo);
        System.exit(1);
    }
}
