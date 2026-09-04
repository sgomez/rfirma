/**
 * Lo que el token puede contestar cuando algo va mal.
 *
 * **La clasificación es de Rust** (`pkcs11::error`, ID-29): el `CKR_*` de
 * `cryptoki` se convierte allí en una situación nuestra, y aquí solo llega ya
 * clasificada, con el código crudo al lado. En este directorio no hay —ni debe
 * haber— una tabla de `CKR_*`: sería una segunda clasificación de lo mismo, que
 * es exactamente lo que el ADR-0009 evita.
 */

/**
 * Las siete situaciones de `pkcs11::error::Situation`, con los mismos nombres.
 *
 * Solo `incorrectPin` vive en el diálogo del secreto y no en un aviso de
 * error: se reintenta sin salir. `pinLocked` ya no está ahí (docs/design/
 * dialogo-pin.md): la v0.4 retira tarjetas y DNIe del alcance, y con ellos la
 * única situación de PKCS#11 que llegaba a bloquearse dentro de esa
 * conversación; se cuenta como cualquier otro fallo, al pie del panel.
 */
export type TokenSituation =
  | "incorrectPin"
  | "pinLocked"
  | "tokenAbsent"
  | "expiredSession"
  | "moduleNotFound"
  | "certificateNotFound"
  | "unknown";

/** Un fallo del token: la situación traducible y el detalle crudo. */
export interface TokenFailure {
  situation: TokenSituation;
  /**
   * El texto original tal cual: `CKR_PIN_INCORRECT (C_Login)`. **No se traduce
   * ni se recorta**: está para pegarlo en un informe de fallo.
   */
  detail: string;
  /**
   * Cuántos intentos quedan antes de que la tarjeta se bloquee, si el módulo lo
   * dice; `null` cuando no lo dice. Se enseña siempre que se sepa: bloquear una
   * tarjeta por no avisar es un daño real y no siempre reversible.
   */
  attemptsLeft: number | null;
}

/**
 * Si el fallo se resuelve **dentro** del diálogo del secreto.
 *
 * Solo un PIN o una contraseña incorrectos: se reintentan ahí mismo, sin
 * reiniciar el recorrido, porque son la respuesta a lo que se acaba de
 * teclear. Todo lo demás —incluida una tarjeta bloqueada— sale del diálogo y
 * se cuenta en el pie del panel, que es donde vive el estado de «error de
 * firma» (docs/design/dialogo-pin.md).
 */
export function belongsToPinDialog(failure: TokenFailure): boolean {
  return failure.situation === "incorrectPin";
}
