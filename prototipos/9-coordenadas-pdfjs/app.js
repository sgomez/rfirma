// PROTOTIPO desechable del ticket #9. Sin abstracciones a proposito.
import * as pdfjs from './vendor/pdf.min.mjs';
pdfjs.GlobalWorkerOptions.workerSrc = './vendor/pdf.worker.min.mjs';

const CASOS = ['a4', 'a4-rot90', 'a4-rot180', 'a4-rot270', 'a5', 'letter',
               'offset', 'offset-rot90', 'offset-rot180', 'offset-rot270', 'mixto'];

const $ = (id) => document.getElementById(id);
let doc = null, pagina = null, viewport = null, nPag = 1, total = 1;
let arrastre = null;

// El recuadro vive en ESPACIO DE USUARIO PDF, no en pixeles de canvas. Es la
// unica forma de que el zoom sea puramente visual: si se guardaran pixeles, al
// cambiar el zoom el recuadro se quedaria clavado en la pantalla y se moveria
// sobre el documento sin que nadie lo tocase.
let recuadroU = null;

CASOS.forEach((c) => $('caso').add(new Option(c, c)));

async function abrir(nombre) {
  // Sin destruir el anterior se agotan los workers de pdf.js y la segunda
  // apertura se queda colgada para siempre.
  if (doc) { await doc.destroy(); doc = null; }
  doc = await pdfjs.getDocument(`./casos/${nombre}.pdf`).promise;
  total = doc.numPages; nPag = 1; recuadroU = null;
  $('caja').style.display = 'none';
  await pintar();
}

async function pintar() {
  pagina = await doc.getPage(nPag);
  const escala = parseFloat($('zoom').value);
  // Sin pasar `rotation`: pdf.js usa la /Rotate de la pagina, que es lo que
  // ve el usuario en cualquier visor.
  viewport = pagina.getViewport({ scale: escala });
  // Lienzo nuevo en cada pintada: pdf.js no admite dos render() sobre el mismo
  // canvas y aqui no hace falta reciclarlo.
  const cv = document.createElement('canvas');
  cv.id = 'cv';
  $('cv').replaceWith(cv);
  const dpr = window.devicePixelRatio || 1;
  cv.width = Math.floor(viewport.width * dpr);
  cv.height = Math.floor(viewport.height * dpr);
  cv.style.width = viewport.width + 'px';
  cv.style.height = viewport.height + 'px';
  const ctx = cv.getContext('2d');
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  await pagina.render({ canvasContext: ctx, viewport }).promise;
  $('lienzo').style.width = viewport.width + 'px';
  $('lienzo').style.height = viewport.height + 'px';
  $('pag').textContent = `${nPag} / ${total}`;
  $('zoomv').textContent = escala.toFixed(2);
  dibujarCaja();
  volcar();
}

// --- la conversion que este prototipo existe para verificar -----------------
//
// Son DOS pasos, y el segundo es el que no se ve venir.
//
// 1) canvas -> espacio de usuario PDF. Lo hace pdf.js: convertToPdfPoint
//    invierte la matriz del viewport, o sea deshace la escala, el volteo del
//    eje Y, la rotacion /Rotate de la pagina y el origen de la MediaBox.
//    El resultado, ya normalizado, es DONDE tiene que acabar el widget: es el
//    /Rect que se lee en el PDF firmado.
//
// 2) espacio de usuario -> lo que esperan los extraParams. iText no toma el
//    rectangulo tal cual: para /Rotate != 0 le aplica una transformacion antes
//    de escribir el /Rect. Medida en el banco (ver README):
//        T90 (x,y) = (mx1 - y, x)
//        T180(x,y) = (mx1 - x, my1 - y)
//        T270(x,y) = (y, my1 - x)
//    donde mx1,my1 son las esquinas SUPERIORES de la MediaBox. Asi que hay que
//    entregarle T^-1 del rectangulo que queremos.
function aEspacioUsuario(x0, y0, x1, y1) {
  const [ax, ay] = viewport.convertToPdfPoint(x0, y0);
  const [bx, by] = viewport.convertToPdfPoint(x1, y1);
  return { llx: Math.min(ax, bx), lly: Math.min(ay, by),
           urx: Math.max(ax, bx), ury: Math.max(ay, by) };
}

