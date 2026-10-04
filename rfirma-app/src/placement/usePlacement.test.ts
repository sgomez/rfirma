import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { PageMode, PageSet, Placement, UserSpaceRect } from "./pageSets";
import { type StandardRectOn, usePlacement } from "./usePlacement";

const PAGE_COUNT = 5;

function standardOn(page: number): UserSpaceRect {
  return { x0: page, y0: page, x1: page + 100, y1: page + 40 };
}

const fakeStandardRectOn: StandardRectOn = async (page) => standardOn(page);

function renderPlacement(viewedPage = 1, onChange?: (placement: Placement | null) => void) {
  return renderHook(() =>
    usePlacement({
      pageCount: PAGE_COUNT,
      standardRectOn: fakeStandardRectOn,
      viewedPage,
      onChange,
    }),
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
    const { result } = renderPlacement(2);

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
    const { result } = renderPlacement(1, onChange);

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
});
