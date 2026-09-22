import { useEffect, useState } from "react";
import { clock } from "../words";

/** Un reloj que cuenta desde `since`, un instante de `performance.now()`, y solo se repinta a sí mismo. */
export function Elapsed({ since }: { since: number }) {
  const [now, setNow] = useState(() => performance.now());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(performance.now()), 250);
    return () => window.clearInterval(timer);
  }, []);
  const ms = now - since;
  return (
    <time className="elapsed" dateTime={`PT${Math.max(0, Math.floor(ms / 1000))}S`}>
      {clock(ms)}
    </time>
  );
}
