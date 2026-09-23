// Los guiones del canal WebSocket: el protocolo escrito en crudo y el canal que reutiliza la sede publicada.

import { createServer } from "node:net";

import { theLaunchesSoFar } from "../lib/browser.mjs";
import { aConditionEvent, aMeasuredConditionEvent, emit, settle } from "../lib/events.mjs";
import { theThirdProtocolPort } from "../lib/modes.mjs";
import { aHandwrittenScript, aPublishedScript } from "../lib/script.mjs";

const A_CANDIDATE_PORT_BOUND = "a-candidate-port-bound";
const THE_ECHO_WITH_ITS_SESSION_ANSWERS_OK = "the-echo-with-its-session-answers-ok";
const THE_ECHO_WITHOUT_A_SESSION_ANSWERS_SAF_46 = "the-echo-without-a-session-answers-saf-46";
const A_SECOND_CLIENT_LEAVES_THE_CHANNEL_ALIVE = "a-second-client-leaves-the-channel-alive";
const A_MALFORMED_SESSION_BINDS_NOTHING = "a-malformed-session-binds-nothing";
const THE_FIXED_PORT_BOUND = "the-fixed-port-bound";
const A_BARE_ECHO_ANSWERS_OK = "a-bare-echo-answers-ok";
const AN_ECHO_WITHOUT_A_SESSION_IS_NOT_REFUSED = "an-echo-without-a-session-is-not-refused";
const NO_CHANNEL_OPENS = "no-channel-opens";
const THE_FIRST_FREE_CANDIDATE_BOUND = "the-first-free-candidate-bound";
const THE_ECHO_WITH_ANOTHER_SESSION_ANSWERS_SAF_46 = "the-echo-with-another-session-answers-saf-46";
const AN_UNSUPPORTED_VERSION_OPENS_NO_CHANNEL = "an-unsupported-version-opens-no-channel";
const THE_CHANNEL_OPENS_DESPITE_THE_WARNING = "the-channel-opens-despite-the-warning";

/** Lo que se espera a que un arranque que no debe abrir canal lo abra antes de darlo por no abierto. */
const THE_UNOPENED_CHANNEL_PATIENCE_MS = 10000;

/** Lo que se espera a que la persona cierre el aviso del cliente antes de que abra el canal. */
const THE_WARNED_CHANNEL_PATIENCE_MS = 60000;

/** Lo que se espera a cada rechazo mientras la persona cierra el diálogo de error de AutoFirma. */
const THE_DIALOGUE_PATIENCE_MS = 120000;

/** Lo que tarda como poco un rechazo retenido por un diálogo que la persona cierra a propósito. */
const THE_DIALOGUE_MIN_MS = 3000;

const THE_REJECTION_WAITED_FOR_THE_DIALOGUE = "the-rejection-waited-for-the-dialogue";
const THE_SECOND_OPERATION_REUSES_THE_CHANNEL = "the-second-operation-reuses-the-channel";
const THE_NAME_ARRIVES_IN_UTF8 = "the-name-arrives-in-utf8";
const THE_SELECTION_ANSWERS_A_CERTIFICATE = "the-selection-answers-a-certificate";

function connectWebSocket(port) {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(`wss://127.0.0.1:${port}`);
    ws.onopen = () => resolve(ws);
    ws.onerror = (err) => reject(err);
  });
}

function exchange(ws, message) {
  return new Promise((resolve) => {
    const onMessage = (event) => {
      ws.removeEventListener("message", onMessage);
      resolve(String(event.data));
    };
    ws.addEventListener("message", onMessage);
    ws.send(message);
  });
}

/** Lo que se espera a cada respuesta de una operación: AutoFirma la retiene tras un diálogo modal. */
const THE_OPERATION_ANSWER_DEADLINE_MS = 10000;

/** Un `exchange` que se rinde a los `deadlineMs` y resuelve `null` si nadie ha contestado. */
function exchangeWithin(ws, message, deadlineMs) {
  return Promise.race([
    exchange(ws, message),
    new Promise((resolve) => setTimeout(() => resolve(null), deadlineMs)),
  ]);
}

