import { useEffect, useState } from "react";
import { useLive } from "../suite/live";
import { Modal } from "../ui/Modal";

export function TranscriptModal({
  check,
  report,
  onClose,
}: {
  check: string;
  report?: string;
  onClose: () => void;
}) {
  const { suite } = useLive();
  const [text, setText] = useState<string | null>(null);

  useEffect(() => {
    let current = true;
    suite
      .transcript(check, report)
      .then((transcript) => {
        if (current) {
          setText(
            transcript || "Sin tramas: la comprobación no ha llegado a ejecutarse en este informe.",
          );
        }
      })
      .catch((error: unknown) => {
        if (current) setText(error instanceof Error ? error.message : String(error));
      });
    return () => {
      current = false;
    };
  }, [suite, check, report]);

  return (
    <Modal
      wide
      title={
        <>
          Tramas de <code>{check}</code>
        </>
      }
      onClose={onClose}
    >
      <pre className="transcript">{text ?? "Leyendo…"}</pre>
    </Modal>
  );
}
