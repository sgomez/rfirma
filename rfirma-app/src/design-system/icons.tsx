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
  return (
    <svg width={size} height={size} {...PEN} aria-hidden="true" focusable="false">
      <path d="M12 4 2.5 20h19z" />
      <path d="M12 10v5M12 17.5v.5" />
    </svg>
  );
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
 * El sello del bloque «El sello aparecerá en…» del diálogo de páginas sin
 * sello (docs/design/dialogo-paginas-sin-sello.md): el recuadro con la marca
 * dentro, para que se lea junto al resto de recuadros del sistema de diseño y
 * no como un icono suelto.
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
  return (
    <svg width={size} height={size} {...PEN} aria-hidden="true" focusable="false">
      <circle cx="12" cy="12" r="9" />
      <path d="M8 12.5 11 15.5 16.5 9" />
    </svg>
  );
}

/** El círculo con el aspa dentro: el desenlace «Has cancelado la firma». */
export function CrossCircleIcon({ size = 24 }: IconProps) {
  return (
    <svg width={size} height={size} {...PEN} aria-hidden="true" focusable="false">
      <circle cx="12" cy="12" r="9" />
      <path d="M9 9l6 6M15 9l-6 6" />
    </svg>
  );
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
export function PlusIcon({ size = 16 }: IconProps) {
  return (
    <svg width={size} height={size} {...PEN} aria-hidden="true" focusable="false">
      <path d="M12 5v14M5 12h14" />
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

/** La cruz de cerrar de la barra de título de la ventana de sede. */
export function CloseIcon({ size = 14 }: IconProps) {
  return (
    <svg width={size} height={size} {...PEN} aria-hidden="true" focusable="false">
      <path d="M6 6l12 12M18 6 6 18" />
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