function isFree(port) {
  return new Promise((resolve) => {
    const server = createServer();
    server.once("error", () => resolve(false));
    server.listen(port, "127.0.0.1", () => server.close(() => resolve(true)));
  });
}

/** El primer candidato que nadie ocupa antes del lanzamiento, o `null` si están todos ocupados. */
async function theFirstFreePort(ports) {
  for (const port of ports) {
    if (await isFree(port)) return port;
  }
  return null;
}

/**
 * Lanza el sujeto en v4 y abre el canal en el primer puerto candidato que conteste; si no contesta
 * ninguno, llama a `beforeGivingUp` antes de rendirse.
 */
async function theProtocolV4ChannelOpening(ports, idSession, beforeGivingUp = () => {}) {
  emit({
    event: "launch",
    url: `afirma://websocket?ports=${ports.join(",")}&v=4&jvc=3&idsession=${idSession}`,
  });
  await new Promise((r) => setTimeout(r, 3000));

  for (const port of ports) {
    try {
      return { ws: await connectWebSocket(port), port };
    } catch {}
  }
  beforeGivingUp();
  emit({
    event: "error",
    type: "cannot_connect",
    message: "no se pudo conectar a los puertos candidatos",
  });
  settle({ event: "error" });
  return null;
}

async function theProtocolV4Script() {
  const idSession = "K3m9Pq2XyZ1w8A4bC7dE";
  const candidates = [54321, 54322, 54323];
  const firstFree = await theFirstFreePort(candidates);
  const channel = await theProtocolV4ChannelOpening(candidates, idSession, () => {
    if (firstFree !== null && firstFree !== candidates[0]) {
      emit(
        aConditionEvent(
          THE_FIRST_FREE_CANDIDATE_BOUND,
          false,
          `primer candidato libre ${firstFree}; no contestó ningún candidato`,
        ),
      );
    }
  });
  if (!channel) return;
  const { ws: ws1, port: connectedPort } = channel;
  emit(aConditionEvent(A_CANDIDATE_PORT_BOUND, true, `conectado en puerto ${connectedPort}`));
  emit(
    aConditionEvent(
      THE_FIRST_FREE_CANDIDATE_BOUND,
      connectedPort === firstFree,
      `primer candidato libre ${firstFree}; canal en ${connectedPort}`,
    ),
  );

  const echoResp = await exchange(ws1, `echo=-idsession=${idSession}@EOF`);
  emit(
    aConditionEvent(
      THE_ECHO_WITH_ITS_SESSION_ANSWERS_OK,
      echoResp === "OK",
      `canal abierto en wss://127.0.0.1:${connectedPort} sin idsession; el eco contestó ${echoResp}`,
    ),
  );

  const absentResp = await exchange(ws1, "echo=@EOF");
  emit(
    aConditionEvent(
      THE_ECHO_WITHOUT_A_SESSION_ANSWERS_SAF_46,
      absentResp.startsWith("SAF_46"),
      absentResp,
    ),
  );

  const foreignResp = await exchangeWithin(
    ws1,
    "echo=-idsession=Zz8Yy7Xx6Ww5Vv4Uu3Tt@EOF",
    THE_OPERATION_ANSWER_DEADLINE_MS,
  );
  emit(
    aMeasuredConditionEvent(
      THE_ECHO_WITH_ANOTHER_SESSION_ANSWERS_SAF_46,
      foreignResp === null ? null : foreignResp.startsWith("SAF_46"),
      foreignResp ?? "el eco con otra sesión no tuvo respuesta",
    ),
  );

  try {
    const ws2 = await connectWebSocket(connectedPort);
    ws2.close();
    await new Promise((r) => setTimeout(r, 200));
    const ping = await exchange(ws1, `echo=-idsession=${idSession}@EOF`);
    emit(
      aConditionEvent(
        A_SECOND_CLIENT_LEAVES_THE_CHANNEL_ALIVE,
        ping === "OK",
        ping === "OK" ? "el servidor sigue vivo tras cerrar el cliente secundario" : "cerrado",
      ),
    );
  } catch (err) {
    emit(
      aMeasuredConditionEvent(A_SECOND_CLIENT_LEAVES_THE_CHANNEL_ALIVE, null, String(err?.message)),
    );
  }

  await theOrdersOverTheChannel(ws1, idSession, [
    ...THE_V4_OPERATION_PROBES,
    ...THE_PASSING_PARAMETER_PROBES,
  ]);
  ws1.close();
  settle({ event: "success" });
}

