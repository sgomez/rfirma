//! La fecha del recuadro y la hora de «Firmado a las…», con el formato del idioma de la ventana. Sin React.

/**
 * La fecha del recuadro, en el idioma de la ventana.
 *
 * El **formato** es lo único de la firma visible que decide el frontal, y es a
 * propósito: quien sabe el huso y las convenciones de fecha del sistema es el
 * navegador, no Rust, y meter una biblioteca de husos en el backend para
 * repetir lo que `Intl` ya sabe sería duplicar el problema. Las **etiquetas**
 * del recuadro siguen siendo de `signing::layer2_text`: aquí no se
 * escribe «Fecha», solo lo que va detrás.
 */
export function formatSignedAt(instant: Date, locale: string): string {
  return new Intl.DateTimeFormat(locale, {
    dateStyle: "short",
    timeStyle: "medium",
  }).format(instant);
}

/**
 * La hora sola, para «Firmado a las 11:04» (docs/design/panel-de-firma.md § El
 * resumen, tras firmar): el mismo instante que `formatSignedAt`, sin la fecha
 * ni los segundos que ahí hacen falta para el recuadro.
 */
export function formatSignedTime(instant: Date, locale: string): string {
  return new Intl.DateTimeFormat(locale, { timeStyle: "short" }).format(instant);
}
