/**
 * Los iconos de la interfaz, **copiados en línea de los artboards**.
 *
 * No hay biblioteca de iconos ni icono de fuente (ID-53): el artboard los trae
 * como `<svg>` en línea y la transcripción los copia tal cual. Meter una
 * dependencia para reproducir un trazado que ya está escrito sería pagar un
 * paquete entero por lo que cabe en este fichero.
 *
 * Todos comparten el mismo lápiz del canvas —`fill="none"`,
 * `stroke="currentColor"`, uniones y extremos redondeados— y heredan el color
 * de quien los monta, así que ninguno fija un color: es lo que exige el ID-58.
 * El grosor del trazo se mantiene en 1.5 salvo donde el artboard lo sube (la
 * marca de verificación, a 3).
 *
 * Los cinco iconos de veredicto son la excepción: no se transcriben del
 * artboard ni usan el `PEN`, sino el `VerdictIcon`. Sus trazados están copiados
 * de [Heroicons](https://heroicons.com) 2.2.0, bajo licencia MIT (© Tailwind
 * Labs). Ver `docs/design/design-system.md`.
 */

interface IconProps {
  /** Lado del cuadro, en px. El artboard fija uno distinto por sitio. */
  size?: number;
}

/** El lápiz común de los artboards: contorno, sin relleno, redondeado. */
const PEN = {
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.5,
  strokeLinecap: "round",
  strokeLinejoin: "round",
} as const;

/** Los dos trazados de Heroicons de un mismo veredicto, uno por rejilla. */
interface VerdictGlyph {
  micro: string;
  mini: string;
}

/**
 * La silueta maciza de un veredicto, en la rejilla que le toca por tamaño.
 *
 * Heroicons dibuja cada glifo dos veces —«Micro» sobre rejilla de 16 y «Mini»
 * sobre la de 20— porque un trazado no se deja escalar: el de 16 ampliado a 24
 * se ve tosco, y el de 20 reducido a 14 pierde los detalles que justifican su
 * rejilla. Se elige el que corresponde en vez de estirar uno.
 */
function VerdictIcon({ size, glyph }: { size: number; glyph: VerdictGlyph }) {
  const isMicro = size <= 16;
  return (
    <svg
      width={size}
      height={size}
      viewBox={isMicro ? "0 0 16 16" : "0 0 20 20"}
      fill="currentColor"
      fillRule="evenodd"
      clipRule="evenodd"
      aria-hidden="true"
      focusable="false"
    >
      <path d={isMicro ? glyph.micro : glyph.mini} />
    </svg>
  );
}

const ALERT: VerdictGlyph = {
  micro:
    "M6.701 2.25c.577-1 2.02-1 2.598 0l5.196 9a1.5 1.5 0 0 1-1.299 2.25H2.804a1.5 1.5 0 0 1-1.3-2.25l5.197-9ZM8 4a.75.75 0 0 1 .75.75v3a.75.75 0 1 1-1.5 0v-3A.75.75 0 0 1 8 4Zm0 8a1 1 0 1 0 0-2 1 1 0 0 0 0 2Z",
  mini: "M8.485 2.495c.673-1.167 2.357-1.167 3.03 0l6.28 10.875c.673 1.167-.17 2.625-1.516 2.625H3.72c-1.347 0-2.189-1.458-1.515-2.625L8.485 2.495ZM10 5a.75.75 0 0 1 .75.75v3.5a.75.75 0 0 1-1.5 0v-3.5A.75.75 0 0 1 10 5Zm0 9a1 1 0 1 0 0-2 1 1 0 0 0 0 2Z",
};

const CHECK_CIRCLE: VerdictGlyph = {
  micro:
    "M8 15A7 7 0 1 0 8 1a7 7 0 0 0 0 14Zm3.844-8.791a.75.75 0 0 0-1.188-.918l-3.7 4.79-1.649-1.833a.75.75 0 1 0-1.114 1.004l2.25 2.5a.75.75 0 0 0 1.15-.043l4.25-5.5Z",
  mini: "M10 18a8 8 0 1 0 0-16 8 8 0 0 0 0 16Zm3.857-9.809a.75.75 0 0 0-1.214-.882l-3.483 4.79-1.88-1.88a.75.75 0 1 0-1.06 1.061l2.5 2.5a.75.75 0 0 0 1.137-.089l4-5.5Z",
};