/** Un canal v4 abierto solo para mandar `probes`, esperando cada respuesta hasta `deadlineMs`. */
function theProbesOverTheFourthProtocol(ports, probes, deadlineMs) {
  return async () => {
    const idSession = "Rj5Ct7Pr9Ob1Es3Tt5Ab";
    const channel = await theProtocolV4ChannelOpening(ports, idSession);
    if (!channel) return;
    await theOrdersOverTheChannel(channel.ws, idSession, probes, deadlineMs);
    channel.ws.close();
    settle({ event: "success" });
  };
}

/** Un rechazo de parámetros y cuánto tardó en llegar: AutoFirma lo retiene tras su diálogo. */
async function theShownRejectionScript() {
  const idSession = "Sh6Rj8Dl0Gq2Ue4Tt6Ab";
  const channel = await theProtocolV4ChannelOpening([54401, 54402, 54403], idSession);
  if (!channel) return;
  const sentAt = Date.now();
  const answer = await exchangeWithin(
    channel.ws,
    `afirma://sign?op=sign&format=CAdES&algorithm=SHA256withRSA&dat=SG9sYQ&id=rfirma-1&idsession=${idSession}`,
    THE_DIALOGUE_PATIENCE_MS,
  );
  const waitedMs = Date.now() - sentAt;
  channel.ws.close();
  emit(
    aMeasuredConditionEvent(
      THE_REJECTION_WAITED_FOR_THE_DIALOGUE,
      answer === null ? null : answer.startsWith("SAF_03") && waitedMs >= THE_DIALOGUE_MIN_MS,
      answer === null
        ? `nadie contestó en ${THE_DIALOGUE_PATIENCE_MS / 1000} s`
        : `${answer} a los ${(waitedMs / 1000).toFixed(1)} s`,
    ),
  );
  settle({ event: "success" });
}

/** Una selección que nombra el almacén del sistema en `keystore` y el token de SoftHSM en `ksb64`. */
async function theKeyStoreOverKsb64Script() {
  const idSession = "Ks4Pr6Ec8Db0Ks2Bs4Xy";
  const channel = await theProtocolV4ChannelOpening([54481, 54482, 54483], idSession);
  if (!channel) return;
  const token = Buffer.from("PKCS11:/usr/lib/softhsm/libsofthsm2.so").toString("base64");
  const answer = await exchangeWithin(
    channel.ws,
    `afirma://selectcert?keystore=SHARED_NSS&ksb64=${token}&idsession=${idSession}`,
    THE_DIALOGUE_PATIENCE_MS,
  );
  channel.ws.close();
  emit(
    aMeasuredConditionEvent(
      THE_SELECTION_ANSWERS_A_CERTIFICATE,
      answer === null ? null : !/^(SAF_|CANCEL)/.test(answer),
      answer === null
        ? `nadie contestó en ${THE_DIALOGUE_PATIENCE_MS / 1000} s`
        : `contestó ${answer.slice(0, 40)}`,
    ),
  );
  settle({ event: "success" });
}

/** Una firma que pasa el análisis de parámetros y se para en el formato inventado (`SAF_06`). */
function aSignOrderStoppingAtTheFormat(idSession, { op = "sign", probed } = {}) {
  const data = Buffer.from("rfirma").toString("base64");
  const extra = probed ? `&${probed}` : "";
  return `afirma://sign?op=${op}&format=INVENTADO&algorithm=SHA256&dat=${data}${extra}&idsession=${idSession}`;
}

