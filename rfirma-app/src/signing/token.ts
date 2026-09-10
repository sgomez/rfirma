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
 * El diálogo modal nativo del sistema operativo gestiona la solicitud
 * interactiva del secreto y los reintentos; los fallos definitivos se cuentan
 * al pie del panel o en el desenlace del trámite.
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
   * Cuántos intentos quedan antes de que la tarjeta se bloquee. Cruza desde
   * Rust y llega siempre a `null`: PKCS#11 no cuenta los intentos, así que no
   * es un hueco por rellenar sino algo estructural (ID-191, docs/design/
   * dialogo-pin.md). No se enseña en ninguna parte.
   */
  attemptsLeft: number | null;
}
