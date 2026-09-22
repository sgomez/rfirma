import { render } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { App } from "../App";
import type { Snapshot } from "../contract/Snapshot";
import { suiteOver } from "../suite/suite";
import { FakeServer } from "./fakeServer";
import { aSnapshot } from "./fixtures";

export function renderConsoleAt(
  path: string,
  snapshot: Snapshot = aSnapshot(),
  prepare: (server: FakeServer) => void = () => {},
) {
  const server = new FakeServer(snapshot);
  prepare(server);
  const separator = path.includes("?") ? "&" : "?";
  const user = userEvent.setup();
  render(
    <MemoryRouter initialEntries={[`${path}${separator}token=t0k`]}>
      <App suite={suiteOver(server, "t0k")} />
    </MemoryRouter>,
  );
  return { server, user };
}
