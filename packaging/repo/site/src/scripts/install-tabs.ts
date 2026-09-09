/** Cada pestaña enseña su panel y deja seleccionada solo a sí misma. */
export function mountInstallTabs(tablist: HTMLElement, panels: readonly HTMLElement[]): void {
  const tabs = Array.from(tablist.querySelectorAll<HTMLElement>("[role='tab']"));
  const select = (chosen: string) => {
    for (const tab of tabs) {
      tab.setAttribute("aria-selected", String(tab.id === chosen));
    }
    for (const panel of panels) {
      panel.hidden = panel.getAttribute("aria-labelledby") !== chosen;
    }
  };
  for (const tab of tabs) {
    tab.addEventListener("click", () => select(tab.id));
  }
}
