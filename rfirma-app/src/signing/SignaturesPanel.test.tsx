import { composeStories } from "@storybook/react-vite";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ComponentProps } from "react";
import { fn } from "storybook/test";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import type { PanelFooter } from "./PanelFooter";
import type { SignaturesMoment } from "./SignaturesPanel";
import * as stories from "./SignaturesPanel.stories";

const { JustSigned, VerifyWithSignatures } = composeStories(stories);

// Lo que se ve en cada momento, en `SignaturesPanel.presentation.test.tsx`.
describe("SignaturesPanel", () => {
  describe("in the acknowledgement", () => {
    it("calls onChangeDestination once when Cambiar is pressed", async () => {
      const onChangeDestination = fn();
      const moment: SignaturesMoment = {
        kind: "acknowledgement",
        signedAt: new Date(2026, 9, 3, 11, 4),
        signatures: [],
        findings: [],
        onChangeDestination,
      };
      renderWithCatalog(<JustSigned moment={moment} />);

      await userEvent.click(screen.getByRole("button", { name: "Cambiar" }));

      expect(onChangeDestination).toHaveBeenCalledOnce();
    });

    it.each([
      ["Abrir el PDF", "onOpenDocument"],
      ["Abrir la carpeta", "onOpenFolder"],
      ["Firmar", "onSign"],
    ] as const)("calls %s once when pressed", async (name, handler) => {
      const spy = fn();
      renderWithCatalog(<JustSigned {...{ [handler]: spy }} />);

      await userEvent.click(screen.getByRole("button", { name }));

      expect(spy).toHaveBeenCalledOnce();
    });
  });

  describe("in the signature reading", () => {
    it.each([
      ["Abrir el PDF", "onOpenDocument"],
      ["Abrir la carpeta", "onOpenFolder"],
      ["Firmar", "onSign"],
    ] as const)("calls %s once when pressed", async (name, handler) => {
      const spy = fn();
      renderWithCatalog(<VerifyWithSignatures {...{ [handler]: spy }} />);

      await userEvent.click(screen.getByRole("button", { name }));

      expect(spy).toHaveBeenCalledOnce();
    });

    it("offers no «Cambiar», and neither the panel nor the footer accept one", () => {
      const onChangeDestination = () => {};
      // @ts-expect-error
      const panelMoment: SignaturesMoment = {
        kind: "reading",
        state: { kind: "reading" },
        signable: true,
        onChangeDestination,
      };
      // @ts-expect-error
      const footer: ComponentProps<typeof PanelFooter> = {
        moment: "reading",
        destination: { folder: "", name: null, writable: true },
        documentName: "contrato.pdf",
        onOpenDocument: () => {},
        onOpenFolder: () => {},
        onSign: () => {},
        signable: true,
        onChangeDestination,
      };
      renderWithCatalog(<VerifyWithSignatures />);

      expect(screen.queryByRole("button", { name: "Cambiar" })).not.toBeInTheDocument();
      expect([panelMoment.kind, footer.moment]).toEqual(["reading", "reading"]);
    });
  });
});