/** Las operaciones que se mandan por el canal v4 y que se rechazan sin abrir ningún diálogo. */
const THE_V4_OPERATION_PROBES = [
  {
    condition: "ver-5-is-ignored",
    order: (idSession) => aSignOrderStoppingAtTheFormat(idSession, { probed: "ver=5" }),
    holds: (answer) => answer.startsWith("SAF_06"),
  },
  {
    condition: "mcv-above-the-client-saf-41",
    order: (idSession) => aSignOrderStoppingAtTheFormat(idSession, { probed: "mcv=99.0.0" }),
    holds: (answer) => answer.startsWith("SAF_41"),
  },
  {
    condition: "mcv-below-the-client-passes",
    order: (idSession) => aSignOrderStoppingAtTheFormat(idSession, { probed: "mcv=1.0.0" }),
    holds: (answer) => !answer.startsWith("SAF_41"),
  },
  {
    condition: "invented-format-saf-06",
    order: (idSession) => aSignOrderStoppingAtTheFormat(idSession),
    holds: (answer) => answer.startsWith("SAF_06"),
  },
  ...["sign", "cosign", "countersign"].map((verb) => ({
    condition: `${verb}-with-a-slash-saf-06`,
    order: (idSession) =>
      aSignOrderStoppingAtTheFormat(idSession, { op: verb }).replace(
        "afirma://sign?",
        `afirma://${verb}/?`,
      ),
    holds: (answer) => answer.startsWith("SAF_06"),
  })),
];

/** Un rechazo de los parámetros de la petición: en AutoFirma, un diálogo de error antes de contestar. */
const aParameterRejection = (condition, order, expected) => ({
  condition,
  order,
  holds: (answer) => answer.startsWith(expected),
});

/** El `rtservlet` en el bucle local, que se rechaza con `SAF_13`. */
const THE_LOCAL_RTSERVLET_PROBE = aParameterRejection(
  "local-rtservlet-saf-13",
  (idSession) =>
    `afirma://sign?op=sign&fileid=rfirma&rtservlet=http://127.0.0.1/rt&idsession=${idSession}`,
  "SAF_13",
);

/** Los rechazos del análisis de la petición. */
const THE_V4_REJECTION_PROBES = [
  aParameterRejection(
    "unknown-operation-saf-04",
    (idSession) => `afirma://unknownop?idsession=${idSession}`,
    "SAF_04",
  ),
  ...["selectcert", "batch"].map((verb) =>
    aParameterRejection(
      `${verb}-with-a-slash-saf-03`,
      (idSession) => `afirma://${verb}/?fileid=abc123&idsession=${idSession}`,
      "SAF_03",
    ),
  ),
  aParameterRejection(
    "no-data-saf-03",
    (idSession) =>
      `afirma://sign?op=sign&id=rfirma-1&format=CAdES&algorithm=SHA256&idsession=${idSession}`,
    "SAF_03",
  ),
  aParameterRejection(
    "a-cipher-key-of-seven-saf-03",
    (idSession) => aSignOrderStoppingAtTheFormat(idSession, { probed: "key=1234567" }),
    "SAF_03",
  ),
  aParameterRejection(
    "fileid-without-rtservlet-saf-03",
    (idSession) =>
      `afirma://sign?op=sign&format=NoSuchFormat&algorithm=SHA256withRSA&fileid=abc123&idsession=${idSession}`,
    "SAF_03",
  ),
];

/** La operación inválida, con un formato que no existe: AutoFirma no valida `op` y se para en el formato. */
const THE_INVALID_OPERATION_PROBE = aParameterRejection(
  "invalid-op-saf-04",
  (idSession) => aSignOrderStoppingAtTheFormat(idSession, { op: "invalid" }),
  "SAF_04",
);

/** Manda por el canal abierto cada orden y juzga su respuesta; tras el primer silencio no manda más. */
async function theOrdersOverTheChannel(
  ws,
  idSession,
  probes,
  deadlineMs = THE_OPERATION_ANSWER_DEADLINE_MS,
) {
  let silentAt = null;
  for (const { condition, order, holds } of probes) {
    const answer = silentAt ? null : await exchangeWithin(ws, order(idSession), deadlineMs);
    if (answer === null) {
      silentAt ??= condition;
      emit(
        aMeasuredConditionEvent(
          condition,
          null,
          `el sujeto dejó de contestar en ${silentAt}: ¿lo retiene un diálogo modal?`,
        ),
      );
      continue;
    }
    emit(aConditionEvent(condition, holds(answer), answer));
  }
}

