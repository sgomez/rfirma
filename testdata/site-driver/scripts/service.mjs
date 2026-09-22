// Los guiones de la sede a mano por el canal del socket, el transporte sin WebSocket.

import { createServer as createTcpServer } from "node:net";
import { networkInterfaces } from "node:os";
import { connect as connectTls } from "node:tls";

import { aConditionEvent, aMeasuredConditionEvent, bytesOf, emit, settle } from "../lib/events.mjs";
import { aHandwrittenScript } from "../lib/script.mjs";

const THE_ECHO_ANSWERS_OK = "the-echo-answers-ok";
const THE_FIRST_FREE_CANDIDATE_BOUND = "the-first-free-candidate-bound";
const READ_UNTIL_THE_EOF_MARK = "read-until-the-eof-mark";
const CMD_ANNOUNCES_THE_PARTS = "cmd-announces-the-parts";
const SEND_DELIVERS_THE_RESULT = "send-delivers-the-result";
const AN_INNER_VER_IS_IGNORED = "an-inner-ver-is-ignored";
const A_DASHED_ECHO_DISCARDS_THE_RESULT = "a-dashed-echo-discards-the-result";
const AN_EARLY_FRAGMENT_ASKS_FOR_MORE = "an-early-fragment-asks-for-more";
const THE_LAST_FRAGMENT_ANSWERS_OK = "the-last-fragment-answers-ok";
const FIRM_RUNS_THE_REASSEMBLED_REQUEST = "firm-runs-the-reassembled-request";
const EVERY_ANSWER_IS_HTTP_200 = "every-answer-is-http-200";
const ONLY_THE_LOOPBACK_SERVED = "only-the-loopback-served";
const NO_CHANNEL_OPENS = "no-channel-opens";
const A_FOREIGN_SESSION_ANSWERS_SAF_03 = "a-foreign-session-answers-saf-03";
const EVERY_ANSWER_ALLOWS_ANY_ORIGIN = "every-answer-allows-any-origin";
const AN_UNKNOWN_ORDER_ANSWERS_SAF_03 = "an-unknown-order-answers-saf-03";
const A_CMD_THAT_IS_NO_OPERATION_ANSWERS_SAF_11 = "a-cmd-that-is-no-operation-answers-saf-11";
const A_SEND_PART_OUT_OF_RANGE_ANSWERS_SAF_11 = "a-send-part-out-of-range-answers-saf-11";
const A_FAILED_SAVE_ANSWERS_SAF_11 = "a-failed-save-answers-saf-11";
const CLOSED_NINETY_SECONDS_AFTER_THE_LAST_VALID_ORDER =
  "closed-ninety-seconds-after-the-last-valid-order";

/** Cuándo, tras la última orden válida, se manda la inválida: dentro de los 90 s, con el canal aún abierto. */
const THE_INVALID_ORDER_AT_MS = 80000;

/** Cuándo, tras la última orden válida, se comprueba que el canal ya se cerró. */
const THE_CLOSURE_CHECKED_AT_MS = 100000;

/** Lo que se espera a que el canal del socket conteste una orden antes de darla por perdida. */
const THE_SERVICE_ORDER_PATIENCE_MS = 10000;

/** Lo que se espera a un guardado rechazado mientras la persona cierra el diálogo de error. */
const THE_DIALOGUE_PATIENCE_MS = 120000;

/** Lo que se espera a que el sujeto ligue su socket después de invocarlo. */
const THE_SERVICE_START_PATIENCE_MS = 30000;

/** Una operación que el canal atiende sin pedir certificado: el formato inexistente da SAF_06. */
const AN_OPERATION_REFUSED_BY_ITS_FORMAT =
  "afirma://sign?op=sign&format=NoSuchFormat&algorithm=SHA256withRSA&dat=SG9sYQ";

/** Otra que tampoco pide certificado y se rechaza por otro motivo: un fichero local en `dat`. */
const AN_OPERATION_REFUSED_BY_ITS_LOCAL_FILE =
  "afirma://sign?op=sign&format=NoSuchFormat&algorithm=SHA256withRSA&dat=file:/etc/hostname";

