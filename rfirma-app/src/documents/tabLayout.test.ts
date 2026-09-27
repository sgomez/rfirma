import { describe, expect, it } from "vitest";
import { layOutTabs, MORE_WIDTH, TAB_GAP, TAB_MIN_WIDTH } from "./tabLayout";

const tabsNamed = (...ids: string[]) => ids.map((id) => ({ id }));
const ids = (tabs: readonly { id: string }[]) => tabs.map((tab) => tab.id);
const widthFor = (count: number) => count * TAB_MIN_WIDTH + (count - 1) * TAB_GAP;

const six = tabsNamed("a", "b", "c", "d", "e", "f");
const threeAndMore = widthFor(3) + MORE_WIDTH;

describe("layOutTabs", () => {
  it("shows every tab and hides none when they all fit", () => {
    const layout = layOutTabs(1000, tabsNamed("a", "b"), "a");

    expect(ids(layout.visible)).toEqual(["a", "b"]);
    expect(layout.hidden).toEqual([]);
  });

  it("shows every tab when they fit exactly, without reserving room for the overflow button", () => {
    const layout = layOutTabs(widthFor(4), tabsNamed("a", "b", "c", "d"), "a");

    expect(ids(layout.visible)).toEqual(["a", "b", "c", "d"]);
    expect(layout.hidden).toEqual([]);
  });

  it("reserves room for the overflow button once a single pixel is missing", () => {
    const layout = layOutTabs(widthFor(4) - 1, tabsNamed("a", "b", "c", "d"), "a");

    expect(ids(layout.visible)).toEqual(["a", "b", "c"]);
    expect(ids(layout.hidden)).toEqual(["d"]);
  });

  it("shows as many tabs as fit beside the overflow button, and hides the rest in order", () => {
    const layout = layOutTabs(threeAndMore, six, "a");

    expect(ids(layout.visible)).toEqual(["a", "b", "c"]);
    expect(ids(layout.hidden)).toEqual(["d", "e", "f"]);
  });

  it("leaves the tabs in place when the active one is first", () => {
    expect(ids(layOutTabs(threeAndMore, six, "a").visible)).toEqual(["a", "b", "c"]);
  });

  it("leaves the tabs in place when the active one already fits in the middle", () => {
    expect(ids(layOutTabs(threeAndMore, six, "b").visible)).toEqual(["a", "b", "c"]);
  });

  it("puts a hidden active tab in the last visible slot", () => {
    const layout = layOutTabs(threeAndMore, six, "e");

    expect(ids(layout.visible)).toEqual(["a", "b", "e"]);
    expect(ids(layout.hidden)).toEqual(["c", "d", "f"]);
  });

  it("puts the last tab in the last visible slot when it is the active one", () => {
    const layout = layOutTabs(threeAndMore, six, "f");

    expect(ids(layout.visible)).toEqual(["a", "b", "f"]);
    expect(ids(layout.hidden)).toEqual(["c", "d", "e"]);
  });

  it("still shows the active tab when the width is less than one tab", () => {
    const layout = layOutTabs(TAB_MIN_WIDTH - 1, six, "d");

    expect(ids(layout.visible)).toEqual(["d"]);
    expect(ids(layout.hidden)).toEqual(["a", "b", "c", "e", "f"]);
  });

  it("shows a lone tab and hides nothing when the width is less than one tab", () => {
    const layout = layOutTabs(TAB_MIN_WIDTH - 1, tabsNamed("a"), "a");

    expect(ids(layout.visible)).toEqual(["a"]);
    expect(layout.hidden).toEqual([]);
  });
});
