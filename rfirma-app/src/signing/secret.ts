/**
 * Cómo hay que pedirle el secreto al almacén, tal y como sale de la prefirma
 * (ID-189, ID-190).
 *
 * Es el espejo exacto de `SecretView` en Rust, con las mismas tres variantes:
 * la ventana lee `kind` y decide entre firmar directo y abrir el diálogo del
 * secreto. La tercera —`typedOnTheReaderKeypad`— no llega hoy a cruzar, porque
 * la prefirma la rechaza antes de que el ciclo se abra; está aquí porque el
 * tipo es de tres variantes y partirlo en dos vocabularios costaría más que la
 * rama que sobra.
 */
export type StoreSecret =
  /**
   * El almacén no exige sesión: se firma directo, sin diálogo. Es lo único que
   * esta variante decide — la ventana no llama a `sign` con ningún PIN
   * inventado, manda la cadena vacía, igual que con un `.p12` sin contraseña.
   */
  | { kind: "notNeeded" }
  /**
   * El secreto se teclea en pantalla: un módulo PKCS#11 con PIN, o un perfil
   * NSS con contraseña maestra.
   */
  | { kind: "typedOnScreen"; attemptsLeft: number | null }
  /** El teclado del propio lector, que rfirma no sabe pedir. */
  | { kind: "typedOnTheReaderKeypad" };
