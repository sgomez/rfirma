import { describe, expect, it } from "vitest";
import { closestStep } from "./guided-demo";

describe("closestStep", () => {
  it("picks the step whose centre is nearest the middle of the viewport", () => {
    expect(closestStep([-400, 100, 900], 400)).toBe(1);
  });

  it("keeps the first step when two are equally near", () => {
    expect(closestStep([300, 500], 400)).toBe(0);
  });

  it("falls back to the first step with nothing to compare", () => {
    expect(closestStep([], 400)).toBe(0);
  });
});
