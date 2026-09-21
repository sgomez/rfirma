import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

HTMLDialogElement.prototype.showModal ??= function showModal(this: HTMLDialogElement) {
  this.open = true;
};
HTMLDialogElement.prototype.close ??= function close(this: HTMLDialogElement) {
  this.open = false;
};
Element.prototype.scrollIntoView ??= () => {};

afterEach(() => {
  cleanup();
  window.localStorage.clear();
});
