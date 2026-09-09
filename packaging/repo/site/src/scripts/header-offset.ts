/** La cabecera se ancla justo debajo del aviso, mida este lo que mida. */
export function mountHeaderOffset(notice: HTMLElement, header: HTMLElement): void {
  if (typeof ResizeObserver === "undefined") {
    return;
  }
  const fit = () => {
    header.style.top = `${notice.offsetHeight}px`;
  };
  new ResizeObserver(fit).observe(notice);
  fit();
}
