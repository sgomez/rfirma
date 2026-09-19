import { Component, type ReactNode } from "react";
import type { ExternalDestinationOpener } from "../desktop/externalDestination";
import { ErrorNotice } from "./ErrorNotice";
import "./RenderErrorBoundary.css";

interface RenderErrorBoundaryProps {
  children: ReactNode;
  externalDestinations?: ExternalDestinationOpener;
  /** Solo lo usan las pruebas: fuera de ellas, recargar la ventana es `window.location.reload()`. */
  onReload?: () => void;
}

interface RenderErrorBoundaryState {
  error: Error | null;
}

/**
 * Un hijo que lanza al pintarse no deja la ventana en blanco.
 *
 * Solo un componente de clase puede ser un *error boundary* — React no ofrece
 * el equivalente en `hook` —, así que esta clase se limita a capturar el fallo
 * y delega la pantalla en `ErrorNotice`, con la situación `renderFailed` y el
 * botón que recarga.
 */
export class RenderErrorBoundary extends Component<
  RenderErrorBoundaryProps,
  RenderErrorBoundaryState
> {
  state: RenderErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: unknown): RenderErrorBoundaryState {
    return { error: error instanceof Error ? error : new Error(String(error)) };
  }

  reload = (): void => {
    if (this.props.onReload) {
      this.props.onReload();
      return;
    }
    window.location.reload();
  };

  render(): ReactNode {
    const { error } = this.state;
    if (error === null) return this.props.children;
    return (
      <div className="render-error-boundary">
        <ErrorNotice
          situation="renderFailed"
          technicalDetail={error.message}
          externalDestinations={this.props.externalDestinations}
          onReload={this.reload}
          focusOnMount
        />
      </div>
    );
  }
}
