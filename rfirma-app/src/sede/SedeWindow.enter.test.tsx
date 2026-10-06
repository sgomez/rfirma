import { composeStories } from "@storybook/react-vite";
import { fireEvent, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { elapse } from "../testing/elapse";
import { renderWithCatalog } from "../testing/render";
import * as confirmModule from "./SedeConfirm.stories";
import * as consentModule from "./SedeConsent.stories";
import * as noCertificateModule from "./SedeNoCertificate.stories";
import * as oldWebClientModule from "./SedeOldWebClient.stories";
import * as outcomeModule from "./SedeOutcome.stories";
import * as signingModule from "./SedeSigning.stories";
import * as waitingModule from "./SedeWaiting.stories";
import { SedeWindow } from "./SedeWindow";
import { scriptedFrom } from "./testing/fixtures/sedeWindow";

/** Grada A: Intro es la acción principal de cada momento, y no hace nada donde no la hay. */

const { OldWebClient } = composeStories(oldWebClientModule);
const { Waiting, NoChannel } = composeStories(waitingModule);
const { Consent } = composeStories(consentModule);
const { ShadowAttackSuspect } = composeStories(confirmModule);
const { Signing, Returning } = composeStories(signingModule);
const { NoneInstalled, TerminalExcludedByFilter } = composeStories(noCertificateModule);
const { Signed } = composeStories(outcomeModule);

const pressEnter = () => fireEvent.keyDown(document, { key: "Enter" });

describe("Enter", () => {
  it("goes on in the validator's confirmation, which comes before consenting", () => {
    const { port, calls } = scriptedFrom(ShadowAttackSuspect);
    renderWithCatalog(<SedeWindow errands={port} />);

    pressEnter();

    expect(calls.confirmSignatures).toHaveBeenCalledOnce();
    expect(calls.consent).not.toHaveBeenCalled();
  });

  it("installs another certificate when there is none", () => {
    const { port, calls } = scriptedFrom(NoneInstalled);
    renderWithCatalog(<SedeWindow errands={port} />);

    pressEnter();

    expect(calls.installCertificate).toHaveBeenCalledOnce();
  });

  it("leaves when the filter excluded every certificate and the order came from the terminal", () => {
    const { port, calls } = scriptedFrom(TerminalExcludedByFilter);
    renderWithCatalog(<SedeWindow errands={port} />);

    pressEnter();

    expect(calls.cancel).toHaveBeenCalledOnce();
  });

  it("installs the local CA when the connection failed", () => {
    const { port, calls } = scriptedFrom(NoChannel);
    renderWithCatalog(<SedeWindow errands={port} />);

    pressEnter();

    expect(calls.installLocalCa).toHaveBeenCalledOnce();
  });

  it("closes the outcome", () => {
    const { port, calls } = scriptedFrom(Signed);
    renderWithCatalog(<SedeWindow errands={port} />);

    pressEnter();

    expect(calls.close).toHaveBeenCalledOnce();
  });

  it("goes on past the old web client warning", () => {
    const { port, calls } = scriptedFrom(OldWebClient);
    renderWithCatalog(<SedeWindow errands={port} />);

    pressEnter();

    expect(calls.dismissWarning).toHaveBeenCalledOnce();
  });

  it.each([
    ["waiting for the channel", Waiting],
    ["signing", Signing],
    ["returning the signature", Returning],
  ])("does nothing while %s", (_, story) => {
    const { port, calls } = scriptedFrom(story);
    renderWithCatalog(<SedeWindow errands={port} />);

    pressEnter();

    expect(calls.cancel).not.toHaveBeenCalled();
    expect(calls.close).not.toHaveBeenCalled();
    expect(calls.consent).not.toHaveBeenCalled();
  });

  it("opens the certificate list from the dropdown instead of signing", async () => {
    const user = userEvent.setup();
    const { port, calls } = scriptedFrom(Consent);
    renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);
    screen.getByRole("combobox").focus();

    await user.keyboard("{Enter}");

    expect(screen.getByRole("listbox")).toBeInTheDocument();
    expect(calls.consent).not.toHaveBeenCalled();
  });

  describe("in the consent", () => {
    beforeEach(() => vi.useFakeTimers());
    afterEach(() => vi.useRealTimers());

    it("does nothing during the countdown", async () => {
      const { port, calls } = scriptedFrom(Consent);
      renderWithCatalog(<SedeWindow errands={port} />);

      pressEnter();
      await elapse(2000);
      pressEnter();

      expect(calls.consent).not.toHaveBeenCalled();
    });

    it("signs once the countdown is over", async () => {
      const { port, calls } = scriptedFrom(Consent);
      renderWithCatalog(<SedeWindow errands={port} />);

      for (let second = 0; second < 3; second++) await elapse(1000);
      pressEnter();

      expect(calls.consent).toHaveBeenCalledOnce();
    });
  });
});