const CROSS_CIRCLE: VerdictGlyph = {
  micro:
    "M8 15A7 7 0 1 0 8 1a7 7 0 0 0 0 14Zm2.78-4.22a.75.75 0 0 1-1.06 0L8 9.06l-1.72 1.72a.75.75 0 1 1-1.06-1.06L6.94 8 5.22 6.28a.75.75 0 0 1 1.06-1.06L8 6.94l1.72-1.72a.75.75 0 1 1 1.06 1.06L9.06 8l1.72 1.72a.75.75 0 0 1 0 1.06Z",
  mini: "M10 18a8 8 0 1 0 0-16 8 8 0 0 0 0 16ZM8.28 7.22a.75.75 0 0 0-1.06 1.06L8.94 10l-1.72 1.72a.75.75 0 1 0 1.06 1.06L10 11.06l1.72 1.72a.75.75 0 1 0 1.06-1.06L11.06 10l1.72-1.72a.75.75 0 0 0-1.06-1.06L10 8.94 8.28 7.22Z",
};

const MINUS_CIRCLE: VerdictGlyph = {
  micro:
    "M8 15A7 7 0 1 0 8 1a7 7 0 0 0 0 14Zm4-7a.75.75 0 0 0-.75-.75h-6.5a.75.75 0 0 0 0 1.5h6.5A.75.75 0 0 0 12 8Z",
  mini: "M10 18a8 8 0 1 0 0-16 8 8 0 0 0 0 16ZM6.75 9.25a.75.75 0 0 0 0 1.5h6.5a.75.75 0 0 0 0-1.5h-6.5Z",
};

const ARROW_PATH: VerdictGlyph = {
  micro:
    "M13.836 2.477a.75.75 0 0 1 .75.75v3.182a.75.75 0 0 1-.75.75h-3.182a.75.75 0 0 1 0-1.5h1.37l-.84-.841a4.5 4.5 0 0 0-7.08.932.75.75 0 0 1-1.3-.75 6 6 0 0 1 9.44-1.242l.842.84V3.227a.75.75 0 0 1 .75-.75Zm-.911 7.5A.75.75 0 0 1 13.199 11a6 6 0 0 1-9.44 1.241l-.84-.84v1.371a.75.75 0 0 1-1.5 0V9.591a.75.75 0 0 1 .75-.75H5.35a.75.75 0 0 1 0 1.5H3.98l.841.841a4.5 4.5 0 0 0 7.08-.932.75.75 0 0 1 1.025-.273Z",
  mini: "M15.312 11.424a5.5 5.5 0 0 1-9.201 2.466l-.312-.311h2.433a.75.75 0 0 0 0-1.5H3.989a.75.75 0 0 0-.75.75v4.242a.75.75 0 0 0 1.5 0v-2.43l.31.31a7 7 0 0 0 11.712-3.138.75.75 0 0 0-1.449-.39Zm1.23-3.723a.75.75 0 0 0 .219-.53V2.929a.75.75 0 0 0-1.5 0V5.36l-.31-.31A7 7 0 0 0 3.239 8.188a.75.75 0 1 0 1.448.389A5.5 5.5 0 0 1 13.89 6.11l.311.31h-2.432a.75.75 0 0 0 0 1.5h4.243a.75.75 0 0 0 .53-.219Z",
};

/** Las tres rayas del botón de menú de la cabecera. */
export function MenuIcon({ size = 20 }: IconProps) {
  return (
    <svg width={size} height={size} {...PEN} aria-hidden="true" focusable="false">
      <path d="M4 7h16M4 12h16M4 17h16" />
    </svg>
  );
}

/** La flecha de subir de las dos zonas de soltar. */
export function UploadIcon({ size = 28 }: IconProps) {
  return (
    <svg width={size} height={size} {...PEN} aria-hidden="true" focusable="false">
      <path d="M12 16V4M8 8l4-4 4 4" />
      <path d="M4 16v3a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-3" />
    </svg>
  );
}

/** La hoja con la esquina doblada, junto al nombre del documento del panel. */
export function FileIcon({ size = 20 }: IconProps) {
  return (
    <svg width={size} height={size} {...PEN} aria-hidden="true" focusable="false">
      <path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z" />
      <path d="M14 3v5h5" />
    </svg>
  );
}

/** El círculo con la «i», de los avisos informativos del panel. */
export function InfoIcon({ size = 20 }: IconProps) {
  return (
    <svg width={size} height={size} {...PEN} aria-hidden="true" focusable="false">
      <circle cx="12" cy="12" r="9" />
      <path d="M12 11v6M12 7.5v.5" />
    </svg>
  );
}

