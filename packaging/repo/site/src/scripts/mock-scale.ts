/** La maqueta se dibuja a tamaño fijo y se escala al ancho que le quede. */
export function mountMockScale(frame: HTMLElement, stage: HTMLElement, width: number, height: number): void {
  if (typeof ResizeObserver === "undefined") {
    return;
  }
  const fit = () => {
    const available = frame.clientWidth;
    if (!available) {
      return;
    }
    const scale = Math.min(1, available / width);
    stage.style.transform = `scale(${scale})`;
    frame.style.height = `${Math.round(height * scale)}px`;
  };
  new ResizeObserver(fit).observe(frame);
  fit();
}
