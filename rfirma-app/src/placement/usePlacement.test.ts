import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { PageMode, PageSet, Placement, UserSpaceRect } from "./pageSets";
import { type PlacedDocument, type StandardRectOn, usePlacement } from "./usePlacement";

const PAGE_COUNT = 5;

function standardOn(page: number): UserSpaceRect {
  return { x0: page, y0: page, x1: page + 100, y1: page + 40 };
}

const fakeStandardRectOn: StandardRectOn = async (page) => standardOn(page);

const SAVED_RECT: UserSpaceRect = { x0: 10, y0: 20, x1: 210, y1: 100 };

function aDocument(placement: Placement | null = null): PlacedDocument {
  return { placement, pageCount: PAGE_COUNT };
}

function renderPlacement(
  onChange?: (placement: Placement | null) => void,
  first: PlacedDocument | null = aDocument(),
) {
  return renderHook(
    ({ document }: { document: PlacedDocument | null }) =>
      usePlacement({ document, standardRectOn: fakeStandardRectOn, onChange }),
    { initialProps: { document: first } },
  );
}

describe("usePlacement", () => {
  it("each page mode keeps its own set and returning brings it back", async () => {
    const { result } = renderPlacement();
    const choose = async (pages: PageSet) => {
      act(() => result.current.choosePages(pages));
      await waitFor(() => expect(result.current.placement?.pages).toEqual(pages));
    };
    const change = (mode: PageMode) => act(() => result.current.changePageMode(mode));

    await choose({ only: [2] });
    change("these");
    await choose({ only: [3, 4] });
    change("single");
    await waitFor(() => expect(result.current.placement?.pages).toEqual({ only: [2] }));
    change("these");
    await waitFor(() => expect(result.current.placement?.pages).toEqual({ only: [3, 4] }));
  });

  it("a page mode used for the first time starts with the previous mode's set", async () => {
    const { result } = renderPlacement();
    act(() => result.current.choosePages({ only: [3] }));
    await waitFor(() => expect(result.current.placement).not.toBeNull());

    act(() => result.current.changePageMode("these"));

    await waitFor(() => expect(result.current.placement?.pages).toEqual({ only: [3] }));
    expect(result.current.pageMode).toBe("these");
  });

  it("choosing a page mode without a box puts it in the standard position", async () => {
    const { result } = renderPlacement();
    act(() => result.current.viewPage(2));

    act(() => result.current.changePageMode("all"));

    await waitFor(() =>
      expect(result.current.placement).toEqual({ rect: standardOn(1), pages: "all" }),
    );
  });

  it("choosing a page set without a box puts it in the standard position of its first page", async () => {
    const { result } = renderPlacement();

    act(() => result.current.choosePages({ only: [4, 5] }));

    await waitFor(() =>
      expect(result.current.placement).toEqual({ rect: standardOn(4), pages: { only: [4] } }),
    );
  });

  it("tells the change notice what would be signed", async () => {
    const onChange = vi.fn();
    const { result } = renderPlacement(onChange);

    act(() => result.current.choosePages({ only: [2] }));

    await waitFor(() =>
      expect(onChange).toHaveBeenLastCalledWith({ rect: standardOn(2), pages: { only: [2] } }),
    );
  });

  it("works without a change notice", async () => {
    const { result } = renderPlacement();

    act(() => result.current.placeOnViewedPage());

    await waitFor(() =>
      expect(result.current.placement).toEqual({ rect: standardOn(1), pages: { only: [1] } }),
    );
  });

  it("remembers the page in view and places on it", async () => {
    const { result } = renderPlacement();

    act(() => result.current.viewPage(4));
    act(() => result.current.placeOnViewedPage());

    expect(result.current.viewedPage).toBe(4);
    await waitFor(() =>
      expect(result.current.placement).toEqual({ rect: standardOn(4), pages: { only: [4] } }),
    );
  });

  it("brings back the saved placement of another document, on its first page", () => {
    const { result, rerender } = renderPlacement();
    act(() => result.current.viewPage(2));

    rerender({ document: aDocument({ rect: SAVED_RECT, pages: { only: [3, 4] } }) });

    expect(result.current.placement).toEqual({ rect: SAVED_RECT, pages: { only: [3, 4] } });
    expect(result.current.pageMode).toBe("these");
    expect(result.current.viewedPage).toBe(3);
  });

  it("starts empty on the first page for a document with nothing saved", async () => {
    const { result, rerender } = renderPlacement();
    act(() => result.current.choosePages({ only: [2] }));
    await waitFor(() => expect(result.current.placement).not.toBeNull());
    act(() => result.current.viewPage(2));

    rerender({ document: aDocument() });

    expect(result.current.placement).toBeNull();
    expect(result.current.pageMode).toBe("single");
    expect(result.current.viewedPage).toBe(1);
  });

  it("forgets the other modes when the same document is reopened", async () => {
    const { result, rerender } = renderPlacement();
    act(() => result.current.choosePages({ only: [2] }));
    await waitFor(() => expect(result.current.placement?.pages).toEqual({ only: [2] }));
    act(() => result.current.changePageMode("these"));
    act(() => result.current.choosePages({ only: [3, 4] }));
    await waitFor(() => expect(result.current.placement?.pages).toEqual({ only: [3, 4] }));

    rerender({ document: aDocument(result.current.placement) });
    act(() => result.current.changePageMode("single"));

    await waitFor(() => expect(result.current.placement?.pages).toEqual({ only: [3] }));
  });

  it("keeps everything while the same document stays in front", async () => {
    const document = aDocument();
    const { result, rerender } = renderPlacement(undefined, document);
    act(() => result.current.choosePages({ only: [2] }));
    await waitFor(() => expect(result.current.placement).not.toBeNull());
    act(() => result.current.viewPage(5));

    rerender({ document });

    expect(result.current.placement?.pages).toEqual({ only: [2] });
    expect(result.current.viewedPage).toBe(5);
  });

  it("has no placement with no document in front, and does not tell the change notice", async () => {
    const onChange = vi.fn();
    const { result, rerender } = renderPlacement(
      onChange,
      aDocument({ rect: SAVED_RECT, pages: { only: [2] } }),
    );

    rerender({ document: null });

    expect(result.current.placement).toBeNull();
    expect(result.current.viewedPage).toBe(1);
    expect(onChange).not.toHaveBeenCalled();
  });
});
