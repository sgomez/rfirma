import type { ResultName } from "../contract/ResultName";
import { resultTone } from "../words";

export type Activity = "running" | "waiting" | "queued";

const ACTIVITY_LABEL: Record<Activity, string> = {
  running: "en curso",
  waiting: "esperando a que des paso",
  queued: "en cola",
};

function Glyph({ result }: { result: ResultName }) {
  switch (result) {
    case "CONFORME":
      return (
        <>
          <circle cx="8" cy="8" r="6.5" className="fill" />
          <path d="M5.2 8.2l1.9 1.9 3.8-4.1" className="mark" />
        </>
      );
    case "NO CONFORME":
      return (
        <>
          <rect x="1.75" y="1.75" width="12.5" height="12.5" rx="2.5" className="fill" />
          <path d="M5.6 5.6l4.8 4.8M10.4 5.6l-4.8 4.8" className="mark" />
        </>
      );
    case "NO OBSERVABLE":
      return (
        <>
          <path d="M8 1.2l6.8 6.8L8 14.8 1.2 8z" className="fill" />
          <path d="M6.1 6.6a1.9 1.9 0 1 1 2.6 1.8c-.5.2-.7.5-.7 1" className="mark" />
          <circle cx="8" cy="11.3" r=".75" className="dot" />
        </>
      );
    case "PENDIENTE":
      return <circle cx="8" cy="8" r="5.5" className="ring" />;
  }
}

/** El resultado como forma y color, para leerlo sin distinguir el rojo del verde. */
export function ResultIcon({
  result,
  size = 16,
  decorative = false,
}: {
  result: ResultName;
  size?: number;
  decorative?: boolean;
}) {
  return (
    <svg
      className={`icon result tone-${resultTone[result]}`}
      width={size}
      height={size}
      viewBox="0 0 16 16"
      role={decorative ? undefined : "img"}
      aria-label={decorative ? undefined : result}
      aria-hidden={decorative || undefined}
    >
      <Glyph result={result} />
    </svg>
  );
}

export function ActivityIcon({ activity, size = 16 }: { activity: Activity; size?: number }) {
  return (
    <svg
      className={`icon activity activity-${activity}`}
      width={size}
      height={size}
      viewBox="0 0 16 16"
      role="img"
      aria-label={ACTIVITY_LABEL[activity]}
    >
      {activity === "running" && (
        <>
          <circle cx="8" cy="8" r="6" className="track" />
          <path d="M8 2a6 6 0 0 1 6 6" className="arc" />
        </>
      )}
      {activity === "waiting" && (
        <>
          <circle cx="8" cy="8" r="6.5" className="halo" />
          <circle cx="8" cy="8" r="4" className="fill" />
        </>
      )}
      {activity === "queued" && <circle cx="8" cy="8" r="5.5" className="dashed" />}
    </svg>
  );
}

export function Chevron({ open }: { open: boolean }) {
  return (
    <svg
      className={`chevron${open ? " open" : ""}`}
      width="12"
      height="12"
      viewBox="0 0 12 12"
      aria-hidden="true"
    >
      <path d="M4.5 2.5L8 6l-3.5 3.5" />
    </svg>
  );
}

export function PlayIcon() {
  return (
    <svg className="glyph" width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
      <path d="M2.2 1.3v7.4L8.6 5z" />
    </svg>
  );
}

export function CopyIcon() {
  return (
    <svg className="glyph stroke" width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
      <rect x="4" y="4" width="6.5" height="6.5" rx="1" />
      <path d="M8 2.5V2a.5.5 0 0 0-.5-.5h-5A.5.5 0 0 0 2 2v5.5a.5.5 0 0 0 .5.5H3" />
    </svg>
  );
}

export function StopIcon() {
  return (
    <svg className="glyph" width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
      <rect x="1.5" y="1.5" width="7" height="7" rx="1" />
    </svg>
  );
}

export function SkipIcon() {
  return (
    <svg className="glyph" width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
      <path d="M1.2 1.5v7L6 5zM7 1.5h1.6v7H7z" />
    </svg>
  );
}

export function ExternalIcon() {
  return (
    <svg className="glyph stroke" width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
      <path d="M4 1.5H1.5v7h7V6M6 1.5h2.5V4M8.5 1.5L4.5 5.5" />
    </svg>
  );
}

export function SunIcon() {
  return (
    <svg className="glyph stroke" width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">
      <circle cx="7" cy="7" r="2.6" />
      <path d="M7 .9v1.5M7 11.6v1.5M.9 7h1.5M11.6 7h1.5M2.7 2.7l1 1M10.3 10.3l1 1M2.7 11.3l1-1M10.3 3.7l1-1" />
    </svg>
  );
}

export function MoonIcon() {
  return (
    <svg className="glyph stroke" width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">
      <path d="M11.8 8.6A5 5 0 0 1 5.4 2.2a5 5 0 1 0 6.4 6.4z" />
    </svg>
  );
}

export function CheckMark() {
  return (
    <svg className="glyph stroke" width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
      <path d="M1.8 5.3l2.1 2.1 4.3-4.8" />
    </svg>
  );
}
