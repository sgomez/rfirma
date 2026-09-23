/** «2,4 MB». El tamaño en la unidad que el usuario reconoce, no en bytes. */
export function formatSize(bytes: number, locale: string): string {
  const megabytes = bytes / 1_000_000;
  const format = new Intl.NumberFormat(locale, { maximumFractionDigits: 1 });
  if (megabytes >= 1) return `${format.format(megabytes)} MB`;
  return `${format.format(bytes / 1000)} kB`;
}
