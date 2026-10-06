import { composeStories } from "@storybook/react-vite";
import { fireEvent, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { expectNoticeLine } from "../signing/testing/harness";
import { elapse } from "../testing/elapse";
import { renderWithCatalog } from "../testing/render";
import * as consentModule from "./SedeConsent.stories";
import { SedeWindow } from "./SedeWindow";
import {
  certificate,
  consentStage,
  scriptedErrand,
  scriptedFrom,
} from "./testing/fixtures/sedeWindow";

/** Grada A: el momento 2, el consentimiento, con su cuenta atrás. Lo que enseña cada historia está en `SedeWindow.presentation.test.tsx`. */

const { Consent, Batch, IdentityData, PreviousSignaturesValid, PreviousSignaturesWithProblem } =
  composeStories(consentModule);

describe("2 · consent", () => {
  it("is not laid under the dialog scrim, which would cover the certificate list", () => {
    const { port } = scriptedFrom(Consent);
    const { container } = renderWithCatalog(<SedeWindow errands={port} />);

    expect(container.querySelector(".rf-scrim")).toBeNull();
  });

  it("orders the body origin, then certificate, then document", () => {
    const { port } = scriptedFrom(Consent);
    const { container } = renderWithCatalog(<SedeWindow errands={port} />);

    const text = container.textContent ?? "";
    expect(text.indexOf("sede.ejemplo.gob.es pide tu firma")).toBeLessThan(
      text.indexOf("Certificado"),
    );
    expect(text.indexOf("Certificado")).toBeLessThan(text.indexOf("Solicitud de subvención 2026"));
  });

  it("labels the selector «Certificado» also when handing over identity data", () => {
    const { port } = scriptedFrom(IdentityData);
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByRole("combobox", { name: "Certificado" })).toBeInTheDocument();
  });

  it.each([
    ["a signature", Consent],
    ["a batch", Batch],
    ["identity data", IdentityData],
    [
      "a document whose previous signature is broken, with no intermediate dialogue",
      PreviousSignaturesWithProblem,
    ],
  ])("consents with the chosen certificate's handle for %s", async (_, story) => {
    const user = userEvent.setup();
    const { port, calls } = scriptedFrom(story);
    renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);

    await user.click(screen.getByRole("button", { name: /^(Firmar|Enviar mis datos)$/ }));

    expect(calls.consent).toHaveBeenCalledWith("handle-1");
  });

  describe("a site asking for SHA-1 the person has not allowed", () => {
    it("signs just this once, with the countdown even when it is turned off", () => {
      const { port } = scriptedErrand(consentStage({ sha1ToAllow: true }));
      renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);

      expect(screen.getByRole("button", { name: /^Firmar solo esta vez \(\d+\)$/ })).toBeDisabled();
    });
  });

  describe("a local batch asking for SHA-1 the person has not allowed", () => {
    it("signs just this once, with the countdown even when it is turned off", () => {
      const { port } = scriptedErrand(
        consentStage({
          document: null,
          signs: 1,
          signing: null,
          sha1ToAllow: true,
          items: [{ id: "001", signing: "pdf", round: { kind: "sign" } }],
        }),
      );
      renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);

      expect(screen.getByRole("button", { name: /^Firmar solo esta vez \(\d+\)$/ })).toBeDisabled();
    });
  });

  describe("preselection", () => {
    const expired = { kind: "expired", notAfter: 1_600_000_000 } as const;

    it("comes with the remembered certificate chosen", async () => {
      const user = userEvent.setup();
      const { port, calls } = scriptedErrand(
        consentStage({
          certificates: [
            certificate({ id: "first", holderName: "ANA" }),
            certificate({ id: "remembered", holderName: "ZOE", remembered: true }),
          ],
        }),
      );
      renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);

      await user.click(screen.getByRole("button", { name: "Firmar" }));

      expect(calls.consent).toHaveBeenCalledWith("remembered");
    });

    it("comes with the first usable one when the remembered certificate cannot be used", async () => {
      const user = userEvent.setup();
      const { port, calls } = scriptedErrand(
        consentStage({
          certificates: [
            certificate({ id: "remembered", holderName: "ANA", remembered: true, status: expired }),
            certificate({ id: "zoe", holderName: "ZOE" }),
            certificate({ id: "bea", holderName: "BEA" }),
          ],
        }),
      );
      renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);

      await user.click(screen.getByRole("button", { name: "Firmar" }));

      expect(calls.consent).toHaveBeenCalledWith("bea");
    });
  });

  describe("previous signatures of the document", () => {
    it("shows the notice with the right count and states", () => {
      const { port } = scriptedFrom(PreviousSignaturesWithProblem);
      renderWithCatalog(<SedeWindow errands={port} />);

      expectNoticeLine("Junto a 2 firmas", "1 problema");
    });

    it("renders the site variant, where the stylesheet keeps the notice to one line", () => {
      const { port } = scriptedFrom(PreviousSignaturesWithProblem);
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(document.querySelector(".panel__co-signature--site")).toBeInTheDocument();
    });

    it("opens the «Ver firmas» dialog from the site notice", async () => {
      const user = userEvent.setup();
      const { port } = scriptedFrom(PreviousSignaturesValid);
      renderWithCatalog(<SedeWindow errands={port} />);

      await user.click(screen.getByRole("button", { name: "Ver firmas →" }));

      expect(screen.getByRole("dialog", { name: "Firmas del documento" })).toBeInTheDocument();
    });
  });

  describe("the countdown before signing", () => {
    beforeEach(() => vi.useFakeTimers());
    afterEach(() => vi.useRealTimers());

    const elapseCountdown = async () => {
      for (let second = 0; second < 3; second++) await elapse(1000);
    };

    it("counts Firmar (3), (2), (1) down disabled, and enables Firmar after three seconds", async () => {
      const { port } = scriptedFrom(Consent);
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByRole("button", { name: "Firmar (3)" })).toBeDisabled();
      await elapse(1000);
      expect(screen.getByRole("button", { name: "Firmar (2)" })).toBeDisabled();
      await elapse(1000);
      expect(screen.getByRole("button", { name: "Firmar (1)" })).toBeDisabled();
      await elapse(1000);
      expect(screen.getByRole("button", { name: "Firmar" })).toBeEnabled();
    });

    it("focuses Firmar once the countdown ends, so Enter signs with the remembered certificate", async () => {
      const { port, calls } = scriptedErrand(
        consentStage({
          certificates: [
            certificate(),
            certificate({ id: "handle-2", label: "Otro", remembered: true }),
          ],
        }),
      );
      renderWithCatalog(<SedeWindow errands={port} />);

      await elapseCountdown();
      expect(screen.getByRole("button", { name: "Firmar" })).toHaveFocus();
      fireEvent.click(document.activeElement as HTMLElement);

      expect(calls.consent).toHaveBeenCalledWith("handle-2");
    });

    it("does not take the focus back from what the person moved it to during the countdown", async () => {
      const { port } = scriptedFrom(Consent);
      renderWithCatalog(<SedeWindow errands={port} />);

      screen.getByRole("button", { name: "Cancelar" }).focus();
      await elapseCountdown();

      expect(screen.getByRole("button", { name: "Cancelar" })).toHaveFocus();
    });

    it("counts down «Enviar mis datos» too: sending an identity is no less final", () => {
      const { port } = scriptedFrom(IdentityData);
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByRole("button", { name: "Enviar mis datos (3)" })).toBeDisabled();
    });

    it("starts enabled and focused, with no number, when the person turned the countdown off", () => {
      const { port } = scriptedFrom(Consent);
      renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);

      const sign = screen.getByRole("button", { name: "Firmar" });
      expect(sign).toBeEnabled();
      expect(sign).toHaveFocus();
    });
  });
});
