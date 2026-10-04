import { composeStories } from "@storybook/react-vite";
import { screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import * as stories from "./DocumentViewer.stories";

const {
  Empty,
  StampUnplaced,
  StampNoCertificate,
  StampComposed,
  StampFrozen,
  StampOnDemand,
  StampComposing,
  StampFailed,
  WithSignatureBox,
  RubricGapBesideText,
  RubricGapFillingBox,
  WithDocument,
} = composeStories(stories);

const boxName = "Recuadro de la firma visible";

describe("the empty viewer", () => {
  it("draws an icon and the supporting line in the drop zone, with no floating bar", () => {
    renderWithCatalog(<Empty />);

    const dropZone = screen.getByRole("button", { name: /Arrastra un PDF/ });

    expect(dropZone.querySelector("svg")).not.toBeNull();
    expect(dropZone).toHaveTextContent("No sale de tu ordenador");
    expect(screen.queryByRole("button", { name: "Acercar" })).not.toBeInTheDocument();
  });
});

describe("the signature box", () => {
  it("hangs one grip on each of the four corners", async () => {
    const { container } = renderWithCatalog(<WithSignatureBox />);
    await screen.findByRole("application", { name: boxName });

    expect(
      [...container.querySelectorAll(".viewer__grip")].map((g) => g.getAttribute("data-corner")),
    ).toEqual(["top-left", "top-right", "bottom-left", "bottom-right"]);
  });

  it("is not drawn when the document has no placement", async () => {
    renderWithCatalog(<WithDocument />);
    await screen.findByLabelText("Número de página");

    expect(screen.queryByRole("application", { name: boxName })).not.toBeInTheDocument();
  });
});

describe("the rubric gap inside the box", () => {
  const cases = [
    { Story: RubricGapBesideText, modifier: "beside" },
    { Story: RubricGapFillingBox, modifier: "fill" },
  ];

  for (const { Story, modifier } of cases) {
    it(`draws the gap as ${modifier}`, async () => {
      renderWithCatalog(<Story />);
      const box = await screen.findByRole("application", { name: boxName });

      expect(within(box).getByTitle("Sin rúbrica cargada")).toHaveClass(
        `viewer__rubric-gap--${modifier}`,
      );
    });
  }

  it("leaves the box empty when there is no certificate to compose with", async () => {
    renderWithCatalog(<StampNoCertificate />);
    const box = await screen.findByRole("application", { name: boxName });

    expect(within(box).queryByTitle("Sin rúbrica cargada")).not.toBeInTheDocument();
  });
});

describe("the stamp pill floating over the button bar", () => {
  const silent = [
    { name: "unplaced", Story: StampUnplaced },
    { name: "no certificate", Story: StampNoCertificate },
    { name: "composed", Story: StampComposed },
    { name: "frozen", Story: StampFrozen },
    { name: "no stamp at all", Story: WithSignatureBox },
  ];

  for (const { name, Story } of silent) {
    it(`mounts no pill when the stamp is ${name}`, async () => {
      renderWithCatalog(<Story />);
      await screen.findByLabelText("Número de página");
      await waitFor(() => expect(screen.getByLabelText("Número de página")).toHaveValue(1));

      expect(screen.queryByRole("status")).not.toBeInTheDocument();
    });
  }

  const speaking = [
    { name: "on demand", Story: StampOnDemand, line: /Documento grande/, button: "Ver cómo queda" },
    {
      name: "composing",
      Story: StampComposing,
      line: /Componiendo la firma visible/,
      button: null,
    },
    {
      name: "failed",
      Story: StampFailed,
      line: "No se ha podido dibujar la firma visible",
      button: "Reintentar",
    },
  ];

  for (const { name, Story, line, button } of speaking) {
    it(`says what the stamp is doing when it is ${name}, in a slot that is always there`, async () => {
      const { container } = renderWithCatalog(<Story />);

      const pill = await screen.findByRole("status");

      expect(pill).toHaveTextContent(line);
      expect(container.querySelector(".viewer__stamp-slot")).toBeInTheDocument();
      const buttons = within(pill).queryAllByRole("button");
      expect(buttons.map((b) => b.textContent)).toEqual(button === null ? [] : [button]);
      expect(container.querySelector(".viewer__scroll")).not.toContainElement(pill);
    });
  }
});