async function theProtocolV4MalformedIdScript() {
  const ports = [54331, 54332, 54333];
  emit({
    event: "launch",
    url: `afirma://websocket?ports=${ports.join(",")}&v=4&jvc=3&idsession=mal%20formed!`,
  });
  await new Promise((r) => setTimeout(r, 3000));
  let ws = null;
  for (const p of ports) {
    try {
      ws = await connectWebSocket(p);
      break;
    } catch {}
  }
  if (!ws) {
    emit(
      aConditionEvent(
        A_MALFORMED_SESSION_BINDS_NOTHING,
        true,
        "rechazado en arranque con idsession inválido",
      ),
    );
    settle({ event: "success" });
    return;
  }
  const echoResp = await exchange(ws, "echo=-idsession=arbitraria@EOF");
  emit(
    aConditionEvent(A_MALFORMED_SESSION_BINDS_NOTHING, !echoResp.startsWith("SAF_46"), echoResp),
  );
  ws.close();
  settle({ event: "success" });
}

/** Los parámetros comunes: si pasa el control medido, la petición acaba en un formato inexistente. */
const THE_PARAMETER_CASES = [
  ["dat-with-a-local-file-saf-03", "dat=file:/etc/hostname", "SAF_03"],
  ["rtservlet-over-ftp-saf-03", "fileid=abc123&rtservlet=ftp://sede.example/rt", "SAF_03"],
  ["rtservlet-on-localhost-saf-13", "fileid=abc123&rtservlet=http://localhost/rt", "SAF_13"],
  [
    "rtservlet-with-a-query-saf-03",
    `fileid=abc123&rtservlet=${encodeURIComponent("https://sede.example/rt?op=get")}`,
    "SAF_03",
  ],
  ["mcv-malformed-saf-03", "dat=SG9sYQ&mcv=uno.dos", "SAF_03"],
  ["id-of-21-saf-03", `dat=SG9sYQ&id=${"a".repeat(21)}`, "SAF_03"],
  ["id-of-20-passes", `dat=SG9sYQ&id=${"a".repeat(20)}`, "SAF_06"],
  ["fileid-of-21-saf-03", `dat=SG9sYQ&fileid=${"a".repeat(21)}`, "SAF_03"],
  ["properties-malformed-passes", "dat=SG9sYQ&properties=esto-no-es-base64!", "SAF_06"],
  ["ksb64-malformed-passes", "dat=SG9sYQ&ksb64=esto-no-es-base64!", "SAF_06"],
];

const THE_PARAMETER_PROBES = THE_PARAMETER_CASES.map(([condition, parameters, expected]) =>
  aParameterRejection(
    condition,
    (idSession) =>
      `afirma://sign?op=sign&format=NoSuchFormat&algorithm=SHA256withRSA&${parameters}` +
      `&idsession=${idSession}`,
    expected,
  ),
);

/** Los que pasan el análisis y acaban en el formato inexistente, sin diálogo de por medio. */
const THE_PASSING_PARAMETER_PROBES = THE_PARAMETER_PROBES.filter(({ condition }) =>
  condition.endsWith("-passes"),
);

const THE_REFUSED_PARAMETER_PROBES = THE_PARAMETER_PROBES.filter(
  ({ condition }) => !condition.endsWith("-passes"),
);

/** Las operaciones por el canal v3, que toma la versión de su `ver`: pasar el control es llegar al formato. */
const THE_V3_OPERATION_PROBES = [
  {
    condition: "ver-4-passes-over-v3",
    order: (idSession) => aSignOrderStoppingAtTheFormat(idSession, { probed: "ver=4" }),
  },
  {
    condition: "ver-below-passes-over-v3",
    order: (idSession) => aSignOrderStoppingAtTheFormat(idSession, { probed: "ver=-10" }),
  },
  {
    condition: "ver-5-over-v3-saf-21",
    order: (idSession) => aSignOrderStoppingAtTheFormat(idSession, { probed: "ver=5" }),
    expected: "SAF_21",
  },
];

