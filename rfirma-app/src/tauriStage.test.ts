import { describe, expect, it } from "vitest";
import { stage } from "./tauriStage";

describe("stage", () => {
  it("passes a site situation through untouched", async () => {
    const result = await stage(() =>
      Promise.reject({ situation: "saveDestinationUnwritable", detail: "x" }),
    );

    expect(result).toMatchObject({
      ok: false,
      failure: { situation: "saveDestinationUnwritable" },
    });
  });
});