/** El triángulo de aviso del error de firma y del PIN incorrecto. */
export function AlertIcon({ size = 20 }: IconProps) {
  return <VerdictIcon size={size} glyph={ALERT} />;
}

/** La escarapela del certificado. */
export function CertificateIcon({ size = 20 }: IconProps) {
  return (
    <svg width={size} height={size} {...PEN} aria-hidden="true" focusable="false">
      <circle cx="12" cy="9" r="5" />
      <path d="M8.5 13.5 7 21l5-2.5L17 21l-1.5-7.5" />
    </svg>
  );
}

/**
 * El icono del bloque «La firma visible aparecerá en…» del diálogo de páginas
 * sin firma visible (docs/design/dialogo-paginas-sin-firma-visible.md): el
 * recuadro con la marca dentro, para que se lea junto al resto de recuadros
 * del sistema de diseño y no como un icono suelto.
 */
export function SealIcon({ size = 20 }: IconProps) {
  return (
    <svg width={size} height={size} {...PEN} aria-hidden="true" focusable="false">
      <rect x="4" y="4" width="16" height="16" rx="3" />
      <path d="M8 12.5 10.5 15 16 9" />
    </svg>
  );
}

/**
 * La marca de verificación.
 *
 * El artboard la dibuja con dos grosores: 3 en las casillas de 12 px del
 * contenido del recuadro, y 2 en la de 20 px de la fase cumplida del diálogo
 * de progreso. Un trazo de 3 a 20 px se ve tosco, y por eso el grosor es un
 * parámetro y no una constante.
 */
export function CheckIcon({ size = 12, strokeWidth = 3 }: IconProps & { strokeWidth?: number }) {
  return (
    <svg
      width={size}
      height={size}
      {...PEN}
      strokeWidth={strokeWidth}
      aria-hidden="true"
      focusable="false"
    >
      <path d="M4 12.5 9.5 18 20 6" />
    </svg>
  );
}

/** El círculo con la marca dentro: el desenlace «Firmado y enviado». */
export function CheckCircleIcon({ size = 24 }: IconProps) {
  return <VerdictIcon size={size} glyph={CHECK_CIRCLE} />;
}

/** El círculo con el aspa dentro: el desenlace «Has cancelado la firma». */
export function CrossCircleIcon({ size = 24 }: IconProps) {
  return <VerdictIcon size={size} glyph={CROSS_CIRCLE} />;
}

/** La carpeta de la fila «Se guardará en». */
export function FolderIcon({ size = 20 }: IconProps) {
  return (
    <svg width={size} height={size} {...PEN} aria-hidden="true" focusable="false">
      <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
    </svg>
  );
}

/** La flecha «siguiente» de la barra del visor y de los enlaces del panel. */
export function ChevronRightIcon({ size = 16 }: IconProps) {
  return (
    <svg width={size} height={size} {...PEN} aria-hidden="true" focusable="false">
      <path d="M9 6l6 6-6 6" />
    </svg>
  );
}

/** La punta de flecha del desplegable de Preferencias. */
export function ChevronDownIcon({ size = 16 }: IconProps) {
  return (
    <svg width={size} height={size} {...PEN} aria-hidden="true" focusable="false">
      <path d="M6 9l6 6 6-6" />
    </svg>
  );
}

/** La flecha «anterior» de la barra del visor. */
export function ChevronLeftIcon({ size = 16 }: IconProps) {
  return (
    <svg width={size} height={size} {...PEN} aria-hidden="true" focusable="false">
      <path d="M15 6l-6 6 6 6" />
    </svg>
  );
}

/** La doble flecha «a la primera página». */
export function ChevronsLeftIcon({ size = 16 }: IconProps) {
  return (
    <svg width={size} height={size} {...PEN} aria-hidden="true" focusable="false">
      <path d="M17 6l-6 6 6 6M9 6l-6 6 6 6" />
    </svg>
  );
}

/** La doble flecha «a la última página». */
export function ChevronsRightIcon({ size = 16 }: IconProps) {
  return (
    <svg width={size} height={size} {...PEN} aria-hidden="true" focusable="false">
      <path d="M7 6l6 6-6 6M15 6l6 6-6 6" />
    </svg>
  );
}

/** El menos de alejar el zoom. */
export function MinusIcon({ size = 16 }: IconProps) {
  return (
    <svg width={size} height={size} {...PEN} aria-hidden="true" focusable="false">
      <path d="M5 12h14" />
    </svg>
  );
}

