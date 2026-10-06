import { renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { usePreferencesState } from "./App.usePreferencesState";
import { inMemoryPreferences } from "./preferences/preferences";
import { defaults } from "./preferences/testing/harness";
import { absentWindowTheme } from "./preferences/theme";
import { emptyRubricPicker } from "./signing/rubric";

describe("usePreferencesState", () => {
  it("reads the settings again once the setup wizard stops covering the window", async () => {
    const preferences = inMemoryPreferences({ ...defaults, consentCountdown: true });
    const rubrics = emptyRubricPicker();
    const windowTheme = absentWindowTheme();
    const { result, rerender } = renderHook(
      ({ covered }) => usePreferencesState(preferences, rubrics, covered, windowTheme),
      { initialProps: { covered: true } },
    );
    await waitFor(() => expect(result.current.settings?.consentCountdown).toBe(true));

    await preferences.save({ ...defaults, consentCountdown: false });
    rerender({ covered: false });

    await waitFor(() => expect(result.current.settings?.consentCountdown).toBe(false));
  });
});