async function theProtocolV3Script() {
  const idSession = "sessionv3test";
  emit({
    event: "launch",
    url: `afirma://websocket?v=3&jvc=3&idsession=${idSession}`,
  });
  await new Promise((r) => setTimeout(r, 3000));
  let ws = null;
  try {
    ws = await connectWebSocket(theThirdProtocolPort());
  } catch {
    emit(
      aMeasuredConditionEvent(
        THE_FIXED_PORT_BOUND,
        null,
        `no se pudo conectar al puerto ${theThirdProtocolPort()}`,
      ),
    );
    settle({ event: "error" });
    return;
  }
  emit(
    aConditionEvent(
      THE_FIXED_PORT_BOUND,
      true,
      `conectado al puerto fijo ${theThirdProtocolPort()}`,
    ),
  );

  const echoResp = await exchange(ws, "echo=");
  emit(aConditionEvent(A_BARE_ECHO_ANSWERS_OK, echoResp === "OK", echoResp));

  const noIdResp = await exchange(ws, "echo=sin_idsession");
  emit(
    aConditionEvent(
      AN_ECHO_WITHOUT_A_SESSION_IS_NOT_REFUSED,
      !noIdResp.startsWith("SAF_46"),
      noIdResp,
    ),
  );

  try {
    const ws2 = await connectWebSocket(theThirdProtocolPort());
    ws2.close();
    await new Promise((r) => setTimeout(r, 200));
    const ping = await exchange(ws, "echo=");
    emit(
      aConditionEvent(
        A_SECOND_CLIENT_LEAVES_THE_CHANNEL_ALIVE,
        ping === "OK",
        ping === "OK" ? "el canal principal sigue activo" : "se cayó",
      ),
    );
  } catch (err) {
    emit(
      aMeasuredConditionEvent(A_SECOND_CLIENT_LEAVES_THE_CHANNEL_ALIVE, null, String(err?.message)),
    );
  }

  for (const { condition, order, expected = "SAF_06" } of THE_V3_OPERATION_PROBES) {
    const answer = await exchangeWithin(ws, order(idSession), THE_OPERATION_ANSWER_DEADLINE_MS);
    emit(
      answer === null
        ? aMeasuredConditionEvent(condition, null, "el sujeto no contestó a la operación")
        : aConditionEvent(condition, answer.startsWith(expected), answer),
    );
  }

  ws.close();
  settle({ event: "success" });
}

/** Lanza `url` y mira si alguno de `ports` llega a abrir canal en plazo: se cumple si ninguno. */
async function theChannelThatMustNotOpen(url, ports, condition) {
  emit({ event: "launch", url });
  const opened = await theChannelOpeningWithin(ports, THE_UNOPENED_CHANNEL_PATIENCE_MS);
  if (opened) opened.ws.close();
  emit(
    aConditionEvent(
      condition,
      opened === null,
      opened
        ? `${url}: abrió canal en ${opened.port}`
        : `${url}: ningún canal abierto en ${THE_UNOPENED_CHANNEL_PATIENCE_MS / 1000} s`,
    ),
  );
  settle({ event: "success" });
}

/** El primer candidato que abre canal antes de `patienceMs`, o `null` si ninguno. */
async function theChannelOpeningWithin(ports, patienceMs) {
  const deadline = Date.now() + patienceMs;
  while (Date.now() < deadline) {
    for (const port of ports) {
      try {
        return { ws: await connectWebSocket(port), port };
      } catch {}
    }
    await new Promise((resume) => setTimeout(resume, 500));
  }
  return null;
}

function theForeignSchemeScript() {
  const ports = [54391, 54392, 54393];
  return theChannelThatMustNotOpen(
    `other://websocket?ports=${ports.join(",")}&v=4&jvc=3&idsession=Fs5Ch3Me7Ot9Hr1Sc2Hm`,
    ports,
    NO_CHANNEL_OPENS,
  );
}

/** Un arranque v4 que pide la versión `version` del protocolo, que el canal no admite. */
function theUnsupportedVersionScript(version, ports) {
  return () =>
    theChannelThatMustNotOpen(
      `afirma://websocket?ports=${ports.join(",")}&v=${version}&jvc=3&idsession=Uv${version}Sp4Rt6Ed8Vr0Sn2`,
      ports,
      AN_UNSUPPORTED_VERSION_OPENS_NO_CHANNEL,
    );
}

