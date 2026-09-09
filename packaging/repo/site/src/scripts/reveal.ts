/** Cada elemento marcado aparece la primera vez que entra en pantalla. */
export function mountReveal(nodes: readonly HTMLElement[]): void {
  if (typeof IntersectionObserver === "undefined") {
    for (const node of nodes) {
      node.classList.add("is-in");
    }
    return;
  }
  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) {
          entry.target.classList.add("is-in");
          observer.unobserve(entry.target);
        }
      }
    },
    { rootMargin: "0px 0px -12% 0px", threshold: 0.1 },
  );
  for (const node of nodes) {
    observer.observe(node);
  }
}
