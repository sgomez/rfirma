import { describe, expect, it } from "vitest";
import { DEFAULT_VISIBLE_SIGNATURE, rubricGapFor, type VisibleSignature } from "./visibleSignature";

const complete: VisibleSignature = { ...DEFAULT_VISIBLE_SIGNATURE, enabled: true };

describe("el hueco de la rúbrica", () => {
  it("puts the gap beside the text when the rubric is on but not loaded", () => {
    expect(rubricGapFor({ ...complete, withRubric: true }, false)).toBe("beside");
  });

  it("fills the box when the model is rubric only and nothing is loaded", () => {
    const rubricOnly: VisibleSignature = {
      ...complete,
      withRubric: true,
      content: { model: "rubricOnly" },
    };
    expect(rubricGapFor(rubricOnly, false)).toBe("fill");
  });

  it("draws no gap once the rubric is loaded", () => {
    expect(rubricGapFor({ ...complete, withRubric: true }, true)).toBeNull();
  });

  it("draws no gap when the rubric is off", () => {
    expect(rubricGapFor(complete, false)).toBeNull();
  });
});
