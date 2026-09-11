import type { Dictionary } from "./es";

export const gl: Dictionary = {
  "meta.title": "rFirma — Sinatura electrónica nativa para o escritorio",
  "meta.description":
    "Aplicación de escritorio en Rust e React que substitúe a interface Swing e os servidores locais de AutoFirma, coa criptografía oficial da Administración.",
  "meta.image.alt":
    "Logotipo de rFirma sobre fondo verde co dominio rfirma.sgomez.me",

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

  "hero.kicker": "Alternativa independente a AutoFirma",
  "hero.title.line1": "Sinatura electrónica nativa.",
  "hero.title.line2": "Sen Java, sen esperas.",
  "hero.body":
    "Aplicación de escritorio en Rust e React que substitúe a interface Swing e os servidores locais de AutoFirma, coa criptografía oficial da Administración.",
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

  "how.kicker": "Cinco pantallas reais",
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
    "O PIN pídeo o almacén de certificados, non rFirma. A clave privada permanece dentro del durante toda a operación.",
  "how.step4.title": "Asinando",
  "how.step4.body":
    "Presinatura, sinatura e ensamblado do PDF. Cada fase queda á vista mentres se completa.",
  "how.step5.title": "Sinatura completada",
  "how.step5.body":
    "O PDF asinado gárdase xunto ao orixinal. O resumo indica o formato PAdES e todas as sinaturas do documento.",

  "mock.aria": "Maqueta da xanela de rFirma durante a sinatura",
  "mock.badge.signed": "Asinado",
  "mock.badge.unsigned": "Sen asinar",
  "mock.dropzone": "Arrastra un PDF ou preme para abrilo",
  "mock.recents": "Recentes",
  "mock.recent1.time": "hoxe, 10:58",
  "mock.recent2.name": "Anexo-II-memoria.pdf",
  "mock.recent2.time": "hoxe, 09:40",
  "mock.recent3.name": "Contrato-alquiler.pdf",
  "mock.recent3.time": "onte, 17:20",
  "mock.stamp.name": "ADA LOVELACE BYRON",
  "mock.stamp.id": "DNI 99999999R",
  "mock.stamp.date": "31/08/2026 11:04",
  "mock.pageOf": "de 27",
  "mock.zoom": "100 %",
  "mock.file.unsigned": "Solicitud-subvencion.pdf",
  "mock.file.signed": "Solicitud-subvencion-firmado.pdf",
  "mock.file.meta": "27 páxinas · 2,4 MB",
  "mock.panel.certificate": "Certificado",
  "mock.panel.summary": "Resumo",
  "mock.chooseCert": "Escoller certificado",
  "mock.cert.chosen.detail": "99999999R · AC FNMT Usuarios · Firefox",
  "mock.summary.format": "PAdES",
  "mock.summary.count": "2 sinaturas",
  "mock.summary.mine": "A túa",
  "mock.summary.sig1.detail": "99999999R · hoxe, 11:04",
  "mock.summary.sig2.name": "SERVICIO DE GESTION TRIBUTARIA",
  "mock.summary.sig2.detail": "Q2800001A · 26/08/2026 09:12",
  "mock.visible.title": "Sinatura visible",
  "mock.visible.toggle": "Estampar un cadro de sinatura no documento",
  "mock.visible.hint.off": "Actívase ao escoller certificado",
  "mock.visible.hint.on": "Páxina 3 · arrástrao para colocalo",
  "mock.saveIn": "Gardarase en",
  "mock.saveIn.dir": "…/Documentos/",
  "mock.action.sign": "Asinar documento",
  "mock.action.open": "Abrir o PDF",
  "mock.certlist.available": "Dispoñibles",
  "mock.certlist.unusable": "Non utilizables",
  "mock.certlist.cert2.name": "GRACE HOPPER MURRAY",
  "mock.certlist.cert2.detail": "88888888T · AC Representación · Instalado en rFirma",
  "mock.certlist.cert3.name": "ALAN TURING MATHISON",
  "mock.certlist.cert3.detail": "77777777P · AC FNMT Usuarios · Chrome",
  "mock.certlist.cert3.state": "Revogado o 12 de xaneiro de 2026",
  "mock.pin.title": "Introduce o PIN",
  "mock.pin.subject": "ADA LOVELACE BYRON · 99999999R",
  "mock.pin.label": "PIN",
  "mock.pin.cancel": "Cancelar",
  "mock.pin.sign": "Asinar",
  "mock.signing.title": "Asinando o documento…",
  "mock.signing.presign": "Preparando a sinatura",
  "mock.signing.presign.note": "(presinatura)",
  "mock.signing.sign": "Asinando",
  "mock.signing.assemble": "Ensamblando o PDF",
  "mock.signing.assemble.note": "(postsinatura)",

  "pillars.kicker": "Por que rFirma",
  "pillars.title": "Catro decisións de fondo",
  "pillars.body":
    "Non é unha capa de pintura sobre AutoFirma: é outra arquitectura, co mesmo motor criptográfico.",
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
  "comparison.title": "AutoFirma e rFirma, feito por feito",
  "comparison.body":
    "Só diferenzas comprobables no código, na interface e na distribución de cada proxecto. Sen cifras que ninguén mediu.",
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
  "comparison.pin.autofirma":
    "Borrado parcial do <code>char[]</code> con <code>Arrays.fill</code>, sen fixalo na RAM",
  "comparison.pin.rfirma":
    "Búfer fixado con <code>mlock</code>, excluído dos volcados con <code>MADV_DONTDUMP</code> e borrado de forma segura",
  "comparison.store.label": "Certificados en ficheiro (<code>.p12</code>)",
  "comparison.store.autofirma":
    "Rexístrase a ruta do ficheiro nun diálogo de almacéns con seis opcións",
  "comparison.store.rfirma":
    "Almacén propio: «Engadir…» copia o certificado e do ficheiro non se garda nada, nin a ruta",
  "comparison.stores.label": "Busca de certificados",
  "comparison.stores.autofirma": "Hai que escoller un almacén e só amosa ese",
  "comparison.stores.rfirma":
    "Percorre o sistema, os perfís do navegador, os módulos PKCS#11 e o almacén propio, e xúntao nunha lista",
  "comparison.stamp.label": "Colocación da sinatura visible",
  "comparison.stamp.autofirma": "Coordenadas ou cadro sen contexto",
  "comparison.stamp.rfirma": "Arrastre sobre a páxina con previsualización fiel",
  "comparison.sede.label": "Sinatura desde unha sede electrónica",
  "comparison.sede.autofirma": "Diálogo de selección de certificado",
  "comparison.sede.rfirma":
    "Xanela de consentimento: quen pide, que se asina e con que certificado, antes de asinar",
  "comparison.ca.label": "Confianza do navegador no servidor local",
  "comparison.ca.autofirma": "O instalador rexistra a CA no sistema, con privilexios",
  "comparison.ca.rfirma": "A aplicación rexistra a súa CA nos almacéns NSS da persoa, sen root",
  "comparison.lang.label": "Idiomas",
  "comparison.lang.autofirma": "Castelán e cooficiais, coas cadeas dos diálogos Swing",
  "comparison.lang.rfirma":
    "Castelán, català, euskara, galego e inglés, con catálogo propio e cambio desde Preferencias",
  "comparison.privacy.label": "Privacidade",
  "comparison.privacy.autofirma": "—",
  "comparison.privacy.rfirma":
    "Recentes e último certificado pódense apagar e baleirar; a única conexión saínte é a comprobación de versión, e pódese apagar",
  "comparison.updates.label": "Canle de actualización",
  "comparison.updates.autofirma": "Descarga manual de <code>.deb</code> ou <code>.rpm</code>",
  "comparison.updates.rfirma": "Repositorios nativos: Flatpak, APT e DNF",
  "comparison.desktop.label": "Integración co escritorio",
  "comparison.desktop.autofirma": "Aparencia propia de Swing",
  "comparison.desktop.rfirma":
    "Interface que segue as convencións do escritorio, tema claro e escuro, contraste AA",

  "install.kicker": "Instalación",
  "install.title": "Un repositorio, e as actualizacións chegan soas",
  "install.body":
    "As canles de rFirma son repositorios nativos. Unha vez engadido o do teu sistema, os parches de seguridade instálanse co xestor de paquetes.",
  "install.tablist.aria": "Canles de distribución",
  "install.copy": "Copiar",
  "install.copied": "Copiado",
  "install.copy.flatpak.aria": "Copiar orde de Flatpak",
  "install.copy.apt.aria": "Copiar ordes para APT",
  "install.copy.dnf.aria": "Copiar ordes para DNF",
  "install.flatpak.body":
    "Instalación recomendada para calquera distribución de Linux. Resólvese desde o remoto ostree propio de rFirma. Require o runtime <code>org.gnome.Platform</code> de Flathub.",
  "install.flatpak.tip":
    "Tamén podes descargar e instalar con dobre clic o ficheiro <a href=\"https://rfirma.sgomez.me/rfirma.flatpakref\">rfirma.flatpakref</a> se o teu escritorio o soporta.",
  "install.apt.body":
    "Para Debian, Ubuntu e distribucións derivadas. Configura o repositorio mediante o formato moderno <code>deb822</code> coa clave GPG verificada en <code>/usr/share/keyrings/</code>.",
  "install.dnf.body":
    "Para Fedora e derivadas baseadas en paquetes RPM. Configura o repositorio con comprobación criptográfica estrita de metadatos e paquetes (<code>gpgcheck=1</code> e <code>repo_gpgcheck=1</code>).",
  "install.soon": "En desenvolvemento",
  "install.windows.title": "Soporte para Windows en preparación",
  "install.windows.body":
    "A integración nativa para Windows atópase actualmente en desenvolvemento. Utilizará directamente o almacén de certificados de Windows (MS-CAPI / CNG) para asinar co teu certificado persoal ou DNIe sen necesidade de software intermedio.",
  "install.windows.note":
    "Estará dispoñible como instalador <code>.msi</code> e a través de <code>winget</code>. Podes seguir o avance do proxecto en <a href=\"https://github.com/sgomez/rfirma\" target=\"_blank\" rel=\"noopener noreferrer\">GitHub</a>.",
  "install.macos.title": "Soporte para macOS en preparación",
  "install.macos.body":
    "A versión nativa para macOS está en fase de desenvolvemento activo. Integrarase co Keychain de Apple e CryptoTokenKit para un acceso fluído e seguro ás identidades dixitais do sistema.",
  "install.macos.note":
    "Distribuirase como imaxe de disco <code>.dmg</code> e mediante fórmula de <code>Homebrew</code>. Podes seguir o avance do proxecto en <a href=\"https://github.com/sgomez/rfirma\" target=\"_blank\" rel=\"noopener noreferrer\">GitHub</a>.",

  "transparency.kicker": "Transparencia",
  "transparency.title": "Todo o código, á vista",
  "transparency.body":
    "rFirma é software libre e auditable baixo licenza <strong>EUPL 1.2</strong>. O código e o motor criptográfico viven en dous repositorios públicos.",
  "transparency.repo1.title": "Aplicación e ponte FFI",
  "transparency.repo1.body":
    "En <a href=\"https://github.com/sgomez/rfirma\" target=\"_blank\" rel=\"noopener noreferrer\">sgomez/rfirma</a> están a interface en Tauri (Rust + React), a ponte FFI (<code>rfirma-native-bridge</code>) e o empaquetado.",
  "transparency.repo2.title": "Motor criptográfico orixinal",
  "transparency.repo2.body":
    "A lóxica de sinatura consume os artefactos de <a href=\"https://github.com/ctt-gob-es/clienteafirma\" target=\"_blank\" rel=\"noopener noreferrer\">ctt-gob-es/clienteafirma</a>, compilados nunha biblioteca nativa con GraalVM Native Image.",
  "transparency.key.title": "Clave de sinatura de paquetes",
  "transparency.key.body":
    "Clave pública en <a href=\"https://rfirma.sgomez.me/rfirma.asc\">rfirma.asc</a>. Comproba a súa pegada tras descargala con <code>gpg --show-keys rfirma.asc</code>:",

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
};
