import { useEffect, useState } from "react";
import { classify, type NamedFailure } from "./errors/classify";
import { acknowledgementFor, type Signing } from "./signing/useSigning";

/**
 * El acuse de recibo del documento que se acaba de firmar, y los dos caminos
 * hasta el fichero. Solo sigue en pie mientras el documento firmado siga
 * activo (ID-51): cambiar de fila lo cierra, en vez de esperar a que vuelva.
 */
export function useSignedSummary(
  signing: Signing,
  activeDocumentId: string | null,
  reopenDocument: () => void,
) {
  // Por qué no se pudo abrir el firmado o su carpeta. Vive aquí y no dentro del
  // resumen porque lo produce quien llama al portal, y el resumen solo lo
  // enseña; sin él, el único camino que el usuario tiene hasta el fichero
  // fallaría sin decir nada (ADR-0011).
  const [openFailure, setOpenFailure] = useState<NamedFailure | null>(null);

  // El acuse de recibo, solo si sigue siendo de lo que hay delante. El estado
  // «Firmado» guarda el asa del documento que se firmó; el recuento de páginas
  // que enseña sale del PDF abierto, así que el panel solo puede montarse
  // mientras los dos sean el mismo documento.
  const signedHere = acknowledgementFor(signing.state, activeDocumentId);
  const signedSomewhere = signing.state.kind === "signed";

  // Y cuando deja de serlo —se elige otro en la bandeja, se olvida el activo,
  // se vacía la lista— el estado se cierra, en vez de quedarse esperando a que
  // el documento firmado vuelva a estar delante.
  const signAnother = signing.signAnother;
  useEffect(() => {
    if (signedSomewhere && signedHere === null) signAnother();
    // Y el fallo de abrir se va con el resumen del que hablaba: es de un
    // documento concreto, como el propio acuse de recibo.
    if (signedHere === null) setOpenFailure(null);
  }, [signedSomewhere, signedHere, signAnother]);

  // Los dos caminos hasta el fichero. El fallo se recoge aquí y se enseña en el
  // resumen: un botón que no hace nada y no dice por qué deja al usuario sin
  // ninguna forma de llegar a lo que acaba de firmar (ID-79).
  const openSigned = (open: () => Promise<void>) => {
    setOpenFailure(null);
    open().catch((thrown: unknown) => setOpenFailure(classify(thrown)));
  };

  // «Volver a firmar»: se cierra el resumen y **se relee el original del
  // disco** (ID-80). Es abrir el documento otra vez, porque entre una firma y
  // la siguiente el usuario ha podido modificarlo fuera o haberse equivocado al
  // configurar la firma. Lo que decida el recuadro recordado —incluido el aviso
  // del ID-22 si ya no cabe— lo resuelve el camino de siempre, no uno nuevo.
  const signAgain = () => {
    setOpenFailure(null);
    signing.signAnother();
    reopenDocument();
  };

  return { signedHere, openFailure, openSigned, signAgain };
}
