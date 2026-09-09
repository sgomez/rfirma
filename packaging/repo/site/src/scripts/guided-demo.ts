/** El paso cuyo centro cae más cerca del centro de la ventana. */
export function closestStep(centers: readonly number[], viewportCenter: number): number {
  let closest = 0;
  let distance = Number.POSITIVE_INFINITY;
  centers.forEach((center, index) => {
    const candidate = Math.abs(center - viewportCenter);
    if (candidate < distance) {
      distance = candidate;
      closest = index;
    }
  });
  return closest;
}

function show(root: HTMLElement, step: number): void {
  root.dataset.step = String(step);
  for (const element of root.querySelectorAll<HTMLElement>("[data-steps]")) {
    element.hidden = !(element.dataset.steps ?? "").split(" ").includes(String(step));
  }
  root.querySelectorAll<HTMLElement>("[data-demo-dot]").forEach((dot, index) => {
    dot.setAttribute("aria-selected", String(index === step));
  });
}

/** La demo avanza con el scroll y con sus botones, y ninguno de los dos mueve la página. */
export function mountGuidedDemo(root: HTMLElement): void {
  const steps = Array.from(root.querySelectorAll<HTMLElement>(".demo-step"));
  if (steps.length === 0) {
    return;
  }
  show(root, 0);

  root.querySelectorAll<HTMLElement>("[data-demo-dot]").forEach((dot, index) => {
    dot.addEventListener("click", () => show(root, index));
  });

  let pending = 0;
  const synchronise = () => {
    pending = 0;
    const centers = steps.map((step) => {
      const box = step.getBoundingClientRect();
      return box.top + box.height / 2;
    });
    const step = closestStep(centers, window.innerHeight / 2);
    if (String(step) !== root.dataset.step) {
      show(root, step);
    }
  };
  const schedule = () => {
    if (!pending) {
      pending = requestAnimationFrame(synchronise);
    }
  };

  document.addEventListener("scroll", schedule, { capture: true, passive: true });
  window.addEventListener("resize", schedule);
  synchronise();
}
