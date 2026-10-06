import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import { Tab, Tabs } from "./Tabs";

function Harness() {
  const [active, setActive] = useState("a");
  return (
    <Tabs aria-label="Documentos">
      {["a", "b", "c"].map((id) => (
        <Tab key={id} selected={id === active} onClick={() => setActive(id)}>
          Pestaña {id}
        </Tab>
      ))}
    </Tabs>
  );
}

const tab = (name: string) => screen.getByRole("tab", { name });

describe("Tabs", () => {
  it("renders a tablist whose tabs say which one is selected", () => {
    render(<Harness />);

    expect(screen.getByRole("tablist", { name: "Documentos" })).toBeInTheDocument();
    expect(tab("Pestaña a")).toHaveAttribute("aria-selected", "true");
    expect(tab("Pestaña b")).toHaveAttribute("aria-selected", "false");
  });

  it("puts only the selected tab in the tab order", () => {
    render(<Harness />);

    expect(tab("Pestaña a")).toHaveAttribute("tabindex", "0");
    expect(tab("Pestaña b")).toHaveAttribute("tabindex", "-1");
  });

  it("moves the focus with the arrows, wrapping, without selecting", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    tab("Pestaña a").focus();

    await user.keyboard("{ArrowRight}");
    expect(tab("Pestaña b")).toHaveFocus();
    await user.keyboard("{ArrowLeft}{ArrowLeft}");
    expect(tab("Pestaña c")).toHaveFocus();
    await user.keyboard("{Home}");
    expect(tab("Pestaña a")).toHaveFocus();
    await user.keyboard("{End}");
    expect(tab("Pestaña c")).toHaveFocus();
    expect(tab("Pestaña a")).toHaveAttribute("aria-selected", "true");
  });

  it("lets the caller choose the active tab by activating it", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    tab("Pestaña a").focus();

    await user.keyboard("{ArrowRight}{Enter}");

    expect(tab("Pestaña b")).toHaveAttribute("aria-selected", "true");
    expect(tab("Pestaña b")).toHaveAttribute("tabindex", "0");
  });

  it("skips disabled tabs", async () => {
    const user = userEvent.setup();
    render(
      <Tabs aria-label="x">
        <Tab selected>uno</Tab>
        <Tab selected={false} disabled>
          dos
        </Tab>
        <Tab selected={false}>tres</Tab>
      </Tabs>,
    );
    tab("uno").focus();

    await user.keyboard("{ArrowRight}");

    expect(tab("tres")).toHaveFocus();
  });
});
