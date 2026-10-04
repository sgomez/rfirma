import type { Dictionary } from "./es";

export const gl: Dictionary = {
  "meta.title": "rFirma: sinatura electrónica sen Java, alternativa a AutoFirma",
  "meta.description":
    "Asina PDF co teu certificado dixital ou DNIe en Linux, Windows e macOS. Funciona nas sedes que piden AutoFirma, sen Java. Versión alfa.",
  "meta.image.alt": "Logotipo de rFirma sobre fondo verde co dominio rfirma.sgomez.me",

  "notice.aria": "Aviso sobre titularidade oficial",
  "notice.badge": "Aviso",
  "notice.official":
    "<strong>rFirma non é un produto oficial da Administración Pública.</strong> É un proxecto independente de código aberto baixo licenza EUPL 1.2.",

  "nav.aria": "Navegación principal",
  "nav.home.aria": "rFirma — inicio",
  "nav.badge.alpha": "Alfa",
  "nav.how": "Así se asina",
  "nav.features": "Características",
  "nav.comparison": "Comparativa",
  "nav.install": "Instalación",
  "nav.transparency": "Transparencia",
  "nav.github": "GitHub",

  "lang.aria": "Idioma",
  "lang.current": "Galego",
  "lang.es": "Español",
  "lang.ca": "Català",
  "lang.eu": "Euskara",
  "lang.gl": "Galego",
  "lang.en": "English",

  "hero.kicker": "Unha alternativa a AutoFirma",
  "hero.title.line1": "rFirma: sinatura electrónica nativa,",
  "hero.title.line2": "sen Java.",
  "hero.body":
    "Sen esperas. Aplicación de escritorio en Rust e React que substitúe a interface Swing e os servidores locais de AutoFirma, coa criptografía oficial da Administración.",
  "hero.cta.primary": "Instalar rFirma",
  "hero.cta.secondary": "Ver como se asina",
  "hero.trust.oss": "Código aberto, EUPL 1.2",
  "hero.trust.keys": "A clave privada non sae do teu equipo",
  "hero.trust.formats": "CAdES, PAdES, XAdES e FacturaE",
  "hero.trust.langs": "En cinco idiomas",
  "hero.window.title": "rFirma — Solicitud-subvencion.pdf",
  "hero.screenshot.alt":
    "Xanela principal de rFirma: visor do documento, colocación da sinatura visible e panel lateral de sinatura",
  "hero.card.cert.label": "Certificado",
  "hero.card.cert.value": "ADA LOVELACE BYRON",
  "hero.card.cert.issuer": "AC FNMT Usuarios",
  "hero.card.phase.label": "Fase",
  "hero.card.phase.value": "Ensamblando",
  "hero.card.stamp.label": "Sinatura visible",
  "hero.card.stamp.value": "Colocada na páxina 3",
  "hero.card.format.value": "PAdES · 2 sinaturas",

  "formats.aria": "Formatos e almacéns compatibles",

  "how.kicker": "O proceso",
  "how.title": "Así se asina un documento",
  "how.tablist.aria": "Pasos da sinatura",
  "how.step1.title": "Documento cargado",
  "how.step1.body":
    "rFirma abre o PDF e mostra o que contén: páxinas, tamaño e as sinaturas que xa leva. Se hai unha previa, a túa será unha cosinatura.",
  "how.step2.title": "Escoller certificado",
  "how.step2.body":
    "A lista compóñese cos almacéns do sistema, os perfís do navegador e os módulos PKCS#11. Os caducados e revogados amósanse, pero non se poden usar.",
  "how.step3.title": "Introducir o PIN",
  "how.step3.body":
    "O PIN pídese nun diálogo nativo, non nunha xanela web, e bórrase da memoria en canto remata a sinatura.",
  "how.step4.title": "Sinatura completada",
  "how.step4.body": "O PDF asinado gárdase xunto ao orixinal, e o resumo indica o seu formato.",
  "how.step1.alt": "rFirma cun PDF aberto que xa leva unha sinatura válida",
  "how.step2.alt": "Selector de certificado aberto, co buscador e os certificados dispoñibles",
  "how.step3.alt": "Diálogo nativo para introducir o PIN do certificado",
  "how.step4.alt": "PDF asinado en PAdES, co resumo da sinatura",

  "pillars.kicker": "Por que rFirma",
  "pillars.title": "Catro decisións de deseño",
  "pillars.body":
    "Substitúe a interface Swing e os servidores locais de AutoFirma; o motor criptográfico é o mesmo, o de clienteafirma.",
  "pillars.native.title": "Rendemento nativo",
  "pillars.native.body":
    "Escritorio en Tauri v2, Rust e React. Sen JVM e sen servidores locais á escoita.",
  "pillars.keys.title": "A clave privada non sae do sistema",
  "pillars.keys.body":
    "A sinatura faise no teu equipo, a través dos almacéns do sistema e dos módulos PKCS#11. Java nunca a ve.",
  "pillars.crypto.title": "Criptografía oficial",
  "pillars.crypto.body":
    "CAdES, PAdES, XAdES e FacturaE saen do código de <code>clienteafirma</code>, compilado a binario nativo con GraalVM.",
  "pillars.stamp.title": "Sinatura visible arrastrable",
  "pillars.stamp.body":
    "Coloca e dimensiona a rúbrica sobre a páxina, con previsualización fiel. Sen coordenadas ás cegas.",

  "comparison.kicker": "Comparativa",
  "comparison.title": "AutoFirma fronte a rFirma",
  "comparison.head.aspect": "Aspecto",
  "comparison.head.autofirma": "AutoFirma (oficial)",
  "comparison.head.rfirma": "rFirma",
  "comparison.arch.label": "Arquitectura",
  "comparison.arch.autofirma": "Java Swing sobre JVM",
  "comparison.arch.rfirma":
    "Tauri v2 (Rust + React) co motor de clienteafirma en GraalVM Native Image",
  "comparison.keys.label": "Onde se procesa a clave privada",
  "comparison.keys.autofirma": "No proceso Java",
  "comparison.keys.rfirma": "No almacén do sistema ou o módulo PKCS#11; non sae del",
  "comparison.pin.label": "Protección do PIN na memoria",
  "comparison.pin.autofirma": "Bórrase só en parte e pode acabar no disco",
  "comparison.pin.rfirma": "Protexido na memoria, nunca vai ao disco e bórrase tras usalo",
  "comparison.dnie.label": "Sinatura co DNIe",
  "comparison.dnie.autofirma": "Si, mediante jMulticard",
  "comparison.dnie.rfirma": "En desenvolvemento",
  "comparison.store.label": "Certificados en ficheiro (<code>.p12</code>)",
  "comparison.store.autofirma":
    "Rexístrase a ruta do ficheiro nun diálogo de almacéns con seis opcións",
  "comparison.store.rfirma": "Almacén propio e cifrado, que se abre coa túa sesión",
  "comparison.stores.label": "Busca de certificados",
  "comparison.stores.autofirma": "Só busca no almacén que elixas",
  "comparison.stores.rfirma": "Todos nunha lista con buscador, un certificado por fila",
  "comparison.stamp.label": "Colocación da sinatura visible",
  "comparison.stamp.autofirma": "Coordenadas ou cadro sen contexto",
  "comparison.stamp.rfirma":
    "Arrastre sobre a páxina, con modelos de texto e previsualización fiel",
  "comparison.ca.label": "Confianza do navegador no servidor local",
  "comparison.ca.autofirma": "O instalador rexistra a CA no sistema, con privilexios",
  "comparison.ca.rfirma": "A aplicación rexistra a súa CA nos almacéns NSS da persoa, sen root",
  "comparison.lang.label": "Idiomas",
  "comparison.lang.autofirma": "Só castelán, coas cadeas dos diálogos Swing",
  "comparison.lang.rfirma":
    "Castelán, català, euskara, galego e inglés, con catálogo propio e cambio desde Preferencias",
  "comparison.privacy.label": "Xestor de documentos",
  "comparison.privacy.autofirma": "—",
  "comparison.privacy.rfirma": "Lembra os documentos recentes e o último certificado usado",
  "comparison.os.label": "Compatibilidade de sistemas",
  "comparison.os.autofirma": "Windows, macOS, Linux, Android e iOS",
  "comparison.os.rfirma": "Linux e Windows; macOS, en desenvolvemento",
  "comparison.updates.label": "Canle de actualización",
  "comparison.updates.autofirma": "Descarga manual de <code>.deb</code> ou <code>.rpm</code>",
  "comparison.updates.rfirma":
    "Repositorios nativos: APT, DNF e Flatpak; en Windows, desde a propia aplicación",
  "comparison.desktop.label": "Integración co escritorio",
  "comparison.desktop.autofirma": "Aparencia propia de Swing",
  "comparison.desktop.rfirma": "Segue o estilo do escritorio, con tema claro e escuro",

  "install.kicker": "Instalación",
  "install.title": "Un repositorio para o teu sistema operativo",
  "install.body":
    "As canles de rFirma son repositorios nativos. Unha vez engadido o do teu sistema, os parches de seguridade instálanse co xestor de paquetes.",
  "install.tablist.aria": "Canles de distribución",
  "install.copy": "Copiar",
  "install.copied": "Copiado",
  "install.copy.flatpak.aria": "Copiar orde de Flatpak",
  "install.copy.apt.aria": "Copiar ordes para APT",
  "install.copy.dnf.aria": "Copiar ordes para DNF",
  "install.flatpak.body":
    "Recomendada para as distribucións de Linux que non usan APT nin DNF. Resólvese desde o remoto ostree propio de rFirma, e o runtime <code>org.gnome.Platform</code> descárgase de Flathub sen configurar nada. Só precisas ter instalados <code>flatpak</code> e <code>xdg-desktop-portal</code>.",
  "install.flatpak.tip":
    'Tamén podes descargar e instalar con dobre clic o ficheiro <a href="https://rfirma.sgomez.me/rfirma.flatpakref">rfirma.flatpakref</a> se o teu escritorio o soporta.',
  "install.apt.body":
    "Para Debian, Ubuntu e distribucións derivadas. Configura o repositorio mediante o formato moderno <code>deb822</code> coa clave GPG verificada en <code>/usr/share/keyrings/</code>.",
  "install.dnf.body":
    "Para Fedora e derivadas baseadas en paquetes RPM. Configura o repositorio con comprobación criptográfica estrita de metadatos e paquetes (<code>gpgcheck=1</code> e <code>repo_gpgcheck=1</code>).",
  "install.soon": "En desenvolvemento",
  "install.windows.title": "rFirma para Windows",
  "install.windows.body":
    "Instalador para o teu usuario, sen permisos de administrador. Usa directamente o almacén de certificados de Windows (MS-CAPI / CNG) e, unha vez instalado, actualízase desde a propia aplicación: cada versión nova chega coa súa sinatura minisign, que se comproba antes de instalala.",
  "install.windows.download": "Descargar o instalador de Windows",
  "install.windows.note":
    'Verifica o que descargas: baixa <code>SHA256SUMS</code> da <a href="https://github.com/sgomez/rfirma/releases/latest" target="_blank" rel="noopener noreferrer">Release</a> e comproba que o hash do instalador coincide coa súa liña. O instalador non está asinado con Authenticode, así que SmartScreen avisarache ao abrilo.',
  "install.macos.title": "Soporte para macOS en preparación",
  "install.macos.body":
    "A versión nativa para macOS está en fase de desenvolvemento activo. Integrarase co Keychain de Apple e CryptoTokenKit para un acceso fluído e seguro ás identidades dixitais do sistema.",
  "install.macos.note":
    'Distribuirase como imaxe de disco <code>.dmg</code> e mediante fórmula de <code>Homebrew</code>. Podes seguir o avance do proxecto en <a href="https://github.com/sgomez/rfirma" target="_blank" rel="noopener noreferrer">GitHub</a>.',

  "transparency.kicker": "Transparencia",
  "transparency.title": "Todo o código, á vista",
  "transparency.body":
    "rFirma é software libre e auditable baixo licenza <strong>EUPL 1.2</strong>. O código e o motor criptográfico viven en dous repositorios públicos.",
  "transparency.repo1.title": "Aplicación e ponte FFI",
  "transparency.repo1.body":
    'En <a href="https://github.com/sgomez/rfirma" target="_blank" rel="noopener noreferrer">sgomez/rfirma</a> están a interface en Tauri (Rust + React), a ponte FFI (<code>rfirma-native-bridge</code>) e o empaquetado.',
  "transparency.repo2.title": "Motor criptográfico orixinal",
  "transparency.repo2.body":
    'A lóxica de sinatura consume os artefactos de <a href="https://github.com/ctt-gob-es/clienteafirma" target="_blank" rel="noopener noreferrer">ctt-gob-es/clienteafirma</a>, compilados nunha biblioteca nativa con GraalVM Native Image.',
  "transparency.key.title": "Clave de sinatura de paquetes",
  "transparency.key.body":
    'Clave pública en <a href="https://rfirma.sgomez.me/rfirma.asc">rfirma.asc</a>. Comproba a súa pegada tras descargala con <code>gpg --show-keys rfirma.asc</code>:',

  "footer.tagline":
    "Sinatura electrónica nativa para o escritorio, co motor criptográfico oficial e sen Java no teu equipo.",
  "footer.license": "Licenza EUPL 1.2",
  "footer.col.project": "Proxecto",
  "footer.repo": "Repositorio en GitHub",
  "footer.releases": "Releases",
  "footer.gpg": "Clave GPG pública",
  "footer.col.origin": "Orixe",
  "footer.clienteafirma": "clienteafirma",
  "footer.comparison": "Diferenzas con AutoFirma",

  "notFound.title": "Páxina non atopada — rFirma",
  "notFound.description": "O enderezo que abriches non existe en rfirma.sgomez.me.",
  "notFound.heading": "Esta páxina non existe",
  "notFound.body": "Pode que a ligazón estea mal escrita ou que a páxina cambiase de sitio.",
  "notFound.home": "Volver á portada",
};
