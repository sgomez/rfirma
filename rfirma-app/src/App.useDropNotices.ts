import { useEffect, useRef, useState } from "react";
import type { DocumentInHand } from "./documents/document";
import type { DocumentDrops, Drop } from "./documents/drops";
import type { DocumentFailure } from "./viewer/source";

/**
 * Un aviso del arrastre, atado al documento del que habla.
 *
 * `about` es el identificador del documento que tenía que estar delante para
 * que el aviso siga significando algo: el que se acaba de abrir cuando se
 * soltaron varios, o el que ya estaba cuando lo soltado no se pudo abrir.
 */
export interface DropNotice {
  about: string | null;
  failure: DocumentFailure;
}

/**
 * El arrastre. Desemboca en el mismo sitio que el diálogo —`accept` es la
 * mitad de `open` que no habla con el portal—, y lo que añade es contar lo
 * que solo pasa al soltar: que no fuera un PDF, que no se dejara leer o que
 * fueran varios.
 *
 * La suscripción se monta una vez y sobrevive a los cambios de documento: si
 * dependiera de `documents`, cada apertura cancelaría el oyente y volvería a
 * suscribirse, y un arrastre que cayera en medio se perdería.
 */
export function useDropNotices(
  drops: DocumentDrops,
  acceptDocument: (document: DocumentInHand) => Promise<void>,
  enterDocument: (document: DocumentInHand) => Promise<void>,
  activeId: string | null,
) {
  const [dropNotice, setDropNotice] = useState<DropNotice | null>(null);
  const activeIdRef = useRef(activeId);
  activeIdRef.current = activeId;

  // La entrega de la invocación **no puede vivir en el ciclo de vida de este
  // efecto**: `drops.pending()` es una lectura que consume, así que descartar
  // su respuesta porque el efecto se rehizo pierde el documento para siempre
  // —la segunda llamada ya devuelve `null`—. Y se rehace de verdad: en
  // desarrollo `<StrictMode>` monta, limpia y vuelve a montar, y en producción
  // la identidad de `acceptDocument` cambia cuando se lee «Recordar mi
  // actividad» y viene apagada. Por eso «ya se preguntó» es una marca que
  // sobrevive a los remontajes, y la respuesta se entrega por el manejador
  // vigente en ese momento en vez de por el que la pidió.
  const invocationAsked = useRef(false);
  const arrivedRef = useRef<(drop: Drop) => void>(() => {});
  useEffect(() => {
    const arrived = (drop: Drop) => {
      if (drop.document === null) {
        // No se ha abierto nada, así que el aviso habla de lo que ya hubiera
        // delante y se va con ello.
        setDropNotice(drop.failure && { about: activeIdRef.current, failure: drop.failure });
        return;
      }
      setDropNotice(
        drop.discarded > 0
          ? {
              about: drop.document.id,
              failure: {
                situation: "droppedSomeDiscarded",
                detail:
                  drop.discarded === 1
                    ? "se ha descartado 1 fichero"
                    : `se han descartado ${drop.discarded} ficheros`,
              },
            }
          : null,
      );
      // Uno detrás de otro, nunca en paralelo: `store.record` reescribe el
      // estado entero sin cerrojo y dos llamadas solapadas pierden una.
      const document = drop.document;
      void (async () => {
        for (const entering of drop.alsoEntering) {
          await enterDocument(entering);
        }
        await acceptDocument(document);
      })();
    };
    arrivedRef.current = arrived;
    const stop = drops.subscribe(arrived);
    // Y por aquí mismo entra el documento con el que se invocó a la aplicación
    // desde fuera (ID-157): desemboca en la ventana completa, en el mismo
    // estado en que la deja arrastrar un PDF (ID-159), así que no tiene camino
    // propio. Se pregunta **después** de suscribirse, no antes: una segunda
    // invocación que llegara en medio se perdería.
    if (!invocationAsked.current) {
      invocationAsked.current = true;
      void drops.pending().then((invoked) => {
        if (invoked !== null) arrivedRef.current(invoked);
      });
    }
    return stop;
  }, [drops, acceptDocument, enterDocument]);

  return { dropNotice, setDropNotice };
}
