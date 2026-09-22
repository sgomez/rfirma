// La sede bajo Node, de la suite y del banco de conformidad: corre un guion contra el cliente.

import { readFileSync } from "node:fs";
import { runInThisContext } from "node:vm";

import { installTheMinimalBrowser } from "./lib/browser.mjs";
import { declaringTheConditions, settle } from "./lib/events.mjs";
import { MODES, SCRIPTS, theManifest } from "./manifest.mjs";

/** Un error de vocabulario: se dice por la salida de eventos y se sale sin correr nada. */
function refusing(type, message) {
  process.stderr.write(`${message}\n`);
  process.stdout.write(`${JSON.stringify({ event: "error", type, message })}\n`, () =>
    process.exit(2),
  );
}

function theScriptToRun(scriptName, modeName) {
  if (!Object.hasOwn(SCRIPTS, scriptName)) {
    return refusing("unknown_script", `el guion «${scriptName}» no está en el manifiesto`);
  }
  if (!Object.hasOwn(MODES, modeName)) {
    return refusing("unknown_mode", `el modo «${modeName}» no está en el manifiesto`);
  }
  const script = SCRIPTS[scriptName];
  if (!script.modes.includes(modeName)) {
    return refusing(
      "mode_not_accepted",
      `el guion «${scriptName}» no funciona en el modo «${modeName}»: ${script.modes.join(", ")}`,
    );
  }
  return script;
}

function running(scriptName, modeName) {
  const script = theScriptToRun(scriptName, modeName);
  if (!script) return;
  const autoscriptPath = process.env.RFIRMA_AUTOSCRIPT;
  if (!autoscriptPath) {
    refusing("no_autoscript", "falta RFIRMA_AUTOSCRIPT");
    return;
  }
  const mode = MODES[modeName];

  installTheMinimalBrowser();
  mode.prepare?.();
  let source = readFileSync(autoscriptPath, "utf8");
  source = mode.patch ? mode.patch(source) : source;
  source = script.patch ? script.patch(source) : source;
  runInThisContext(source, { filename: autoscriptPath });

  SupportDialog.enableSupportDialog(false);
  SupportDialog.enableLoadingDialog(false);
  SupportDialog.enableErrorDialog(false);

  const timeoutMs = Number(process.env.RFIRMA_BENCH_TIMEOUT_MS ?? "45000");
  const timer = setTimeout(() => settle({ event: "timeout" }), timeoutMs);
  timer.unref?.();
  process.on("uncaughtException", (error) => {
    settle({ event: "error", type: "uncaught", message: String(error?.message) });
  });

  declaringTheConditions(script.conditions);
  if (script.site === "published") {
    mode.beforeTheApp?.();
    AutoScript.cargarAppAfirma();
  }
  script.run();
}

if (process.argv.includes("--manifest")) {
  process.stdout.write(`${JSON.stringify(theManifest(), null, 2)}\n`);
} else {
  running(process.env.RFIRMA_BENCH_SCRIPT ?? "selectcert", process.env.RFIRMA_BENCH_MODE ?? "v4");
}
