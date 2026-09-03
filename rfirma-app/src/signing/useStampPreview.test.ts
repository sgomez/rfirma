import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { PdfDocument } from "../viewer/pdf";
import type { SigningOrder } from "./flow";
import type { ComposedStamp, StampComposer, StampRequest } from "./stampPreview";
import { useStampPreview } from "./useStampPreview";

/**
 * **Grada A** (`vitest`, carril rápido). Sub-issue #177, TD-32.
 *
 * Los cuatro estados de la vista previa se prueban **contra un doble del
 * puerto**, no contra el puente: son estados de la interfaz y no dependen de
 * que haya un token puesto. Y con ellos el camino de fallo del ID-111, que es
 * el que dice que esto no es una puerta.
 */

const stamped = { pageCount: 3, getPage: vi.fn() } as unknown as PdfDocument;
const restamped = { pageCount: 3, getPage: vi.fn() } as unknown as PdfDocument;

/** Una orden cualquiera: lo que se prueba es el ciclo, no su contenido. */
function anOrder(rect: [number, number, number, number] = [50, 60, 250, 140]): SigningOrder {
  return {
    document: "un-documento",
    certificate: "0123456789abcdef",
    placement: {
      page: 1,
      pages: { only: [1] },
      pageCount: 3,
      mediaBox: [0, 0, 595, 842],
      rotation: 0,
      rect,
    },
    fields: { signerName: true, idNumber: true, signedAt: true, reason: false },
    reason: "",
    signedAt: "3 de septiembre de 2026, 12:00",
    rubric: null,
    language: "es",
  };
}

const ready = (rect?: [number, number, number, number]): StampRequest => ({
  kind: "ready",
  order: anOrder(rect),
});

/** Un compositor que contesta lo que se le diga, y cuenta cuántas veces le llaman. */
function composerOf(...answers: ComposedStamp[]): StampComposer & { calls: number } {
  const composer = {
    calls: 0,
    compose: async () => {
      const answer = answers[Math.min(composer.calls, answers.length - 1)];
      composer.calls += 1;
      return answer as ComposedStamp;
    },
  };
  return composer;
}

const ok: ComposedStamp = { ok: true, pdf: stamped };
const again: ComposedStamp = { ok: true, pdf: restamped };
const broken: ComposedStamp = {
  ok: false,
  failure: { situation: "documentUnreadable", detail: "el documento tiene contraseña" },
};

describe("los cuatro estados de la vista previa", () => {
  it("shows nothing and asks for nothing without a certificate", async () => {
    const composer = composerOf(ok);

    const { result } = renderHook(() =>
      useStampPreview({
        composer,
        request: { kind: "noCertificate" },
        gesturing: false,
        onDemand: false,
      }),
    );

    expect(result.current.state).toEqual({ kind: "noCertificate" });
    expect(result.current.pdf).toBeNull();
    expect(composer.calls).toBe(0);
  });

  it("shows nothing and asks for nothing while the box is not placed", async () => {
    const composer = composerOf(ok);

    const { result } = renderHook(() =>
      useStampPreview({
        composer,
        request: { kind: "unplaced" },
        gesturing: false,
        onDemand: false,
      }),
    );

    expect(result.current.state).toEqual({ kind: "unplaced" });
    expect(composer.calls).toBe(0);
  });

  it("composes the stamp once the box is placed", async () => {
    const composer = composerOf(ok);

    const { result } = renderHook(() =>
      useStampPreview({ composer, request: ready(), gesturing: false, onDemand: false }),
    );

    await waitFor(() => expect(result.current.state).toEqual({ kind: "composed" }));
    expect(result.current.pdf).toBe(stamped);
    expect(composer.calls).toBe(1);
  });

  it("freezes the previous view during the gesture, and does not recompose", async () => {
    const composer = composerOf(ok, again);
    const { result, rerender } = renderHook(
      ({ gesturing }: { gesturing: boolean }) =>
        useStampPreview({ composer, request: ready(), gesturing, onDemand: false }),
      { initialProps: { gesturing: false } },
    );
    await waitFor(() => expect(result.current.state).toEqual({ kind: "composed" }));

    rerender({ gesturing: true });

    expect(result.current.state).toEqual({ kind: "frozen" });
    // La vista anterior sigue ahí: es la que sirve para medir el bulto.
    expect(result.current.pdf).toBe(stamped);
    expect(composer.calls).toBe(1);
  });

  it("recomposes on its own when the box is dropped somewhere else", async () => {
    const composer = composerOf(ok, again);
    const { result, rerender } = renderHook(
      ({ rect }: { rect: [number, number, number, number] }) =>
        useStampPreview({
          composer,
          request: ready(rect),
          gesturing: false,
          onDemand: false,
        }),
      { initialProps: { rect: [50, 60, 250, 140] as [number, number, number, number] } },
    );
    await waitFor(() => expect(result.current.pdf).toBe(stamped));

    rerender({ rect: [90, 60, 290, 140] });

    await waitFor(() => expect(result.current.pdf).toBe(restamped));
    expect(composer.calls).toBe(2);
  });

  it("does not compose twice for the same order", async () => {
    const composer = composerOf(ok);
    const { result, rerender } = renderHook(() =>
      useStampPreview({ composer, request: ready(), gesturing: false, onDemand: false }),
    );
    await waitFor(() => expect(result.current.state).toEqual({ kind: "composed" }));

    rerender();
    rerender();

    expect(composer.calls).toBe(1);
  });
});

describe("en un documento grande el recálculo se pide a mano (ID-109)", () => {
  it("waits for «Ver cómo queda» instead of composing on its own", async () => {
    const composer = composerOf(ok);

    const { result } = renderHook(() =>
      useStampPreview({ composer, request: ready(), gesturing: false, onDemand: true }),
    );

    expect(result.current.state).toEqual({ kind: "onDemand" });
    expect(composer.calls).toBe(0);
  });

  it("composes when asked", async () => {
    const composer = composerOf(ok);
    const { result } = renderHook(() =>
      useStampPreview({ composer, request: ready(), gesturing: false, onDemand: true }),
    );

    act(() => result.current.compose());

    await waitFor(() => expect(result.current.state).toEqual({ kind: "composed" }));
    expect(composer.calls).toBe(1);
  });
});

describe("si no se puede componer, se dice y se firma igual (ID-111)", () => {
  it("names the failure and keeps the box empty", async () => {
    const composer = composerOf(broken);

    const { result } = renderHook(() =>
      useStampPreview({ composer, request: ready(), gesturing: false, onDemand: false }),
    );

    await waitFor(() => expect(result.current.state.kind).toBe("failed"));
    expect(result.current.pdf).toBeNull();
  });

  it("does not try again on its own, and does try when asked", async () => {
    const composer = composerOf(broken, ok);
    const { result, rerender } = renderHook(() =>
      useStampPreview({ composer, request: ready(), gesturing: false, onDemand: false }),
    );
    await waitFor(() => expect(result.current.state.kind).toBe("failed"));

    rerender();
    expect(composer.calls).toBe(1);

    act(() => result.current.compose());

    await waitFor(() => expect(result.current.state).toEqual({ kind: "composed" }));
    expect(composer.calls).toBe(2);
  });
});
