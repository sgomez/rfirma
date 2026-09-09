const FEEDBACK_MS = 2000;

/** Copia el bloque de órdenes que acompaña al botón y avisa de que lo ha hecho. */
export function mountCopyButton(button: HTMLElement, code: HTMLElement): void {
  button.addEventListener("click", () => {
    if (!navigator.clipboard) {
      return;
    }
    navigator.clipboard
      .writeText(code.textContent ?? "")
      .then(() => {
        button.classList.add("is-copied");
        setTimeout(() => button.classList.remove("is-copied"), FEEDBACK_MS);
      })
      .catch(() => {});
  });
}
