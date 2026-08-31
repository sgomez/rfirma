/**
 * El catálogo en castellano, que es además la **forma** del catálogo: el resto
 * de idiomas se declara como `Catalog`, y ese tipo sale de aquí.
 *
 * Las cadenas se escriben desde cero con el vocabulario de `CONTEXT.md`
 * (ADR-0009). Los `.properties` de AutoFirma se consultan como referencia
 * terminológica —cómo dice el cliente oficial «certificado» o «almacén» es lo
 * que el usuario ya ha visto— pero no son el origen de ninguna cadena: tienen
 * forma de diálogo Swing y esta interfaz es un rediseño.
 *
 * La clave va en inglés y el texto en castellano, como manda `CLAUDE.md`.
 */
export const es = {
  app: {
    // El nombre propio del programa. No se traduce en ningún idioma, pero vive
    // en el catálogo para que ningún componente lo escriba en línea.
    name: "rFirma",
  },
  actions: {
    sign: "Firmar documento",
    chooseCertificate: "Elegir certificado",
    cancel: "Cancelar",
    close: "Cerrar",
    change: "Cambiar",
  },
  /**
   * Los nombres de los seis idiomas, en endónimo: un desplegable de idiomas
   * enseña cada lengua como se llama a sí misma, así que estas seis cadenas
   * son las mismas en los seis catálogos.
   */
  languages: {
    es: "Español",
    ca: "Català",
    eu: "Euskara",
    gl: "Galego",
    va: "Valencià",
    en: "English",
  },
  /**
   * El molde del ID-29: una **situación** nuestra, traducida, y aparte el
   * texto original crudo. Las situaciones son las de `pkcs11::error` y
   * `rubric::error` en el backend, con los mismos nombres: quien clasifica es
   * Rust y aquí solo se traduce lo ya clasificado. Lo que no sabe clasificar
   * cae en `unknown`, que sigue llevando el código crudo.
   */
  errors: {
    technicalDetail: "Detalle técnico",
    situations: {
      unknown: {
        title: "No se ha podido completar la operación",
        body: "Vuelve a intentarlo. Si sigue ocurriendo, adjunta el detalle técnico al informe del fallo.",
      },
      notAnAcceptedImage: {
        title: "Esa imagen no vale como rúbrica",
        body: "La rúbrica tiene que ser un PNG o un JPEG.",
      },
      damagedImage: {
        title: "No hemos podido abrir la imagen",
        body: "El fichero es un PNG o un JPEG, pero está dañado. Prueba con otra copia.",
      },
      imageTooLarge: {
        title: "La imagen es demasiado grande",
        body: "Elige una imagen más pequeña o guárdala con menos resolución.",
      },
      sourceUnreadable: {
        title: "No hemos podido leer el fichero que elegiste",
        body: "Comprueba que sigue donde estaba y vuelve a elegirlo.",
      },
      storeUnwritable: {
        title: "No hemos podido guardar tu rúbrica",
        body: "Vuelve a elegir la imagen. Si sigue ocurriendo, adjunta el detalle técnico al informe del fallo.",
      },
      storeUnreadable: {
        title: "No hemos podido leer tu rúbrica guardada",
        body: "Vuelve a elegir la imagen para reemplazarla.",
      },
    },
  },
  /**
   * El armazón de la ventana: la cabecera y las tres regiones fijas del ID-25.
   * Ninguna aparece ni desaparece durante el recorrido, así que sus rótulos son
   * los nombres accesibles de las regiones y no títulos visibles.
   */
  window: {
    tray: "Bandeja de documentos",
    viewer: "Visor del documento",
    panel: "Panel de firma",
  },
  header: {
    menu: "Menú",
    preferences: "Preferencias…",
    about: "Acerca de rFirma",
  },
  /**
   * El vocabulario de insignias, compartido por la cabecera y la bandeja.
   *
   * `Firmado` y `Sin firmar` describen el documento y se cachean; `No
   * disponible` no describe el documento sino que **la ruta no responde**, así
   * que solo aparece en la bandeja y no se guarda nunca.
   */
  badges: {
    signed: "Firmado",
    unsigned: "Sin firmar",
    unavailable: "No disponible",
  },
  /**
   * La bandeja: el único sitio donde se abre o se cambia de documento
   * (docs/design/bandeja-de-documentos.md).
   */
  tray: {
    dropZone: "Arrastra un PDF o pulsa para abrirlo",
    empty: "Aquí aparecerán los documentos que vayas firmando",
    recents: "Recientes",
    remove: "Quitar de la lista",
  },
  /**
   * El visor (docs/design/visor-de-documento.md): la hoja, el recuadro de la
   * firma y la barra flotante. `dragHandle` es el asa del recuadro, y
   * `outOfPage` el aviso del ID-22: fuera de la página no se acepta.
   */
  viewer: {
    dropZone: "Arrastra un PDF o pulsa para abrirlo",
    privacy: "Solo PDF. El documento no sale de tu ordenador en ningún momento.",
    pageNumber: "Número de página",
    pageOf: "de {{total}}",
    firstPage: "Primera página",
    previousPage: "Página anterior",
    nextPage: "Página siguiente",
    lastPage: "Última página",
    zoomOut: "Alejar",
    zoomIn: "Acercar",
    fitToWindow: "Ajustar a la ventana",
    signatureBox: "Recuadro de la firma visible",
    dragHandle: "Arrastra para colocar",
    outOfPage: "El recuadro se ha quedado fuera de la página, así que sigue donde estaba.",
  },
  /**
   * El panel de firma (docs/design/panel-de-firma.md): todo lo que hay que
   * decidir antes de firmar, y el botón que firma.
   *
   * **Ningún comodín.** No hay aquí ni `$$SUBJECTCN$$` ni `$$SIGNDATE$$`: el
   * usuario marca casillas y el texto del recuadro lo compone Rust (ID-19).
   */
  panel: {
    document: {
      pages: "{{pages}} páginas",
    },
    coSignature: {
      one: "Ya lleva 1 firma: la tuya será una cofirma.",
      many: "Ya lleva {{count}} firmas: la tuya será una cofirma.",
    },
    certificate: {
      title: "Certificado",
      loading: "Buscando certificados…",
      issuer: "Emitido por {{issuer}}",
      choose: "Elegir certificado",
      empty: {
        title: "No hemos encontrado ningún certificado",
        body: "Si usas una tarjeta, comprueba que está insertada y que el lector está conectado.",
        retry: "Volver a buscar",
        otherModule: "Otro módulo…",
      },
      expired: "El certificado caducó el {{date}}, así que no se puede firmar con él.",
      notYetValid:
        "El certificado todavía no ha entrado en vigor, así que no se puede firmar con él.",
      revoked: "El certificado está revocado ({{reason}}), así que no se puede firmar con él.",
      unreadable: "No hemos podido leer este certificado, así que no se puede firmar con él.",
    },
    visibleSignature: {
      title: "Firma visible",
      toggle: "Estampar un recuadro de firma en el documento",
      placement: "Página {{page}} · arrástralo para colocarlo",
      noPlacement: "Arrastra el recuadro sobre el documento para colocarlo.",
      content: "Contenido",
      fields: {
        rubric: "Tu rúbrica",
        rubricDisabled: "Elige antes una imagen",
        signerName: "Nombre y apellidos",
        idNumber: "DNI",
        signedAt: "Fecha y hora de la firma",
        reason: "Un motivo",
      },
      reason: {
        label: "Motivo",
        placeholder: "Conforme",
      },
      rubric: {
        title: "Imagen de la rúbrica",
        choose: "Elegir imagen",
        change: "Cambiar imagen",
        thumbnail: "Tu rúbrica, tal como se estampará",
        flattened:
          "Se estampa sobre blanco: el recuadro va al PDF como JPEG y un JPEG no tiene transparencia.",
      },
      preview: {
        title: "Lo que dirá el recuadro",
        empty: "Marca alguna casilla para que el recuadro diga algo.",
        unavailable: "La vista previa aparecerá al elegir el certificado.",
      },
    },
    footer: {
      savedIn: "Se guardará en",
      unwritable: "No se puede escribir en {{folder}}",
      retry: "Volver a intentarlo",
    },
  },
  /** Los ajustes (docs/design/preferencias.md). Se aplican al hacerlos. */
  preferences: {
    title: "Preferencias",
    rememberVisibleSignature: {
      label: "Recordar la última configuración de firma visible",
      hint: "La página, la posición y el contenido del recuadro se reutilizan en el siguiente documento.",
    },
    rememberActivity: {
      label: "Recordar mi actividad",
      hint: "Cubre los documentos recientes y el certificado usado la última vez.",
      clear: "Vaciar la lista",
      confirm: {
        body: "Al apagarlo se borra lo ya recordado: los documentos recientes y el certificado usado la última vez.",
        accept: "Apagar y borrar",
      },
    },
    destination: {
      label: "Dónde se guarda el documento firmado",
    },
    language: {
      label: "Idioma",
    },
  },
  /** Acerca de rFirma (docs/design/acerca-de.md). */
  about: {
    version: "Versión {{version}}",
    whatItDoes:
      "Firma y cofirma documentos PDF con tu certificado. El documento y la clave privada no salen de tu ordenador.",
    independence:
      "Proyecto independiente. rFirma no está relacionada con AutoFirma ni con la Administración General del Estado, que publican el cliente oficial, ni cuenta con su respaldo. Si necesitas la aplicación oficial, descárgala de su web.",
    licenses: {
      title: "Licencias",
      view: "Ver las licencias",
      rfirma: "rFirma: EUPL-1.2.",
      afirma: "Bibliotecas de Cliente @firma: GPL-2.0+ / EUPL-1.1.",
    },
  },
};
