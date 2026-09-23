import { useEffect, useMemo, useRef, useState } from "react";
import {
  hasMenuAttention,
  type SignalRow,
  type StatusPort,
  withLocalCaCertificateMeasured,
} from "./status/status";
import type { NewVersion, VersionCheck } from "./updates/newVersion";

/**
 * Lo que se comprueba **una vez, al arrancar**: si hay versión nueva
 * publicada (ID-181) y las filas del panel de estado, para el triángulo del
 * menú (ID-347/ID-353).
 */
export function useStartupNotices(status: StatusPort, versions: VersionCheck) {
  // El aviso de versión: lo que contestó el puerto y si ya se descartó. Se
  // descarta **para esta sesión** y no se anota en disco: quien decide cada
  // cuánto se vuelve a preguntar es el backend (una vez cada 24 h), y una
  // segunda memoria aquí sería una regla más que no manda nadie.
  const [newVersion, setNewVersion] = useState<NewVersion | null>(null);
  const [versionDismissed, setVersionDismissed] = useState(false);
  // Las filas del panel de estado, para el triángulo del menú (ID-353): se
  // miden aquí al arrancar, y `StatusView` reenvía cada remedición suya
  // propia —al abrirse, tras una acción, con «Volver a comprobar»— sin que
  // esta ventana dispare ninguna por su cuenta.
  const [statusRows, setStatusRows] = useState<SignalRow[]>([]);
  const hasAttention = useMemo(() => hasMenuAttention(statusRows), [statusRows]);
  // El puerto **por omisión** de `status` es un objeto nuevo en cada pintada
  // (`= memoryStatus()`), así que el efecto de más abajo lo lee de una `ref` y
  // no de la lista de dependencias: si `status` fuera su dependencia, cada
  // remedición cambiaría de identidad y volvería a disparar la lectura del
  // arranque sin parar.
  const statusAtStartup = useRef(status);

  // Si hay versión nueva se pregunta **una vez, al arrancar**, y lo que se
  // haga con la respuesta es enseñar una franja: nada de esto interrumpe el
  // recorrido (ID-181). Un `null` —o directamente que no haya red— deja la
  // ventana exactamente como estaba, sin aviso, sin error y sin reintento.
  useEffect(() => {
    let current = true;
    versions
      .latest()
      .then((published) => {
        if (current) setNewVersion(published);
      })
      .catch(() => {
        // La comprobación de versión no tiene voz para quejarse: es un extra,
        // y un fallo suyo no es asunto de quien está firmando.
      });
    return () => {
      current = false;
    };
  }, [versions]);

  // El triángulo del menú se mide **una vez, al arrancar**, para no obligar a
  // abrir el panel antes de saber si hay algo que arreglar (ID-347).
  useEffect(() => {
    let current = true;
    const port = statusAtStartup.current;
    port
      .readStatus()
      .then((rows) => withLocalCaCertificateMeasured(rows, port))
      .then((rows) => {
        if (current) setStatusRows(rows);
      });
    return () => {
      current = false;
    };
  }, []);

  return {
    newVersion,
    versionDismissed,
    setVersionDismissed,
    statusRows,
    setStatusRows,
    hasAttention,
  };
}
