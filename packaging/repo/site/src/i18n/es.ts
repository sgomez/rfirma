export const es = {
  "meta.title": "rFirma: firma electrónica sin Java, alternativa a AutoFirma",
  "meta.description":
    "Firma PDF con tu certificado digital o DNIe en Linux, Windows y macOS. Funciona en las sedes que piden AutoFirma, sin Java. Versión alfa.",
  "meta.image.alt": "Logotipo de rFirma sobre fondo verde con el dominio rfirma.sgomez.me",

  "notice.aria": "Aviso sobre titularidad oficial",
  "notice.badge": "Aviso",
  "notice.official":
    "<strong>rFirma no es un producto oficial de la Administración Pública.</strong> Es un proyecto independiente de código abierto bajo licencia EUPL 1.2.",

  "nav.aria": "Navegación principal",
  "nav.home.aria": "rFirma — inicio",
  "nav.badge.alpha": "Alfa",
  "nav.how": "Así se firma",
  "nav.comparison": "Comparativa",
  "nav.install": "Instalación",
  "nav.manual": "Manual",
  "nav.transparency": "Transparencia",
  "nav.github": "GitHub",

  "lang.aria": "Idioma",
  "lang.current": "Español",
  "lang.es": "Español",
  "lang.ca": "Català",
  "lang.eu": "Euskara",
  "lang.gl": "Galego",
  "lang.en": "English",

  "hero.kicker": "Alternativa a AutoFirma",
  "hero.title.line1": "rFirma: firma electrónica nativa,",
  "hero.title.line2": "sin Java.",
  "hero.body":
    "Sin esperas. Aplicación de escritorio en Rust y React que sustituye la interfaz Swing y los servidores locales de AutoFirma, con la criptografía oficial de la Administración.",
  "hero.cta.primary": "Instalar rFirma",
  "hero.cta.secondary": "Ver cómo se firma",
  "hero.trust.oss": "Código abierto, EUPL 1.2",
  "hero.trust.keys": "La clave privada no sale de tu equipo",
  "hero.trust.formats": "CAdES, PAdES, XAdES y FacturaE",
  "hero.trust.langs": "En cinco idiomas",
  "hero.window.title": "rFirma — Solicitud-subvencion.pdf",
  "hero.screenshot.alt":
    "Ventana principal de rFirma: visor del documento, colocación de la firma visible y panel lateral de firma",
  "hero.card.cert.label": "Certificado",
  "hero.card.cert.value": "ADA LOVELACE BYRON",
  "hero.card.cert.issuer": "AC FNMT Usuarios",
  "hero.card.phase.label": "Fase",
  "hero.card.phase.value": "Ensamblando",
  "hero.card.stamp.label": "Firma visible",
  "hero.card.stamp.value": "Colocada en la página 3",
  "hero.card.format.value": "PAdES · 2 firmas",

  "formats.aria": "Formatos y almacenes compatibles",

  "how.kicker": "Paso a paso",
  "how.title": "Así se firma un documento",
  "how.tablist.aria": "Pasos de la firma",
  "how.step1.title": "Documento cargado",
  "how.step1.body":
    "rFirma abre el PDF y muestra lo que contiene: páginas, tamaño y las firmas que ya lleva. Si hay una previa, la tuya será una cofirma.",
  "how.step2.title": "Elegir certificado",
  "how.step2.body":
    "La lista se compone con los almacenes del sistema, los perfiles del navegador y los módulos PKCS#11. Los caducados y revocados se muestran, pero no se pueden usar.",
  "how.step3.title": "Introducir el PIN",
  "how.step3.body":
    "El PIN se pide en un diálogo nativo, no en una ventana web, y se borra de la memoria en cuanto termina la firma.",
  "how.step4.title": "Firma completada",
  "how.step4.body": "El PDF firmado se guarda junto al original, y el resumen indica su formato.",
  "how.step1.alt": "rFirma con un PDF abierto que ya lleva una firma válida",
  "how.step2.alt":
    "Selector de certificado abierto, con el buscador y los certificados disponibles",
  "how.step3.alt": "Diálogo nativo para introducir el PIN del certificado",
  "how.step4.alt": "PDF firmado en PAdES, con el resumen de la firma",

  "pillars.kicker": "Por qué rFirma",
  "pillars.title": "Cuatro decisiones de diseño",
  "pillars.body":
    "Sustituye la interfaz Swing y los servidores locales de AutoFirma; el motor criptográfico es el mismo, el de clienteafirma.",
  "pillars.native.title": "Rendimiento nativo",
  "pillars.native.body":
    "Escritorio en Tauri v2, Rust y React. Sin JVM y sin servidores locales a la escucha.",
  "pillars.keys.title": "La clave privada no sale del sistema",
  "pillars.keys.body":
    "La firma se hace en tu equipo, a través de los almacenes del sistema y de los módulos PKCS#11. Java nunca la ve.",
  "pillars.crypto.title": "Criptografía oficial",
  "pillars.crypto.body":
    "CAdES, PAdES, XAdES y FacturaE salen del código de <code>clienteafirma</code>, compilado a binario nativo con GraalVM.",
  "pillars.stamp.title": "Firma visible arrastrable",
  "pillars.stamp.body":
    "Coloca y dimensiona la rúbrica sobre la página, con previsualización fiel. Sin coordenadas a ciegas.",

  "comparison.kicker": "Comparativa",
  "comparison.title": "AutoFirma frente a rFirma",
  "comparison.head.autofirma": "AutoFirma",
  "comparison.head.rfirma": "rFirma",
  "comparison.java.label": "Java",
  "comparison.java.autofirma": "Necesario",
  "comparison.java.rfirma": "No lo usa",
  "comparison.pin.label": "Lectura del PIN en Linux",
  "comparison.pin.autofirma": "Ventana de Java",
  "comparison.pin.rfirma": "Diálogo nativo y memoria protegida",
  "comparison.dnie.label": "Firma con DNIe",
  "comparison.dnie.autofirma": "Sí, con jMulticard",
  "comparison.dnie.rfirma": "Sí, con autodetección en caliente",
  "comparison.store.label": "Certificados <code>.p12</code>",
  "comparison.store.autofirma": "Eliges el fichero en un diálogo",
  "comparison.store.rfirma": "Almacén cifrado propio",
  "comparison.stores.label": "Búsqueda de certificados",
  "comparison.stores.autofirma": "Por almacenes, solo uno activo a la vez",
  "comparison.stores.rfirma": "Todos a la vez, con buscador",
  "comparison.stamp.label": "Firma visible",
  "comparison.stamp.autofirma": "Por coordenadas",
  "comparison.stamp.rfirma": "Arrastrándola sobre la página",
  "comparison.lang.label": "Idiomas",
  "comparison.lang.autofirma": "Castellano",
  "comparison.lang.rfirma": "Castellano, catalán, euskera, gallego e inglés",
  "comparison.os.label": "Sistemas operativos",
  "comparison.os.autofirma": "Windows, macOS, Linux, Android e iOS",
  "comparison.os.rfirma": "Linux y Windows",
  "comparison.updates.label": "Actualizaciones",
  "comparison.updates.autofirma": "Descarga manual",
  "comparison.updates.rfirma": "Automáticas",
  "comparison.desktop.label": "Apariencia",
  "comparison.desktop.autofirma": "Ventanas de Java",
  "comparison.desktop.rfirma": "Las de tu escritorio, con tema claro y oscuro",

  "install.kicker": "Instalación",
  "install.title": "Un repositorio para tu sistema operativo",
  "install.body":
    "Los canales de rFirma son repositorios nativos. Una vez añadido el de tu sistema, los parches de seguridad se instalan con el gestor de paquetes.",
  "install.tablist.aria": "Canales de distribución",
  "install.copy": "Copiar",
  "install.copied": "Copiado",
  "install.copy.flatpak.aria": "Copiar orden de Flatpak",
  "install.copy.apt.aria": "Copiar órdenes para APT",
  "install.copy.dnf.aria": "Copiar órdenes para DNF",
  "install.flatpak.body":
    "Recomendada para las distribuciones de Linux que no usan APT ni DNF. Das de alta Flathub, de donde sale el runtime <code>org.gnome.Platform</code>, y el remoto de rFirma, como un repositorio más: así rFirma aparece también al buscar en GNOME Software. Solo necesitas tener instalados <code>flatpak</code> y <code>xdg-desktop-portal</code>.",
  "install.apt.body":
    "Para Debian, Ubuntu y distribuciones derivadas. Configura el repositorio mediante el formato moderno <code>deb822</code> con la clave GPG verificada en <code>/usr/share/keyrings/</code>.",
  "install.dnf.body":
    "Para Fedora y derivadas basadas en paquetes RPM. Configura el repositorio con comprobación criptográfica estricta de metadatos y paquetes (<code>gpgcheck=1</code> y <code>repo_gpgcheck=1</code>).",
  "install.manual": "Instrucciones detalladas, huella de la clave y verificación, en el manual",
  "install.uninstall":
    "Para no dejar el certificado de rFirma en el navegador, pulsa <em>Retirar certificado</em> en el panel de estado antes de desinstalar.",
  "install.soon": "En desarrollo",
  "install.windows.title": "rFirma para Windows",
  "install.windows.body":
    "Instalador para tu usuario, sin permisos de administrador. Usa directamente el almacén de certificados de Windows (MS-CAPI / CNG) y, una vez instalado, se actualiza desde la propia aplicación: cada versión nueva llega con su firma minisign, que se comprueba antes de instalarla.",
  "install.windows.download": "Descargar el instalador de Windows",
  "install.windows.note":
    'Verifica lo que descargas: baja <code>SHA256SUMS</code> de la <a href="https://github.com/sgomez/rfirma/releases/latest" target="_blank" rel="noopener noreferrer">Release</a> y comprueba que el hash del instalador coincide con su línea. El instalador no está firmado con Authenticode, así que SmartScreen avisará al abrirlo.',
  "install.macos.title": "Soporte para macOS en preparación",
  "install.macos.body":
    "La versión nativa para macOS está en fase de desarrollo activo. Se integrará con el Keychain de Apple y CryptoTokenKit para un acceso fluido y seguro a las identidades digitales del sistema.",
  "install.macos.note":
    'Se distribuirá como imagen de disco <code>.dmg</code> y mediante fórmula de <code>Homebrew</code>. Puedes seguir el avance del proyecto en <a href="https://github.com/sgomez/rfirma" target="_blank" rel="noopener noreferrer">GitHub</a>.',

  "transparency.kicker": "Transparencia",
  "transparency.title": "Todo el código, a la vista",
  "transparency.body":
    "rFirma es software libre y auditable bajo licencia <strong>EUPL 1.2</strong>. El código y el motor criptográfico viven en dos repositorios públicos.",
  "transparency.repo1.title": "Aplicación y puente FFI",
  "transparency.repo1.body":
    'En <a href="https://github.com/sgomez/rfirma" target="_blank" rel="noopener noreferrer">sgomez/rfirma</a> están la interfaz en Tauri (Rust + React), el puente FFI (<code>rfirma-native-bridge</code>) y el empaquetado.',
  "transparency.repo2.title": "Motor criptográfico original",
  "transparency.repo2.body":
    'La lógica de firma consume los artefactos de <a href="https://github.com/ctt-gob-es/clienteafirma" target="_blank" rel="noopener noreferrer">ctt-gob-es/clienteafirma</a>, compilados en una biblioteca nativa con GraalVM Native Image.',
  "transparency.key.title": "Clave de firma de paquetes",
  "transparency.key.body":
    'Clave pública en <a href="https://rfirma.sgomez.me/rfirma.asc">rfirma.asc</a>. Comprueba su huella tras descargarla con <code>gpg --show-keys rfirma.asc</code>:',

  "footer.tagline":
    "Firma electrónica nativa para el escritorio, con el motor criptográfico oficial y sin Java en tu equipo.",
  "footer.license": "Licencia EUPL 1.2",
  "footer.col.project": "Proyecto",
  "footer.repo": "Repositorio en GitHub",
  "footer.releases": "Releases",
  "footer.gpg": "Clave GPG pública",
  "footer.col.origin": "Origen",
  "footer.clienteafirma": "clienteafirma",
  "footer.comparison": "Diferencias con AutoFirma",

  "notFound.title": "Página no encontrada — rFirma",
  "notFound.description": "La dirección que has abierto no existe en rfirma.sgomez.me.",
  "notFound.heading": "Esta página no existe",
  "notFound.body": "Puede que el enlace esté mal escrito o que la página haya cambiado de sitio.",
  "notFound.home": "Volver a la portada",
} as const;

export type Key = keyof typeof es;
export type Dictionary = Record<Key, string>;