/** Los puertos del arranque que la suite ocupa antes, para que el cliente no pueda ligar ninguno. */
const THE_OCCUPIED_PORTS = [54451, 54452, 54453];

function theOccupiedPortsScript() {
  return theChannelThatMustNotOpen(
    `afirma://websocket?ports=${THE_OCCUPIED_PORTS.join(",")}&v=4&jvc=3&idsession=Oc2Cu4Pi6Ed8Po0Rt1S`,
    THE_OCCUPIED_PORTS,
    NO_CHANNEL_OPENS,
  );
}

/** Un arranque con `jvc=0`, un cliente web anterior al mínimo: el canal tiene que abrirse igual. */
async function theOldJavascriptScript() {
  const idSession = "Jv0Ol2Dc4Li6En8Tt0Ab";
  const ports = [54471, 54472, 54473];
  emit({
    event: "launch",
    url: `afirma://websocket?ports=${ports.join(",")}&v=4&jvc=0&idsession=${idSession}`,
  });
  const opened = await theChannelOpeningWithin(ports, THE_WARNED_CHANNEL_PATIENCE_MS);
  const echo = opened
    ? await exchangeWithin(
        opened.ws,
        `echo=-idsession=${idSession}@EOF`,
        THE_OPERATION_ANSWER_DEADLINE_MS,
      )
    : null;
  opened?.ws.close();
  emit(
    aConditionEvent(
      THE_CHANNEL_OPENS_DESPITE_THE_WARNING,
      echo === "OK",
      opened
        ? `canal en ${opened.port}; el eco contestó ${echo ?? "nada"}`
        : `ningún canal abierto en ${THE_WARNED_CHANNEL_PATIENCE_MS / 1000} s`,
    ),
  );
  settle({ event: "success" });
}

/** Una selección del cliente publicado resuelta cuando contesta, con lo que contestó. */
function aPublishedSelection() {
  return new Promise((resolve) => {
    AutoScript.selectCertificate(
      "",
      (data) => resolve({ data: String(data) }),
      (type, message) => resolve({ type: String(type), message: String(message) }),
    );
  });
}

/** Dos selecciones seguidas del cliente publicado: la segunda viaja por el canal ya abierto. */
async function theTwoSelectionsOverOneChannelScript() {
  const first = await aPublishedSelection();
  const second = await aPublishedSelection();
  const launches = theLaunchesSoFar();
  const both = first.data !== undefined && second.data !== undefined;
  emit(
    aConditionEvent(
      THE_SECOND_OPERATION_REUSES_THE_CHANNEL,
      both && launches === 1,
      `${launches} invocaciones para dos selecciones; ` +
        `primera: ${first.data ? "certificado" : first.message}; segunda: ${second.data ? "certificado" : second.message}`,
    ),
  );
  settle(both ? { event: "success", data: second.data } : { event: "error", ...second });
}

/** El fichero que prepara el arnés `a_file_with_a_non_ascii_name`, con eñe y sin ASCII puro. */
const THE_NON_ASCII_FILE_NAME = "señal-año.bin";

/** Una carga del cliente publicado: el nombre del fichero elegido tiene que llegar en UTF-8. */
function theNonAsciiNameLoadScript() {
  AutoScript.getFileNameContentBase64(
    "Carga un documento",
    "bin",
    "Datos binarios",
    null,
    (filename, data) => {
      emit(
        aConditionEvent(
          THE_NAME_ARRIVES_IN_UTF8,
          String(filename).normalize("NFC") === THE_NON_ASCII_FILE_NAME,
          `llegó el nombre «${filename}»`,
        ),
      );
      settle({ event: "success", filename: String(filename), data: String(data) });
    },
    (type, message) => settle({ event: "error", type: String(type), message: String(message) }),
  );
}

const theConditionsOf = (probes) => probes.map(({ condition }) => condition);

/** Los rechazos de parámetros, con la operación inválida al final porque pide un certificado. */
const THE_REJECTIONS_IN_ORDER = [
  ...THE_V4_REJECTION_PROBES,
  ...THE_REFUSED_PARAMETER_PROBES,
  THE_INVALID_OPERATION_PROBE,
];

