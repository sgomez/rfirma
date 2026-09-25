// Los eventos de la sede: una línea de JSON cada uno en la salida estándar.

let settled = false;
let declared = new Set();

/** Una línea de JSON por evento, y nada más, en la salida estándar. */
export function emit(event) {
  process.stdout.write(`${JSON.stringify(event)}\n`);
}

/** El veredicto, una sola vez: sale cuando el `write` avisa, o un veredicto grande quedaría cortado. */
export function settle(event) {
  if (settled) return;
  settled = true;
  process.stdout.write(`${JSON.stringify(event)}\n`, () => process.exit(0));
}

export const settlingTheError = (type, message) =>
  settle({ event: "error", type: String(type), message: String(message) });

/** Las condiciones que el guion en curso declara en el manifiesto: solo esas se pueden emitir. */
export function declaringTheConditions(names) {
  declared = new Set(names);
}

function theDeclared(name) {
  if (!declared.has(name)) {
    throw new Error(`la condición «${name}» no está en el manifiesto de su guion`);
  }
  return name;
}

/** Los bytes de una respuesta en Base64, venga en el alfabeto estándar o en el URL-safe. */
export function bytesOf(base64) {
  return Buffer.from(String(base64).replace(/-/g, "+").replace(/_/g, "/"), "base64");
}

export function aCondition(name, held, observation) {
  return { name: theDeclared(name), verdict: held ? "compliant" : "discrepant", observation };
}

export function aConditionEvent(name, held, observation) {
  return { event: "condition", ...aCondition(name, held, observation) };
}

/** Una condición que puede quedar sin medir: `held === null` la deja como no observable. */
export function aMeasuredCondition(name, held, observation) {
  if (held === null) return { name: theDeclared(name), verdict: "not_observable", observation };
  return aCondition(name, held, observation);
}

export function aMeasuredConditionEvent(name, held, observation) {
  return { event: "condition", ...aMeasuredCondition(name, held, observation) };
}

/**
 * Sin persona delante, un diálogo que no tenía que salir solo se ve como un trámite que no vuelve:
 * la condición sale no conforme antes de que se agote la espera y se mate al cliente.
 */
export function unansweredMeansAsked(condition) {
  setTimeout(
    () => {
      emit(
        aConditionEvent(
          condition,
          false,
          "el trámite no volvió solo: el cliente preguntó donde no tocaba o se quedó mostrando un error",
        ),
      );
      settle({ event: "done" });
    },
    theUnattendedDeadlineMs(),
  );
}

/** Lo que se espera a un trámite desatendido antes de darlo por preguntado: menos que la paciencia. */
export function theUnattendedDeadlineMs() {
  return Math.max(Number(process.env.RFIRMA_BENCH_TIMEOUT_MS ?? "45000") - 8000, 1000);
}
