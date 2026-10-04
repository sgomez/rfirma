//! Lo que `/design-sync` compila para Claude Design: la ventana principal y sus piezas, el panel de estado y la retirada del certificado, la vista de la ventana de sede, los diálogos de firma, el resultado de la firma, el selector y la tarjeta de certificado, los primitivos, la raíz con idioma y tema y los iconos.

import "./src/design-system/index.css";
import "./src/app.css";

export { Badge } from "./src/design-system/Badge";
export { Button } from "./src/design-system/Button";
export { Card } from "./src/design-system/Card";
export { DesignRoot } from "./src/design-system/DesignRoot";
export { Dialog } from "./src/design-system/Dialog";
export { Field } from "./src/design-system/Field";
export * from "./src/design-system/icons";
export { Popover } from "./src/design-system/Popover";
export { Row } from "./src/design-system/Row";
export { Stack } from "./src/design-system/Stack";
export { DocumentTabs } from "./src/documents/DocumentTabs";
export { RecentRows, RecentsSection } from "./src/documents/RecentRows";
export { ErrorNotice } from "./src/errors/ErrorNotice";
export { SedeView } from "./src/sede/SedeView";
export { Header } from "./src/shell/Header";
export { MainWindow } from "./src/shell/MainWindow";
export { CertificateCard } from "./src/signing/CertificateCard";
export { CertificateSelect } from "./src/signing/CertificateSelect";
export { SignAnywayDialog } from "./src/signing/SignAnywayDialog";
export { SignaturesDialog } from "./src/signing/SignaturesDialog";
export { SignedPanel } from "./src/signing/SignedPanel";
export { SigningProgressDialog } from "./src/signing/SigningProgressDialog";
export { UnsealedPagesDialog } from "./src/signing/UnsealedPagesDialog";
export { StatusView } from "./src/status/StatusView";
export { WithdrawCertificateView } from "./src/status/WithdrawCertificateView";
export { NewVersionStrip } from "./src/updates/NewVersionStrip";
export { DocumentViewer } from "./src/viewer/DocumentViewer";
