// Los guiones de la sede a mano por el canal WebSocket: el protocolo escrito en crudo.

import { aConditionEvent, aMeasuredConditionEvent, emit, settle } from "../lib/events.mjs";
import { theThirdProtocolPort } from "../lib/modes.mjs";
import { aHandwrittenScript } from "../lib/script.mjs";

const A_CANDIDATE_PORT_BOUND = "a-candidate-port-bound";
const THE_ECHO_WITH_ITS_SESSION_ANSWERS_OK = "the-echo-with-its-session-answers-ok";
const THE_ECHO_WITHOUT_A_SESSION_ANSWERS_SAF_46 = "the-echo-without-a-session-answers-saf-46";
const A_SECOND_CLIENT_LEAVES_THE_CHANNEL_ALIVE = "a-second-client-leaves-the-channel-alive";
const A_MALFORMED_SESSION_BINDS_NOTHING = "a-malformed-session-binds-nothing";
const THE_FIXED_PORT_BOUND = "the-fixed-port-bound";
const A_BARE_ECHO_ANSWERS_OK = "a-bare-echo-answers-ok";
const AN_ECHO_WITHOUT_A_SESSION_IS_NOT_REFUSED = "an-echo-without-a-session-is-not-refused";
const NO_CHANNEL_OPENS = "no-channel-opens";

/** Lo que se espera a que un esquema ajeno abra canal antes de darlo por no abierto. */
const THE_FOREIGN_SCHEME_PATIENCE_MS = 10000;

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

/** Lanza el sujeto en v4 y abre el canal en el primer puerto candidato que conteste. */
async function theProtocolV4ChannelOpening(ports, idSession) {
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
  const channel = await theProtocolV4ChannelOpening([54321, 54322, 54323], idSession);
  if (!channel) return;
  const { ws: ws1, port: connectedPort } = channel;
  emit(aConditionEvent(A_CANDIDATE_PORT_BOUND, true, `conectado en puerto ${connectedPort}`));

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

  ws1.close();
  settle({ event: "success" });
}

/** Una firma que pasa el análisis de parámetros y se para en el formato inventado (`SAF_06`). */
function aSignOrderStoppingAtTheFormat(idSession, { op = "sign", probed } = {}) {
  const data = Buffer.from("rfirma").toString("base64");
  const extra = probed ? `&${probed}` : "";
  return `afirma://sign?op=${op}&format=INVENTADO&algorithm=SHA256&dat=${data}${extra}&idsession=${idSession}`;
}

/** Las operaciones que se mandan por el canal v4 y lo que tiene que cumplir cada respuesta. */
const THE_V4_OPERATION_PROBES = [
  {
    condition: "ver-4-passes",
    order: (idSession) => aSignOrderStoppingAtTheFormat(idSession, { probed: "ver=4" }),
    holds: (answer) => !answer.startsWith("SAF_21"),
  },
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
    condition: "unknown-operation-saf-04",
    order: (idSession) => `afirma://unknownop?idsession=${idSession}`,
    holds: (answer) => answer.startsWith("SAF_04"),
  },
  {
    condition: "invalid-op-saf-04",
    order: (idSession) => aSignOrderStoppingAtTheFormat(idSession, { op: "invalid" }),
    holds: (answer) => answer.startsWith("SAF_04"),
  },
  {
    condition: "invented-format-saf-06",
    order: (idSession) => aSignOrderStoppingAtTheFormat(idSession),
    holds: (answer) => answer.startsWith("SAF_06"),
  },
  {
    condition: "local-rtservlet-saf-13",
    order: (idSession) =>
      `afirma://sign?op=sign&fileid=rfirma&rtservlet=http://127.0.0.1/rt&idsession=${idSession}`,
    holds: (answer) => answer.startsWith("SAF_13"),
  },
  {
    condition: "no-data-saf-03",
    order: (idSession) =>
      `afirma://sign?op=sign&id=rfirma-1&format=CAdES&algorithm=SHA256&idsession=${idSession}`,
    holds: (answer) => answer.startsWith("SAF_03"),
  },
];