function aExtraParams(u) {
  const mx1 = pagina.view[2], my1 = pagina.view[3];
  const inv = {
    0:   (x, y) => [x, y],
    90:  (x, y) => [y, mx1 - x],
    180: (x, y) => [mx1 - x, my1 - y],
    270: (x, y) => [my1 - y, x],
  }[((pagina.rotate % 360) + 360) % 360];
  const [px, py] = inv(u.llx, u.lly);
  const [qx, qy] = inv(u.urx, u.ury);
  return {
    llx: Math.round(Math.min(px, qx)), lly: Math.round(Math.min(py, qy)),
    urx: Math.round(Math.max(px, qx)), ury: Math.round(Math.max(py, qy)),
  };
}

// Camino de vuelta, solo para pintar: espacio de usuario -> pixeles de canvas.
function aPixeles(u) {
  const [ax, ay] = viewport.convertToViewportPoint(u.llx, u.lly);
  const [bx, by] = viewport.convertToViewportPoint(u.urx, u.ury);
  return { x0: Math.min(ax, bx), y0: Math.min(ay, by),
           x1: Math.max(ax, bx), y1: Math.max(ay, by) };
}

// La version que sale sola si uno no se para a pensar: divide por la escala y
// voltea la Y contra la altura del canvas. Ignora la MediaBox y la rotacion.
function ingenua(r) {
  const s = viewport.scale, h = viewport.height;
  const ay = (h - r.y0) / s, by = (h - r.y1) / s;
  return {
    llx: Math.round(Math.min(r.x0, r.x1) / s), lly: Math.round(Math.min(ay, by)),
    urx: Math.round(Math.max(r.x0, r.x1) / s), ury: Math.round(Math.max(ay, by)),
  };
}

function fila(t, k, v, clase) {
  t.insertAdjacentHTML('beforeend',
    `<tr><td>${k}</td><td class="${clase || ''}">${v}</td></tr>`);
}

function dibujarCaja() {
  const caja = $('caja');
  if (!recuadroU) { caja.style.display = 'none'; return; }
  const r = aPixeles(recuadroU);
  caja.style.display = 'block';
  caja.style.left = r.x0 + 'px';
  caja.style.top = r.y0 + 'px';
  caja.style.width = (r.x1 - r.x0) + 'px';
  caja.style.height = (r.y1 - r.y0) + 'px';
}

