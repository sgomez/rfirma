import { describe, expect, it } from "vitest";
import { createI18n } from "../i18n/i18n";
import { classify } from "./classify";
import { errorText } from "./errorMessage";

const { t } = createI18n("es");

describe("classify", () => {
  it("keeps the situation of a rejection Rust already classified", () => {
    expect(
      classify({ situation: "incorrectPin", detail: "CKR_PIN_INCORRECT", attemptsLeft: 2 }),
    ).toEqual({
      situation: "incorrectPin",
      detail: "CKR_PIN_INCORRECT",
      attemptsLeft: 2,
    });
  });

  it("keeps a situation of the site catalogue as it came", () => {
    expect(classify({ situation: "saveDestinationUnwritable", detail: "x" }).situation).toBe(
      "saveDestinationUnwritable",
    );
  });

  it("gives a situation the window does not know a retry message", () => {
    expect(errorText("aSituationFromTheFuture", t).title).toBeTruthy();
  });

  it.each(["promptFailed", "secretOnTheReaderKeypad", "userCancelled"])(
    "gives %s a message",
    (situation) => {
      const failure = classify({ situation, detail: "x" });

      expect(failure.situation).toBe(situation);
      expect(errorText(failure.situation, t).title).toBeTruthy();
    },
  );
});