/** El más de acercar el zoom. */
export function PlusIcon({ size = 16, strokeWidth = 1.5 }: IconProps & { strokeWidth?: number }) {
  return (
    <svg
      width={size}
      height={size}
      {...PEN}
      strokeWidth={strokeWidth}
      aria-hidden="true"
      focusable="false"
    >
      <path d="M12 5v14M5 12h14" />
    </svg>
  );
}

/** El aspa de cerrar una pestaña. */
export function CloseIcon({ size = 12 }: IconProps) {
  return (
    <svg width={size} height={size} {...PEN} strokeWidth={1.8} aria-hidden="true" focusable="false">
      <path d="M6 6l12 12M18 6 6 18" />
    </svg>
  );
}

/** La marca de «ya lleva firmas» de las pestañas y de los recientes. */
export function SignedMarkIcon({ size = 13 }: IconProps) {
  return (
    <svg width={size} height={size} {...PEN} strokeWidth={2} aria-hidden="true" focusable="false">
      <path d="M5 12.5l4.5 4.5L19 7.5" />
    </svg>
  );
}

/** Las cuatro esquinas de «ajustar a la ventana». */
export function FitIcon({ size = 16 }: IconProps) {
  return (
    <svg width={size} height={size} {...PEN} aria-hidden="true" focusable="false">
      <path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5" />
    </svg>
  );
}

/** La hoja entera dentro del marco: «ajustar a la página». */
export function FitPageIcon({ size = 16 }: IconProps) {
  return (
    <svg width={size} height={size} {...PEN} aria-hidden="true" focusable="false">
      <rect x="7" y="4" width="10" height="16" rx="1" />
      <path d="M4 8V4h3M20 8V4h-3M4 16v4h3M20 16v4h-3" />
    </svg>
  );
}

/** La cruz de cuatro puntas del rótulo «Arrástralo para colocarlo». */
export function MoveIcon({ size = 14 }: IconProps) {
  return (
    <svg width={size} height={size} {...PEN} aria-hidden="true" focusable="false">
      <path d="M12 3v18M3 12h18M12 3 9 6M12 3l3 3M12 21l-3-3M12 21l3-3M3 12l3-3M3 12l3 3M21 12l-3-3M21 12l-3 3" />
    </svg>
  );
}

/** La flecha de «hay una versión nueva», en *Acerca de*. */
export function ArrowUpIcon({ size = 18 }: IconProps) {
  return (
    <svg width={size} height={size} {...PEN} aria-hidden="true" focusable="false">
      <path d="M12 20V6M6 12l6-6 6 6" />
    </svg>
  );
}

/** El icono del botón «Copiar» del bloque de órdenes, en *Acerca de*. */
export function CopyIcon({ size = 14 }: IconProps) {
  return (
    <svg width={size} height={size} {...PEN} aria-hidden="true" focusable="false">
      <rect x="9" y="9" width="11" height="11" rx="2" />
      <path d="M5 15H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v1" />
    </svg>
  );
}

/** El icono de enlace externo de «Comentarios y ayuda». */
export function ExternalLinkIcon({ size = 14 }: IconProps) {
  return (
    <svg width={size} height={size} {...PEN} strokeWidth={1.8} aria-hidden="true" focusable="false">
      <path d="M14 4h6v6" />
      <path d="M20 4 11 13" />
      <path d="M18 14v4a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4" />
    </svg>
  );
}

/** El círculo con la raya dentro: el veredicto «No aplica» del panel de estado. */
export function NotApplicableIcon({ size = 16 }: IconProps) {
  return <VerdictIcon size={size} glyph={MINUS_CIRCLE} />;
}

/** Las dos flechas en ciclo: el veredicto «Comprobando» del panel de estado. */
export function CheckingIcon({ size = 16 }: IconProps) {
  return <VerdictIcon size={size} glyph={ARROW_PATH} />;
}

/** El trazo de la rúbrica, en el hueco de las tarjetas de modelo. */
export function RubricIcon({
  width = "100%",
  height = "100%",
}: {
  width?: number | string;
  height?: number | string;
}) {
  return (
    <svg
      width={width}
      height={height}
      viewBox="0 0 84 40"
      preserveAspectRatio="xMidYMid meet"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      <path d="M4 30c8-18 13-22 16-14 3 8-4 18-7 16-3-2 6-14 15-16 6-1 4 8 8 9 4 1 8-6 12-10" />
      <path d="M50 24c6 2 12-2 18-8M62 32c6 0 12-3 18-9" />
    </svg>
  );
}
