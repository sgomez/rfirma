import { Route, Routes } from "react-router";
import { ComparePage } from "./compare/ComparePage";
import { ReportPage } from "./report/ReportPage";
import { SessionPage } from "./session/SessionPage";
import { Shell } from "./shell/Shell";
import { SuiteProvider } from "./suite/live";
import type { Suite } from "./suite/suite";

export function App({ suite }: { suite: Suite }) {
  return (
    <SuiteProvider suite={suite}>
      <Shell>
        <Routes>
          <Route path="/" element={<SessionPage />} />
          <Route path="/informe/:name" element={<ReportPage />} />
          <Route path="/comparar" element={<ComparePage />} />
        </Routes>
      </Shell>
    </SuiteProvider>
  );
}
