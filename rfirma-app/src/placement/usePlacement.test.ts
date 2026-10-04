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

describe("sealing a page", () => {
  const TRACED: UserSpaceRect = { x0: 30, y0: 40, x1: 230, y1: 120 };

  function renderPlaced(placement: Placement) {
    return renderPlacement(undefined, aDocument(placement));
  }

  it("replaces the page under «one page»", () => {
    const { result } = renderPlaced({ rect: SAVED_RECT, pages: { only: [1] } });

    act(() => result.current.sealPage(TRACED, 3));

    expect(result.current.placement).toEqual({ rect: TRACED, pages: { only: [3] } });
  });

  it("adds the page under «these pages»", () => {
    const { result } = renderPlaced({ rect: SAVED_RECT, pages: { only: [1, 4] } });

    act(() => result.current.sealPage(TRACED, 2));

    expect(result.current.placement).toEqual({ rect: TRACED, pages: { only: [1, 2, 4] } });
  });

  it("keeps every page under «all pages»", () => {
    const { result } = renderPlaced({ rect: SAVED_RECT, pages: "all" });

    act(() => result.current.sealPage(TRACED, 2));

    expect(result.current.placement).toEqual({ rect: TRACED, pages: "all" });
  });

  it("starts the set from the page when nothing is placed", () => {
    const { result } = renderPlacement();

    act(() => result.current.sealPage(TRACED, 2));

    expect(result.current.placement).toEqual({ rect: TRACED, pages: { only: [2] } });
  });

  it("starts the set from the page under «these pages» with no range yet", () => {
    const { result } = renderPlacement();
    act(() => result.current.changePageMode("these"));

    act(() => result.current.sealPage(TRACED, 4));

    expect(result.current.pageMode).toBe("these");
    expect(result.current.placement).toEqual({ rect: TRACED, pages: { only: [4] } });
  });

  it("tells the change notice what would be signed", () => {
    const onChange = vi.fn();
    const { result } = renderPlacement(onChange);

    act(() => result.current.sealPage(TRACED, 2));

    expect(onChange).toHaveBeenLastCalledWith({ rect: TRACED, pages: { only: [2] } });
  });
});

describe("moving the box", () => {
  it("moves it on every page of the set and leaves the set alone", () => {
    const moved: UserSpaceRect = { x0: 60, y0: 40, x1: 260, y1: 120 };
    const { result } = renderPlacement(
      undefined,
      aDocument({ rect: SAVED_RECT, pages: { only: [1, 3] } }),
    );

    act(() => result.current.moveBox(moved));

    expect(result.current.placement).toEqual({ rect: moved, pages: { only: [1, 3] } });
  });
});

describe("unsealing a page", () => {
  it("takes the page out of the set", () => {
    const { result } = renderPlacement(
      undefined,
      aDocument({ rect: SAVED_RECT, pages: { only: [1, 3] } }),
    );

    act(() => result.current.unsealPage(3));

    expect(result.current.placement).toEqual({ rect: SAVED_RECT, pages: { only: [1] } });
  });

  it("names the rest one by one when the whole document was sealed", () => {
    const { result } = renderPlacement(undefined, aDocument({ rect: SAVED_RECT, pages: "all" }));
    act(() => result.current.changePageMode("these"));

    act(() => result.current.unsealPage(2));

    expect(result.current.placement).toEqual({ rect: SAVED_RECT, pages: { only: [1, 3, 4, 5] } });
  });

  it("takes the whole placement away with the last page of the set", () => {
    const onChange = vi.fn();
    const { result } = renderPlacement(
      onChange,
      aDocument({ rect: SAVED_RECT, pages: { only: [2] } }),
    );

    act(() => result.current.unsealPage(2));

    expect(result.current.placement).toBeNull();
    expect(onChange).toHaveBeenLastCalledWith(null);
  });

  it("does nothing, and tells nobody, when nothing is placed", () => {
    const onChange = vi.fn();
    const { result } = renderPlacement(onChange);

    act(() => result.current.unsealPage(1));

    expect(result.current.placement).toBeNull();
    expect(onChange).not.toHaveBeenCalled();
  });
});