/** El Base64 URL-safe con relleno con el que viaja una URL dentro de `cmd=` y de `fragment=`. */
function asServiceBase64(text) {
  return Buffer.from(text, "utf8").toString("base64").replace(/\+/g, "-").replace(/\//g, "_");
}

/** La orden envuelta como la manda el cliente publicado: un `POST` a `/afirma`. */
function asServicePost(port, body) {
  return (
    `POST /afirma HTTP/1.1\r\nHost: 127.0.0.1:${port}\r\n` +
    "Content-Type: application/x-www-form-urlencoded\r\n" +
    `Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`
  );
}

function anEcho(idSession) {
  return idSession ? `echo=-idsession=${idSession}@EOF` : "echo=-@EOF";
}

/** La respuesta cruda del canal: su línea de estado, sus cabeceras y su cuerpo ya descodificado. */
function aServiceAnswer(raw) {
  const text = raw.toString("utf8");
  const blankLine = /\r?\n\r?\n/.exec(text);
  const head = blankLine ? text.slice(0, blankLine.index) : text;
  const body = blankLine ? text.slice(blankLine.index + blankLine[0].length).trim() : "";
  const [status, ...headerLines] = head.split(/\r?\n/);
  const headers = Object.fromEntries(
    headerLines
      .filter((line) => line.includes(":"))
      .map((line) => [
        line.slice(0, line.indexOf(":")).trim().toLowerCase(),
        line.slice(line.indexOf(":") + 1).trim(),
      ]),
  );
  return { status: status.trim(), headers, text: bytesOf(body).toString("utf8") };
}

/** Una conversación por TLS: escribe `pieces` con `pauseMs` y recoge la respuesta hasta el cierre. */
function talkingToTheService({
  host = "127.0.0.1",
  port,
  pieces,
  pauseMs = 0,
  trusted = true,
  patienceMs = THE_SERVICE_ORDER_PATIENCE_MS,
}) {
  return new Promise((resolve) => {
    const chunks = [];
    let written = 0;
    let answeredEarly = false;
    let over = false;
    const socket = connectTls({ host, port, rejectUnauthorized: trusted });
    const finish = (failure) => {
      if (over) return;
      over = true;
      clearTimeout(patience);
      socket.destroy();
      const raw = Buffer.concat(chunks);
      const answer =
        raw.length > 0 ? aServiceAnswer(raw) : { status: null, headers: {}, text: null };
      resolve({ ...answer, failure, answeredEarly });
    };
    const patience = setTimeout(() => finish("silencio"), patienceMs);
    socket.on("secureConnect", async () => {
      for (const piece of pieces) {
        if (written > 0) await new Promise((resume) => setTimeout(resume, pauseMs));
        if (over) return;
        socket.write(piece);
        written += 1;
      }
    });
    socket.on("data", (chunk) => {
      if (written < pieces.length) answeredEarly = true;
      chunks.push(chunk);
    });
    socket.on("end", () => finish(null));
    socket.on("close", () => finish(null));
    socket.on("error", (error) => finish(error.code ?? String(error.message)));
  });
}

function aServiceOrder(port, body, patienceMs) {
  return talkingToTheService({ port, pieces: [asServicePost(port, body)], patienceMs });
}

/** El primer candidato que atiende el eco, en el orden ofrecido, o `null` si ninguno en plazo. */
async function theServiceChannelOpening(ports, idSession) {
  const deadline = Date.now() + THE_SERVICE_START_PATIENCE_MS;
  while (Date.now() < deadline) {
    for (const port of ports) {
      const answer = await aServiceOrder(port, anEcho(idSession));
      if (answer.text !== null) return { port, answer };
    }
    await new Promise((resume) => setTimeout(resume, 500));
  }
  return null;
}

/** Un puerto ocupado que cuelga cada conexión que le llega: uno mudo colgaría al cliente. */
function anOccupiedPortThatHangsUp(port) {
  return new Promise((resolve, reject) => {
    const server = createTcpServer((socket) => socket.destroy());
    server.once("error", reject);
    server.listen(port, "0.0.0.0", () => resolve(server));
  });
}

function aHostAddressOutsideTheLoopback() {
  return (
    Object.values(networkInterfaces())
      .flat()
      .find((address) => address?.family === "IPv4" && !address.internal)?.address ?? null
  );
}

/** Las partes que anunció `announced`, pedidas una a una con `send=` y unidas. */
async function theServiceResult(order, idSession, announced) {
  const parts = Number.parseInt(announced.text ?? "", 10);
  if (!(parts >= 1)) return { parts: null, result: null };
  let result = "";
  for (let part = 1; part <= parts; part++) {
    const sent = await order(`send=@${part}@${parts}idsession=${idSession}@EOF`);
    result += sent.text ?? "";
  }
  return { parts, result };
}

async function theServiceOperation(order, idSession, uri) {
  const announced = await order(`cmd=${asServiceBase64(uri)}idsession=${idSession}@EOF`);
  return { announced, ...(await theServiceResult(order, idSession, announced)) };
}

/** El canal del socket en crudo, con el primer candidato ocupado por un obstáculo que cuelga. */
async function theServiceProtocolScript() {
  const ports = [54351, 54352, 54353];
  const idSession = "Sv3c4Ch9Lm2Np7Qr5Tw1";
  const obstacle = await anOccupiedPortThatHangsUp(ports[0]).catch(() => null);
  emit({
    event: "launch",
    url: `afirma://service?ports=${ports.join(",")}&v=3&jvc=3&idsession=${idSession}`,
  });
  const opened = await theServiceChannelOpening(ports, idSession);
  obstacle?.close();
  if (!opened) {
    settle({
      event: "error",
      type: "cannot_connect",
      message: "ningún puerto candidato contestó al eco",
    });
    return;
  }
  const { port } = opened;
  const statusLines = [opened.answer.status];
  const origins = [opened.answer.headers["access-control-allow-origin"]];
  const order = async (body) => {
    const answer = await aServiceOrder(port, body);
    if (answer.status !== null) {
      statusLines.push(answer.status);
      origins.push(answer.headers["access-control-allow-origin"]);
    }
    return answer;
  };

  emit(
    aMeasuredConditionEvent(
      THE_FIRST_FREE_CANDIDATE_BOUND,
      obstacle ? port === ports[1] : null,
      obstacle
        ? `con ${ports[0]} ocupado, abrió en ${port}`
        : `no se pudo ocupar ${ports[0]} para medirlo`,
    ),
  );
  emit(
    aConditionEvent(THE_ECHO_ANSWERS_OK, opened.answer.text === "OK", `v=3: ${opened.answer.text}`),
  );

  const post = asServicePost(port, anEcho(idSession));
  const eof = post.length - "@EOF".length;
  const streamed = await talkingToTheService({
    port,
    pieces: [post.slice(0, eof + 2), post.slice(eof + 2)],
    pauseMs: 1000,
  });
  if (streamed.status !== null) statusLines.push(streamed.status);
  emit(
    aConditionEvent(
      READ_UNTIL_THE_EOF_MARK,
      !streamed.answeredEarly && streamed.text === "OK",
      streamed.answeredEarly
        ? "contestó antes de que llegara @EOF"
        : (streamed.text ?? streamed.failure),
    ),
  );

  const first = await theServiceOperation(
    order,
    idSession,
    `${AN_OPERATION_REFUSED_BY_ITS_FORMAT}&ver=5`,
  );
  emit(
    aConditionEvent(
      CMD_ANNOUNCES_THE_PARTS,
      first.parts !== null,
      first.announced.text ?? first.announced.failure,
    ),
  );
  emit(
    aMeasuredConditionEvent(
      SEND_DELIVERS_THE_RESULT,
      first.parts === null ? null : first.result.startsWith("SAF_"),
      first.result ?? "sin partes que pedir",
    ),
  );
  emit(
    aMeasuredConditionEvent(
      AN_INNER_VER_IS_IGNORED,
      first.parts === null ? null : !first.result.startsWith("SAF_21"),
      first.result ?? "sin partes que pedir",
    ),
  );

  await order(anEcho(idSession));
  const second = await theServiceOperation(
    order,
    idSession,
    AN_OPERATION_REFUSED_BY_ITS_LOCAL_FILE,
  );
  emit(
    aMeasuredConditionEvent(
      A_DASHED_ECHO_DISCARDS_THE_RESULT,
      first.parts === null || second.parts === null ? null : second.result !== first.result,
      `antes: ${first.result ?? "-"}; después: ${second.result ?? "-"}`,
    ),
  );

  await order(anEcho(idSession));
  const half = Math.ceil(AN_OPERATION_REFUSED_BY_ITS_FORMAT.length / 2);
  const firstFragment = await order(
    `fragment=@1@2@${asServiceBase64(AN_OPERATION_REFUSED_BY_ITS_FORMAT.slice(0, half))}` +
      `idsession=${idSession}@EOF`,
  );
  const lastFragment = await order(
    `fragment=@2@2@${asServiceBase64(AN_OPERATION_REFUSED_BY_ITS_FORMAT.slice(half))}` +
      `idsession=${idSession}@EOF`,
  );
  emit(
    aConditionEvent(
      AN_EARLY_FRAGMENT_ASKS_FOR_MORE,
      firstFragment.text === "MORE_DATA_NEED",
      firstFragment.text ?? firstFragment.failure,
    ),
  );
  emit(
    aConditionEvent(
      THE_LAST_FRAGMENT_ANSWERS_OK,
      lastFragment.text === "OK",
      lastFragment.text ?? lastFragment.failure,
    ),
  );
  const fired = await order(`firm=idsession=${idSession}@EOF`);
  const reassembled = await theServiceResult(order, idSession, fired);
  emit(
    aConditionEvent(
      FIRM_RUNS_THE_REASSEMBLED_REQUEST,
      reassembled.result?.startsWith("SAF_06") ?? false,
      reassembled.result ?? fired.text ?? fired.failure,
    ),
  );

  const foreign = await order("echo=-idsession=OtraSesionAjena00000@EOF");
  emit(anAnswerCondition(A_FOREIGN_SESSION_ANSWERS_SAF_03, foreign, "SAF_03"));
  const unknown = await order(`nada=idsession=${idSession}@EOF`);
  emit(anAnswerCondition(AN_UNKNOWN_ORDER_ANSWERS_SAF_03, unknown, "SAF_03"));
  const notAnOperation = [];
  for (const uri of ["afirma://service?ports=54351&v=3", "https://sede.example/tramite"]) {
    notAnOperation.push(await order(`cmd=${asServiceBase64(uri)}idsession=${idSession}@EOF`));
  }
  emit(anAnswersCondition(A_CMD_THAT_IS_NO_OPERATION_ANSWERS_SAF_11, notAnOperation, "SAF_11"));
  await order(anEcho(idSession));
  const outOfRange = await order(`send=@3@1idsession=${idSession}@EOF`);
  emit(anAnswerCondition(A_SEND_PART_OUT_OF_RANGE_ANSWERS_SAF_11, outOfRange, "SAF_11"));
  const withoutAnyOrigin = origins.filter((origin) => origin !== "*");
  emit(
    aConditionEvent(
      EVERY_ANSWER_ALLOWS_ANY_ORIGIN,
      withoutAnyOrigin.length === 0,
      withoutAnyOrigin.length === 0
        ? `${origins.length} respuestas, todas con Access-Control-Allow-Origin: *`
        : `${withoutAnyOrigin.length} respuestas sin Access-Control-Allow-Origin: *`,
    ),
  );
  const otherStatuses = statusLines.filter((line) => line !== "HTTP/1.1 200 OK");
  emit(
    aConditionEvent(
      EVERY_ANSWER_IS_HTTP_200,
      otherStatuses.length === 0,
      otherStatuses.length === 0
        ? `${statusLines.length} respuestas, todas HTTP/1.1 200 OK`
        : `también: ${[...new Set(otherStatuses)].join(", ")}`,
    ),
  );

  const outside = aHostAddressOutsideTheLoopback();
  if (outside === null) {
    emit(
      aMeasuredConditionEvent(
        ONLY_THE_LOOPBACK_SERVED,
        null,
        "el equipo no tiene ninguna dirección fuera del bucle local",
      ),
    );
  } else {
    const fromOutside = await talkingToTheService({
      host: outside,
      port,
      pieces: [asServicePost(port, anEcho(idSession))],
      trusted: false,
    });
    emit(
      aConditionEvent(
        ONLY_THE_LOOPBACK_SERVED,
        fromOutside.text !== "OK",
        `desde fuera del bucle local: ${fromOutside.text ?? fromOutside.failure ?? "cerró sin contestar"}`,
      ),
    );
  }

  settle({ event: "success" });
}

function anAnswersCondition(name, answers, expected) {
  return aConditionEvent(
    name,
    answers.every((answer) => answer.text?.startsWith(expected) ?? false),
    answers.map((answer) => answer.text ?? answer.failure).join("; "),
  );
}

function anAnswerCondition(name, answer, expected) {
  return anAnswersCondition(name, [answer], expected);
}

/** Un guardado sin datos por `cmd=`: en AutoFirma, un diálogo de error antes de la respuesta. */
async function theFailedSaveScript() {
  const idSession = "Fs7Sv9Er1Dl3Gq5Tt7Ab";
  const ports = [54481, 54482, 54483];
  emit({ event: "launch", url: aServiceLaunch({ ports, version: 3, idSession }) });
  const opened = await theServiceChannelOpening(ports, idSession);
  if (!opened) {
    emit(aMeasuredConditionEvent(A_FAILED_SAVE_ANSWERS_SAF_11, null, "el canal no se abrió"));
    settle({ event: "success" });
    return;
  }
  const save = "afirma://save?op=save&filename=rfirma.txt&exts=txt";
  const answer = await aServiceOrder(
    opened.port,
    `cmd=${asServiceBase64(save)}idsession=${idSession}@EOF`,
    THE_DIALOGUE_PATIENCE_MS,
  );
  emit(anAnswerCondition(A_FAILED_SAVE_ANSWERS_SAF_11, answer, "SAF_11"));
  settle({ event: "success" });
}

/** Una invocación que solo se mide abriendo el canal: si el eco contesta OK en algún candidato. */
async function theServiceLaunchVariantScript({ ports, launch }) {
  const idSession = "Vr2Sl4Ng6Pt8Xz0Ab3Cd";
  const url = launch(ports, idSession);
  emit({ event: "launch", url });
  const opened = await theServiceChannelOpening(ports, idSession);
  const observation = opened
    ? `${url}: el eco contestó ${opened.answer.text} en ${opened.port}`
    : `${url}: ningún puerto candidato contestó al eco`;
  emit(aConditionEvent(THE_ECHO_ANSWERS_OK, opened?.answer.text === "OK", observation));
  settle({ event: "success" });
}

/** Un arranque que no debe abrir canal: se cumple si ningún candidato contesta al eco en plazo. */
async function theRefusedLaunchScript({ ports, launch }) {
  const idSession = "Rf4Sd6Ln8Ch0Xz2Ab4Cd";
  const url = launch(ports, idSession);
  emit({ event: "launch", url });
  const opened = await theServiceChannelOpening(ports, idSession);
  emit(
    aConditionEvent(
      NO_CHANNEL_OPENS,
      opened === null,
      opened
        ? `${url}: el eco contestó ${opened.answer.text} en ${opened.port}`
        : `${url}: ningún puerto candidato contestó al eco`,
    ),
  );
  settle({ event: "success" });
}

const aMomentAfter = (start, ms) =>
  new Promise((resume) => setTimeout(resume, Math.max(0, start + ms - Date.now())));

/** El canal abierto, una orden inválida a los 80 s del último eco y otro eco a los 100 s. */
async function theInactivityScript() {
  const idSession = "In4Ac6Tv8Ty0Xz2Ab4Cd";
  const ports = [54461, 54462, 54463];
  emit({ event: "launch", url: aServiceLaunch({ ports, version: 3, idSession }) });
  const opened = await theServiceChannelOpening(ports, idSession);
  if (!opened) {
    emit(
      aMeasuredConditionEvent(
        CLOSED_NINETY_SECONDS_AFTER_THE_LAST_VALID_ORDER,
        null,
        "ningún puerto candidato contestó al eco",
      ),
    );
    settle({ event: "success" });
    return;
  }
  const lastValidOrder = Date.now();
  await aMomentAfter(lastValidOrder, THE_INVALID_ORDER_AT_MS);
  const invalid = await aServiceOrder(opened.port, `nada=idsession=${idSession}@EOF`);
  await aMomentAfter(lastValidOrder, THE_CLOSURE_CHECKED_AT_MS);
  const late = await aServiceOrder(opened.port, anEcho(idSession));
  const aliveAtTheInvalid = invalid.text !== null;
  const closedAfter = late.text === null;
  emit(
    aConditionEvent(
      CLOSED_NINETY_SECONDS_AFTER_THE_LAST_VALID_ORDER,
      aliveAtTheInvalid && closedAfter,
      `a los ${THE_INVALID_ORDER_AT_MS / 1000} s la orden inválida contestó ${invalid.text ?? invalid.failure}; ` +
        `a los ${THE_CLOSURE_CHECKED_AT_MS / 1000} s el eco contestó ${late.text ?? late.failure}`,
    ),
  );
  settle({ event: "success" });
}

const overTheService = (run, conditions) =>
  aHandwrittenScript(run, { family: "service", modes: ["service"], conditions });

/** Una invocación del canal que cambia una sola cosa respecto de la de siempre. */
function aServiceLaunch({ ports, version, idSession, slash = false }) {
  return (
    `afirma://service${slash ? "/" : ""}?ports=${ports.join(",")}&v=${version}&jvc=3` +
    `&idsession=${idSession}`
  );
}

const aLaunchVariant = (ports, launch) =>
  overTheService(() => theServiceLaunchVariantScript({ ports, launch }), [THE_ECHO_ANSWERS_OK]);

export const SERVICE_SCRIPTS = {
  "protocol-service": overTheService(theServiceProtocolScript, [
    THE_ECHO_ANSWERS_OK,
    THE_FIRST_FREE_CANDIDATE_BOUND,
    READ_UNTIL_THE_EOF_MARK,
    CMD_ANNOUNCES_THE_PARTS,
    SEND_DELIVERS_THE_RESULT,
    AN_INNER_VER_IS_IGNORED,
    A_DASHED_ECHO_DISCARDS_THE_RESULT,
    AN_EARLY_FRAGMENT_ASKS_FOR_MORE,
    THE_LAST_FRAGMENT_ANSWERS_OK,
    FIRM_RUNS_THE_REASSEMBLED_REQUEST,
    EVERY_ANSWER_IS_HTTP_200,
    ONLY_THE_LOOPBACK_SERVED,
    A_FOREIGN_SESSION_ANSWERS_SAF_03,
    AN_UNKNOWN_ORDER_ANSWERS_SAF_03,
    A_CMD_THAT_IS_NO_OPERATION_ANSWERS_SAF_11,
    A_SEND_PART_OUT_OF_RANGE_ANSWERS_SAF_11,
    EVERY_ANSWER_ALLOWS_ANY_ORIGIN,
  ]),
  "protocol-service-failed-save": overTheService(theFailedSaveScript, [
    A_FAILED_SAVE_ANSWERS_SAF_11,
  ]),
  "protocol-service-v1": aLaunchVariant([54361, 54362, 54363], (ports, idSession) =>
    aServiceLaunch({ ports, version: 1, idSession }),
  ),
  "protocol-service-v2": aLaunchVariant([54371, 54372, 54373], (ports, idSession) =>
    aServiceLaunch({ ports, version: 2, idSession }),
  ),
  "protocol-service-negative-ports": aLaunchVariant([54381, 54382, 54383], (ports, idSession) =>
    aServiceLaunch({ ports: ports.map((port) => -port), version: 3, idSession }),
  ),
  "protocol-service-v4": overTheService(
    () =>
      theRefusedLaunchScript({
        ports: [54421, 54422, 54423],
        launch: (ports, idSession) => aServiceLaunch({ ports, version: 4, idSession }),
      }),
    [NO_CHANNEL_OPENS],
  ),
  "protocol-service-inactivity": overTheService(theInactivityScript, [
    CLOSED_NINETY_SECONDS_AFTER_THE_LAST_VALID_ORDER,
  ]),
  "protocol-service-slash": aLaunchVariant([54411, 54412, 54413], (ports, idSession) =>
    aServiceLaunch({ ports, version: 3, idSession, slash: true }),
  ),
};
