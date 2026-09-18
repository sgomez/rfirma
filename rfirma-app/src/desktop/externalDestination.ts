export type ExternalDestination = "discussions";

export interface ExternalDestinationOpener {
  open(destination: ExternalDestination): Promise<void>;
}

export interface InMemoryExternalDestinationOpener extends ExternalDestinationOpener {
  readonly opened: readonly ExternalDestination[];
}

export function inMemoryExternalDestinationOpener(
  onOpen?: (destination: ExternalDestination) => void,
): InMemoryExternalDestinationOpener {
  const opened: ExternalDestination[] = [];
  return {
    get opened() {
      return opened;
    },
    open: async (destination: ExternalDestination) => {
      opened.push(destination);
      onOpen?.(destination);
    },
  };
}

export function unavailableExternalDestinationOpener(): ExternalDestinationOpener {
  return {
    open: () => Promise.reject(new Error("no hay quien abra destinos externos")),
  };
}