const onTheFourthProtocol = (run, conditions) =>
  aHandwrittenScript(run, { family: "v4-echo", modes: ["v4"], conditions });

export const WEBSOCKET_SCRIPTS = {
  "protocol-v4": onTheFourthProtocol(theProtocolV4Script, [
    A_CANDIDATE_PORT_BOUND,
    THE_FIRST_FREE_CANDIDATE_BOUND,
    THE_ECHO_WITH_ANOTHER_SESSION_ANSWERS_SAF_46,
    THE_ECHO_WITH_ITS_SESSION_ANSWERS_OK,
    THE_ECHO_WITHOUT_A_SESSION_ANSWERS_SAF_46,
    A_SECOND_CLIENT_LEAVES_THE_CHANNEL_ALIVE,
    ...theConditionsOf([...THE_V4_OPERATION_PROBES, ...THE_PASSING_PARAMETER_PROBES]),
  ]),
  "protocol-v4-rejections": onTheFourthProtocol(
    theProbesOverTheFourthProtocol(
      [54341, 54342, 54343],
      THE_REJECTIONS_IN_ORDER,
      THE_DIALOGUE_PATIENCE_MS,
    ),
    theConditionsOf(THE_REJECTIONS_IN_ORDER),
  ),
  "protocol-v4-local-access": onTheFourthProtocol(
    theProbesOverTheFourthProtocol(
      [54451, 54452, 54453],
      [THE_LOCAL_RTSERVLET_PROBE],
      THE_DIALOGUE_PATIENCE_MS,
    ),
    theConditionsOf([THE_LOCAL_RTSERVLET_PROBE]),
  ),
  "protocol-v4-shown-rejection": onTheFourthProtocol(theShownRejectionScript, [
    THE_REJECTION_WAITED_FOR_THE_DIALOGUE,
  ]),
  "protocol-v4-keystore-over-ksb64": onTheFourthProtocol(theKeyStoreOverKsb64Script, [
    THE_SELECTION_ANSWERS_A_CERTIFICATE,
  ]),
  loadnonascii: aPublishedScript(theNonAsciiNameLoadScript, {
    modes: ["v4", "v3", "service"],
    conditions: [THE_NAME_ARRIVES_IN_UTF8],
  }),
  selectcerttwice: aPublishedScript(theTwoSelectionsOverOneChannelScript, {
    modes: ["v4", "v3"],
    conditions: [THE_SECOND_OPERATION_REUSES_THE_CHANNEL],
  }),
  "protocol-v4-malformed-id": onTheFourthProtocol(theProtocolV4MalformedIdScript, [
    A_MALFORMED_SESSION_BINDS_NOTHING,
  ]),
  "protocol-foreign-scheme": onTheFourthProtocol(theForeignSchemeScript, [NO_CHANNEL_OPENS]),
  "protocol-v4-version-1": onTheFourthProtocol(
    theUnsupportedVersionScript(1, [54431, 54432, 54433]),
    [AN_UNSUPPORTED_VERSION_OPENS_NO_CHANNEL],
  ),
  "protocol-v4-version-99": onTheFourthProtocol(
    theUnsupportedVersionScript(99, [54441, 54442, 54443]),
    [AN_UNSUPPORTED_VERSION_OPENS_NO_CHANNEL],
  ),
  "protocol-v4-occupied-ports": onTheFourthProtocol(theOccupiedPortsScript, [NO_CHANNEL_OPENS]),
  "protocol-v4-old-javascript": onTheFourthProtocol(theOldJavascriptScript, [
    THE_CHANNEL_OPENS_DESPITE_THE_WARNING,
  ]),
  "protocol-v3": aHandwrittenScript(theProtocolV3Script, {
    family: "v4-echo",
    modes: ["v3"],
    conditions: [
      THE_FIXED_PORT_BOUND,
      A_BARE_ECHO_ANSWERS_OK,
      AN_ECHO_WITHOUT_A_SESSION_IS_NOT_REFUSED,
      A_SECOND_CLIENT_LEAVES_THE_CHANNEL_ALIVE,
      ...THE_V3_OPERATION_PROBES.map(({ condition }) => condition),
    ],
  }),
};
