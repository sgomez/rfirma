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
    name: "rfirma",
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
   * texto original crudo. El mapeo de los `CKR_*` concretos y de las
   * excepciones del puente es de otro sub-issue; aquí solo está la situación
   * en la que cae lo que no sabemos clasificar.
   */
  errors: {
    technicalDetail: "Detalle técnico",
    situations: {
      unknown: {
        title: "No se ha podido completar la operación",
        body: "Vuelve a intentarlo. Si sigue ocurriendo, adjunta el detalle técnico al informe del fallo.",
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
    status: {
      signed: "Firmado",
      unsigned: "Sin firmar",
    },
  },
};