async function theProtocolV4OperationsScript() {
  const idSession = "Op4Rt6Yu8Io0Pa2Sd4Fg";
  const channel = await theProtocolV4ChannelOpening([54381, 54382, 54383], idSession);
  if (!channel) return;
  let silentAt = null;
  for (const { condition, order, holds } of THE_V4_OPERATION_PROBES) {
    const answer = silentAt
      ? null
      : await exchangeWithin(channel.ws, order(idSession), THE_OPERATION_ANSWER_DEADLINE_MS);
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
  channel.ws.close();
  settle({ event: "success" });
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
  ["ver-below-passes", "dat=SG9sYQ&ver=-10", "SAF_06"],
  ["mcv-malformed-saf-03", "dat=SG9sYQ&mcv=uno.dos", "SAF_03"],
  ["id-of-21-saf-03", `dat=SG9sYQ&id=${"a".repeat(21)}`, "SAF_03"],
  ["id-of-20-passes", `dat=SG9sYQ&id=${"a".repeat(20)}`, "SAF_06"],
  ["fileid-of-21-saf-03", `dat=SG9sYQ&fileid=${"a".repeat(21)}`, "SAF_03"],
  ["properties-malformed-passes", "dat=SG9sYQ&properties=esto-no-es-base64!", "SAF_06"],
  ["ksb64-malformed-passes", "dat=SG9sYQ&ksb64=esto-no-es-base64!", "SAF_06"],
];

async function theProtocolV4ParametersScript() {
  const ports = [54341, 54342, 54343];
  const idSession = "Pq7Rs2Tu9Vw4Xy1Za6Bc";
  emit({
    event: "launch",
    url: `afirma://websocket?ports=${ports.join(",")}&v=4&jvc=3&idsession=${idSession}`,
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
    emit({
      event: "error",
      type: "cannot_connect",
      message: "no se pudo conectar a los puertos candidatos",
    });
    settle({ event: "error" });
    return;
  }
  await exchange(ws, `echo=-idsession=${idSession}@EOF`);

  for (const [condition, parameters, expected] of THE_PARAMETER_CASES) {
    const answer = await exchange(
      ws,
      `afirma://sign?op=sign&format=NoSuchFormat&algorithm=SHA256withRSA&${parameters}` +
        `&idsession=${idSession}`,
    );
    emit(aConditionEvent(condition, answer.startsWith(expected), answer));
  }

  ws.close();
  settle({ event: "success" });
}

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

  ws.close();
  settle({ event: "success" });
}

/** Lanza con un esquema ajeno a `afirma://` y mira si algún puerto candidato llega a abrir canal. */
async function theForeignSchemeScript() {
  const ports = [54391, 54392, 54393];
  emit({
    event: "launch",
    url: `other://websocket?ports=${ports.join(",")}&v=4&jvc=3&idsession=Fs5Ch3Me7Ot9Hr1Sc2Hm`,
  });
  const deadline = Date.now() + THE_FOREIGN_SCHEME_PATIENCE_MS;
  while (Date.now() < deadline) {
    for (const port of ports) {
      try {
        const ws = await connectWebSocket(port);
        ws.close();
        emit(aConditionEvent(NO_CHANNEL_OPENS, false, `el esquema ajeno abrió canal en ${port}`));
        settle({ event: "success" });
        return;
      } catch {}
    }
    await new Promise((resume) => setTimeout(resume, 500));
  }
  emit(
    aConditionEvent(
      NO_CHANNEL_OPENS,
      true,
      `ningún canal abierto en ${THE_FOREIGN_SCHEME_PATIENCE_MS / 1000} s`,
    ),
  );
  settle({ event: "success" });
}

const onTheFourthProtocol = (run, conditions) =>
  aHandwrittenScript(run, { family: "v4-echo", modes: ["v4"], conditions });

export const WEBSOCKET_SCRIPTS = {
  "protocol-v4": onTheFourthProtocol(theProtocolV4Script, [
    A_CANDIDATE_PORT_BOUND,
    THE_ECHO_WITH_ITS_SESSION_ANSWERS_OK,
    THE_ECHO_WITHOUT_A_SESSION_ANSWERS_SAF_46,
    A_SECOND_CLIENT_LEAVES_THE_CHANNEL_ALIVE,
  ]),
  "protocol-v4-operations": onTheFourthProtocol(
    theProtocolV4OperationsScript,
    THE_V4_OPERATION_PROBES.map(({ condition }) => condition),
  ),
  "protocol-v4-parameters": onTheFourthProtocol(
    theProtocolV4ParametersScript,
    THE_PARAMETER_CASES.map(([condition]) => condition),
  ),
  "protocol-v4-malformed-id": onTheFourthProtocol(theProtocolV4MalformedIdScript, [
    A_MALFORMED_SESSION_BINDS_NOTHING,
  ]),
  "protocol-foreign-scheme": onTheFourthProtocol(theForeignSchemeScript, [NO_CHANNEL_OPENS]),
  "protocol-v3": aHandwrittenScript(theProtocolV3Script, {
    family: "v4-echo",
    modes: ["v3"],
    conditions: [
      THE_FIXED_PORT_BOUND,
      A_BARE_ECHO_ANSWERS_OK,
      AN_ECHO_WITHOUT_A_SESSION_IS_NOT_REFUSED,
      A_SECOND_CLIENT_LEAVES_THE_CHANNEL_ALIVE,
    ],
  }),
};
