import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { DesignRoot } from "./DesignRoot";

describe("DesignRoot", () => {
  it("paints each instance with its own theme", () => {
    const { container } = render(
      <>
        <DesignRoot theme="light">
          <p>claro</p>
        </DesignRoot>
        <DesignRoot theme="dark">
          <p>oscuro</p>
        </DesignRoot>
      </>,
    );

    const themes = [...container.querySelectorAll(".rf-root")].map((root) =>
      root.getAttribute("data-theme"),
    );
    expect(themes).toEqual(["light", "dark"]);
  });

  it("leaves the body with the theme of the first root still mounted", () => {
    const first = render(<DesignRoot theme="light">uno</DesignRoot>);
    const second = render(<DesignRoot theme="dark">dos</DesignRoot>);
    expect(document.body.getAttribute("data-theme")).toBe("light");

    first.unmount();
    expect(document.body.getAttribute("data-theme")).toBe("dark");

    second.unmount();
    expect(document.body.classList.contains("rf-root")).toBe(false);
    expect(document.body.hasAttribute("data-theme")).toBe(false);
  });
});
