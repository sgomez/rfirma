import { describe, expect, it, vi } from "vitest";
import type { RenderTask } from "./pdf";
import { createRenderQueue } from "./renderQueue";

/**
 * **Grada A** (`vitest`, carril rápido).
 *
 * La prueba que pide el sub-issue #58: un cambio de zoom o de página **cancela**
 * la pintada anterior. Sin esto dos `RenderTask` escriben sobre el mismo lienzo
 * y lo que queda es una mezcla de las dos escalas.
 */

/** Una pintada que no acaba hasta que la prueba lo dice. */
function fakeTask(): RenderTask & { finish: () => void; cancelled: boolean } {
  let settle: () => void = () => {};
  let fail: (error: unknown) => void = () => {};
  const task = {
    promise: new Promise<void>((resolve, reject) => {
      settle = resolve;
      fail = reject;
    }),
    cancelled: false,
    cancel() {
      task.cancelled = true;
      // Lo que hace `pdf.js` al cancelar: rechazar con esta excepción.
      const cancellation = new Error("Rendering cancelled");
      cancellation.name = "RenderingCancelledException";
      fail(cancellation);
    },
    finish: () => settle(),
  };
  return task;
}

describe("la cola de pintadas", () => {
  it("cancels the render in flight before starting the next one", async () => {
    const queue = createRenderQueue();
    const first = fakeTask();
    const second = fakeTask();

    const pending = queue.run(() => first);
    expect(first.cancelled).toBe(false);

    const next = queue.run(() => second);
    expect(first.cancelled).toBe(true);
    expect(second.cancelled).toBe(false);

    second.finish();
    await expect(pending).resolves.toBeUndefined();
    await expect(next).resolves.toBeUndefined();
  });

  it("cancels before asking for the new task, so two never share the canvas", async () => {
    const queue = createRenderQueue();
    const first = fakeTask();
    const order: string[] = [];

    void queue.run(() => first);
    const started = vi.fn(() => {
      order.push("empieza la segunda");
      return fakeTask();
    });
    // La primera tiene que estar cancelada **antes** de que nadie pida la
    // segunda: si el orden se invierte hay un instante con dos vivas.
    first.promise.catch(() => order.push("cancelada la primera"));
    void queue.run(started);

    expect(started).toHaveBeenCalled();
    expect(order).toEqual(["empieza la segunda"]);
    expect(first.cancelled).toBe(true);
  });

  it("swallows the cancellation, which is not a failure", async () => {
    const queue = createRenderQueue();
    const task = fakeTask();

    const pending = queue.run(() => task);
    queue.cancel();

    await expect(pending).resolves.toBeUndefined();
  });

  it("lets a real rendering failure through", async () => {
    const queue = createRenderQueue();
    const broken: RenderTask = {
      promise: Promise.reject(new Error("el PDF está roto")),
      cancel: () => {},
    };

    await expect(queue.run(() => broken)).rejects.toThrow("el PDF está roto");
  });

  it("forgets a task that finished, so cancelling later is harmless", async () => {
    const queue = createRenderQueue();
    const task = fakeTask();

    const pending = queue.run(() => task);
    task.finish();
    await pending;

    queue.cancel();
    expect(task.cancelled).toBe(false);
  });
});
