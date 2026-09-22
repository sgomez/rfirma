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

/** Una condición que puede quedar sin medir: `held === null` la emite como no observable. */
export function aMeasuredConditionEvent(name, held, observation) {
  if (held === null) {
    return { event: "condition", name: theDeclared(name), verdict: "not_observable", observation };
  }
  return aConditionEvent(name, held, observation);
}