describe("the page button", () => {
  function renderPlaced(placement: Placement | null, onChange?: (p: Placement | null) => void) {
    return renderPlacement(onChange, aDocument(placement));
  }

  it("puts the box at the standard position of the page in view when nothing is placed", async () => {
    const { result } = renderPlaced(null);
    act(() => result.current.viewPage(3));

    act(() => result.current.sealViewedPage());

    await waitFor(() =>
      expect(result.current.placement).toEqual({ rect: standardOn(3), pages: { only: [3] } }),
    );
  });

  it("moves the page under «one page» and keeps the box where it was", async () => {
    const { result } = renderPlaced({ rect: SAVED_RECT, pages: { only: [1] } });
    act(() => result.current.viewPage(4));

    act(() => result.current.sealViewedPage());

    await waitFor(() =>
      expect(result.current.placement).toEqual({ rect: SAVED_RECT, pages: { only: [4] } }),
    );
  });

  it("adds the page in view under «these pages»", async () => {
    const { result } = renderPlaced({ rect: SAVED_RECT, pages: { only: [1, 4] } });
    act(() => result.current.viewPage(2));

    act(() => result.current.sealViewedPage());

    await waitFor(() =>
      expect(result.current.placement).toEqual({ rect: SAVED_RECT, pages: { only: [1, 2, 4] } }),
    );
  });

  it("starts «these pages» with no range yet from the page in view, at the standard position", async () => {
    const { result } = renderPlaced(null);
    act(() => result.current.changePageMode("these"));
    act(() => result.current.viewPage(5));

    act(() => result.current.sealViewedPage());

    await waitFor(() =>
      expect(result.current.placement).toEqual({ rect: standardOn(5), pages: { only: [5] } }),
    );
  });

  it("takes the page in view out of «these pages»", () => {
    const { result } = renderPlaced({ rect: SAVED_RECT, pages: { only: [1, 3] } });
    act(() => result.current.viewPage(3));

    act(() => result.current.unsealViewedPage());

    expect(result.current.placement).toEqual({ rect: SAVED_RECT, pages: { only: [1] } });
  });

  it("names the rest one by one when the whole document was sealed", () => {
    const { result } = renderPlaced({ rect: SAVED_RECT, pages: "all" });
    act(() => result.current.changePageMode("these"));
    act(() => result.current.viewPage(2));

    act(() => result.current.unsealViewedPage());

    expect(result.current.placement).toEqual({ rect: SAVED_RECT, pages: { only: [1, 3, 4, 5] } });
  });

  it("takes the whole placement away with the last page of the set", () => {
    const onChange = vi.fn();
    const { result } = renderPlaced({ rect: SAVED_RECT, pages: { only: [2] } }, onChange);
    act(() => result.current.viewPage(2));

    act(() => result.current.unsealViewedPage());

    expect(result.current.placement).toBeNull();
    expect(onChange).toHaveBeenLastCalledWith(null);
  });

  it("unseals nothing, and tells nobody, when nothing is placed", () => {
    const onChange = vi.fn();
    const { result } = renderPlaced(null, onChange);

    act(() => result.current.unsealViewedPage());

    expect(result.current.placement).toBeNull();
    expect(onChange).not.toHaveBeenCalled();
  });

  it("places nothing while there is no document to measure", async () => {
    const onChange = vi.fn();
    const { result } = renderHook(() =>
      usePlacement({ document: aDocument(), standardRectOn: null, onChange }),
    );

    await act(async () => result.current.sealViewedPage());

    expect(result.current.placement).toBeNull();
    expect(onChange).not.toHaveBeenCalled();
  });
});
