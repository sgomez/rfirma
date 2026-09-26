import { useEffect, useState } from "react";
import type { Preferences, PreferencesStore } from "./preferences/preferences";
import { applyTheme } from "./preferences/theme";
import type { Destination, DestinationSource } from "./signing/destination";
import type { Rubric, RubricFailure, RubricPicker } from "./signing/rubric";
import type { SigningState } from "./signing/useSigning";

/**
 * Los ajustes y la rúbrica adoptada: los dos viven del mismo almacén de
 * preferencias y se leen al arrancar sin que nadie los vuelva a pedir.
 */
export function usePreferencesState(preferences: PreferencesStore, rubrics: RubricPicker) {
  const [settings, setSettings] = useState<Preferences | null>(null);
  const [rubric, setRubric] = useState<Rubric | null>(null);
  const [rubricFailure, setRubricFailure] = useState<RubricFailure | null>(null);

  useEffect(() => {
    let current = true;
    preferences.read().then((read) => {
      if (current) setSettings(read);
    });
    return () => {
      current = false;
    };
  }, [preferences]);

  // La rúbrica adoptada en una sesión anterior sigue en el almacén aunque se
  // cierre la aplicación (ID-33): sin esta lectura al arrancar, «Tu rúbrica»
  // aparecía siempre apagada aunque el JPEG estuviera ahí.
  useEffect(() => {
    let current = true;
    rubrics.stored().then((found) => {
      if (current && found !== null) setRubric(found);
    });
    return () => {
      current = false;
    };
  }, [rubrics]);

  // El tema elegido, puesto en el documento. Es lo único de los ajustes que no
  // se pinta dentro de la ventana sino **sobre** ella: los tokens de color
  // cuelgan de `<html>`, así que quien lo aplica tiene que salir del árbol de
  // React. Mientras los ajustes se leen no se toca nada, y manda el sistema.
  useEffect(() => {
    if (settings) applyTheme(settings.theme);
  }, [settings]);

  /**
   * Un ajuste cambia **en cuanto se toca**, y solo se queda si el disco lo
   * acepta.
   *
   * Si guardar falla —el fichero de configuración no se deja escribir— la
   * pantalla vuelve a lo que había: una ventana que enseña un ajuste que el
   * disco no tiene estaría mintiendo sobre la sesión siguiente. Repuesto el
   * valor, **el rechazo sigue su camino**: quien lo recoge es Preferencias, que
   * es quien sabe en qué sección se pulsó y por tanto dónde va el aviso
   * (ID-70).
   */
  const changeSettings = async (next: Preferences) => {
    const before = settings;
    setSettings(next);
    try {
      await preferences.save(next);
    } catch (thrown) {
      setSettings(before);
      throw thrown;
    }
  };

  /**
   * La carpeta de destino se elige con el **selector de directorio** del
   * sistema, que abre Rust: la ventana no manda ninguna ruta —no la conoce— y
   * lo que recibe de vuelta es el nombre que enseña (ID-65).
   *
   * Cerrar el selector sin elegir deja la carpeta que hubiera. Si guardar
   * falla, el rechazo sigue su camino hasta Preferencias, que es quien sabe
   * dónde va el aviso (ID-70).
   */
  const chooseDestination = async () => {
    const chosen = await preferences.chooseFolder();
    if (chosen !== null && settings !== null) {
      setSettings({ ...settings, destination: chosen });
    }
  };

  // La rúbrica se comprueba y se normaliza **al elegirla**, con el panel
  // abierto, y nunca al firmar (ADR-0012): el fallo se cuenta aquí.
  const chooseRubric = async () => {
    const choice = await rubrics.choose();
    if (choice === null) return;
    if ("failure" in choice) {
      setRubricFailure(choice.failure);
      return;
    }
    setRubricFailure(null);
    setRubric(choice.rubric);
  };

  return {
    settings,
    changeSettings,
    chooseDestination,
    rubric,
    rubricFailure,
    chooseRubric,
  };
}

/**
 * Dónde caerá el firmado, tal y como lo cuenta el backend, y el destino de una
 * sola firma que lo puede sustituir. Es estado y no un cálculo del pie porque
 * el nombre lo compone Rust —con el sufijo y el homónimo ya resueltos— y
 * `writable` sale de comprobar la carpeta de verdad (ID-67): la ventana lo
 * enseña, no lo deduce.
 *
 * El destino de una sola firma, elegido con «Cambiar», vale solo para el
 * documento activo: cambiar de pestaña o terminar de firmar lo olvida
 * (ADR-0011).
 */
export function useDestination(
  destinations: DestinationSource,
  activeId: string | null,
  chosenFolder: string | null,
  signingStateKind: SigningState["kind"],
) {
  const [singleDestinationId, setSingleDestinationId] = useState<string | null>(null);
  const [destination, setDestination] = useState<Destination | null>(null);

  // biome-ignore lint/correctness/useExhaustiveDependencies: `activeId` dispara el efecto, no lo alimenta.
  useEffect(() => {
    setSingleDestinationId(null);
  }, [activeId]);

  useEffect(() => {
    if (signingStateKind === "signed") setSingleDestinationId(null);
  }, [signingStateKind]);

  useEffect(() => {
    // Sin documento delante no hay destino que enseñar, y sin ajustes leídos
    // tampoco: la carpeta que se va a consultar es la que ellos dicen.
    if (activeId === null || chosenFolder === null) {
      setDestination(null);
      return;
    }
    let current = true;
    destinations
      .previewFor(activeId, singleDestinationId)
      .then((found) => {
        if (current) setDestination(found);
      })
      .catch(() => {
        // Un destino que no se puede consultar no apaga el panel: se queda sin
        // pie hasta la siguiente vuelta, que es menos que perder el documento.
        if (current) setDestination(null);
      });
    return () => {
      current = false;
    };
  }, [destinations, activeId, chosenFolder, singleDestinationId]);

  const chooseSingleDestination = async () => {
    if (activeId === null) return;
    const chosen = await destinations.chooseSingle(activeId);
    if (chosen !== null) setSingleDestinationId(chosen.id);
  };

  return { destination, singleDestinationId, chooseSingleDestination };
}