function volcar() {
  const [mx0, my0, mx1, my1] = pagina.view;
  $('tpag').innerHTML = '';
  fila($('tpag'), 'MediaBox', `[${mx0} ${my0} ${mx1} ${my1}]`);
  fila($('tpag'), '/Rotate', pagina.rotate);
  fila($('tpag'), 'viewport', `${viewport.width.toFixed(1)} × ${viewport.height.toFixed(1)} px`);

  for (const id of ['tpx', 'tuser', 'tpades', 'tnaive']) $(id).innerHTML = '';
  $('props').value = ''; $('difnota').textContent = '';
  if (!recuadroU) return;

  const px = aPixeles(recuadroU);
  fila($('tpx'), 'x0, y0', `${px.x0.toFixed(1)}, ${px.y0.toFixed(1)}`);
  fila($('tpx'), 'x1, y1', `${px.x1.toFixed(1)}, ${px.y1.toFixed(1)}`);

  const p = aExtraParams(recuadroU), n = ingenua(px);
  const uu = { llx: Math.round(recuadroU.llx), lly: Math.round(recuadroU.lly),
               urx: Math.round(recuadroU.urx), ury: Math.round(recuadroU.ury) };

  fila($('tuser'), 'llx, lly', `${uu.llx}, ${uu.lly}`);
  fila($('tuser'), 'urx, ury', `${uu.urx}, ${uu.ury}`);

  fila($('tpades'), 'signaturePage', nPag);
  fila($('tpades'), 'LowerLeftX', p.llx);
  fila($('tpades'), 'LowerLeftY', p.lly);
  fila($('tpades'), 'UpperRightX', p.urx);
  fila($('tpades'), 'UpperRightY', p.ury);
  for (const k of ['llx', 'lly', 'urx', 'ury'])
    fila($('tnaive'), k, n[k], n[k] === p[k] ? '' : 'malo');
  const dif = ['llx', 'lly', 'urx', 'ury'].filter((k) => n[k] !== p[k]).length;
  $('difnota').textContent = dif
    ? `${dif} de 4 coordenadas difieren: aquí la ingenua colocaría mal el recuadro.`
    : 'Coinciden en este caso (página sin rotar, MediaBox en el origen y zoom 1).';

  $('props').value = [
    `# PROTOTIPO #9 — caso ${$('caso').value}, generado desde el visor`,
    `# rfirma-esperado: {"caso":"${$('caso').value}","pagina":${nPag},` +
      `"widget":[${uu.llx},${uu.lly},${uu.urx},${uu.ury}],` +
      `"mediabox":[${mx0},${my0},${mx1},${my1}],"rotate":${pagina.rotate},` +
      `"zoom":${viewport.scale}}`,
    `signaturePage=${nPag}`,
    `signaturePositionOnPageLowerLeftX=${p.llx}`,
    `signaturePositionOnPageLowerLeftY=${p.lly}`,
    `signaturePositionOnPageUpperRightX=${p.urx}`,
    `signaturePositionOnPageUpperRightY=${p.ury}`,
    `layer2Text=RECUADRO ${uu.llx},${uu.lly} - ${uu.urx},${uu.ury}`,
    '',
  ].join('\n');
}

// --- arrastre ---------------------------------------------------------------
const capa = $('capa');
capa.addEventListener('pointerdown', (e) => {
  const r = capa.getBoundingClientRect();
  arrastre = { x: e.clientX - r.left, y: e.clientY - r.top };
  try { capa.setPointerCapture(e.pointerId); } catch { /* eventos sinteticos */ }
});
capa.addEventListener('pointermove', (e) => {
  if (!arrastre) return;
  const r = capa.getBoundingClientRect();
  // A espacio de usuario ya mismo: en pixeles no se guarda nada.
  recuadroU = aEspacioUsuario(arrastre.x, arrastre.y,
                              e.clientX - r.left, e.clientY - r.top);
  dibujarCaja();
  volcar();
});
capa.addEventListener('pointerup', () => { arrastre = null; });

$('caso').onchange = (e) => abrir(e.target.value);
$('zoom').oninput = () => pintar();
$('ant').onclick = () => { if (nPag > 1) { nPag--; recuadroU = null; pintar(); } };
$('sig').onclick = () => { if (nPag < total) { nPag++; recuadroU = null; pintar(); } };
$('descargar').onclick = () => {
  const a = document.createElement('a');
  a.href = URL.createObjectURL(new Blob([$('props').value], { type: 'text/plain' }));
  a.download = `${$('caso').value}.properties`;
  a.click();
};

// Gancho para el arnes de comprobacion por lotes: repite el mismo rectangulo
// de pantalla en cada caso sin depender de un raton real.
window.__proto9 = {
  // Igual que arrastrar() pero sin pintar: abre el PDF, monta el viewport y
  // aplica la conversion. Es lo que usa el arnes, que no necesita ver nada.
  async medir(nombre, x0, y0, x1, y1, pag = 1, escala = 1) {
    if (doc) { await doc.destroy(); doc = null; }
    doc = await pdfjs.getDocument(`./casos/${nombre}.pdf`).promise;
    total = doc.numPages; nPag = pag;
    pagina = await doc.getPage(pag);
    viewport = pagina.getViewport({ scale: escala });
    $('caso').value = nombre;
    recuadroU = aEspacioUsuario(x0, y0, x1, y1);
    volcar();
    return $('props').value;
  },
  arrastrar(x0, y0, x1, y1) {
    recuadroU = aEspacioUsuario(x0, y0, x1, y1);
    dibujarCaja();
    volcar();
    return $('props').value;
  },
};

abrir(CASOS[0]);
