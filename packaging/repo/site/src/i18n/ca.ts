import type { Dictionary } from "./es";

export const ca: Dictionary = {
  "meta.title": "rFirma — Signatura electrònica nativa per a l'escriptori",
  "meta.description":
    "Aplicació d'escriptori en Rust i React que substitueix la interfície Swing i els servidors locals d'AutoFirma, amb la criptografia oficial de l'Administració.",

  "notice.aria": "Avís sobre titularitat oficial",
  "notice.badge": "Avís",
  "notice.official":
    "<strong>rFirma no és un producte oficial de l'Administració Pública.</strong> És un projecte independent de codi obert sota llicència EUPL 1.2.",

  "nav.aria": "Navegació principal",
  "nav.home.aria": "rFirma — inici",
  "nav.badge.alpha": "Alfa",
  "nav.how": "Així es signa",
  "nav.features": "Característiques",
  "nav.comparison": "Comparativa",
  "nav.install": "Instal·lació",
  "nav.transparency": "Transparència",
  "nav.github": "GitHub",

  "lang.aria": "Idioma",
  "lang.current": "Català",
  "lang.es": "Español",
  "lang.ca": "Català",
  "lang.eu": "Euskara",
  "lang.gl": "Galego",
  "lang.en": "English",

  "hero.kicker": "Alternativa independent a AutoFirma",
  "hero.title.line1": "Signatura electrònica nativa.",
  "hero.title.line2": "Sense Java, sense espera.",
  "hero.body":
    "Aplicació d'escriptori en Rust i React que substitueix la interfície Swing i els servidors locals d'AutoFirma, amb la criptografia oficial de l'Administració.",
  "hero.cta.primary": "Instal·lar rFirma",
  "hero.cta.secondary": "Veure com es signa",
  "hero.trust.oss": "Codi obert, EUPL 1.2",
  "hero.trust.keys": "La clau privada no surt del teu equip",
  "hero.trust.formats": "CAdES, PAdES, XAdES i FacturaE",
  "hero.trust.langs": "En cinc idiomes",
  "hero.window.title": "rFirma — Solicitud-subvencion.pdf",
  "hero.screenshot.alt":
    "Finestra principal de rFirma: visor del document, col·locació de la signatura visible i tauler lateral de signatura",
  "hero.card.cert.label": "Certificat",
  "hero.card.cert.value": "ADA LOVELACE BYRON",
  "hero.card.cert.issuer": "AC FNMT Usuarios",
  "hero.card.phase.label": "Fase",
  "hero.card.phase.value": "Muntant",
  "hero.card.stamp.label": "Signatura visible",
  "hero.card.stamp.value": "Col·locada a la pàgina 3",
  "hero.card.format.value": "PAdES · 2 signatures",

  "formats.aria": "Formats i magatzems compatibles",

  "how.kicker": "Cinc pantalles reals",
  "how.title": "Així es signa un document",
  "how.tablist.aria": "Passos de la signatura",
  "how.step1.title": "Document carregat",
  "how.step1.body":
    "rFirma obre el PDF i mostra el que conté: pàgines, mida i les signatures que ja porta. Si n'hi ha una de prèvia, la teva serà una cosignatura.",
  "how.step2.title": "Triar certificat",
  "how.step2.body":
    "La llista es compon amb els magatzems del sistema, els perfils del navegador i els mòduls PKCS#11. Els caducats i revocats es mostren, però no es poden fer servir.",
  "how.step3.title": "Introduir el PIN",
  "how.step3.body":
    "El PIN el demana el magatzem de certificats, no rFirma. La clau privada roman dins d'ell durant tota l'operació.",
  "how.step4.title": "Signant",
  "how.step4.body":
    "Presignatura, signatura i muntatge del PDF. Cada fase queda a la vista mentre es completa.",
  "how.step5.title": "Signatura completada",
  "how.step5.body":
    "El PDF signat es desa al costat de l'original. El resum indica el format PAdES i totes les signatures del document.",

  "mock.aria": "Maqueta de la finestra de rFirma durant la signatura",
  "mock.badge.signed": "Signat",
  "mock.badge.unsigned": "Sense signar",
  "mock.dropzone": "Arrossega un PDF o prem per obrir-lo",
  "mock.recents": "Recents",
  "mock.recent1.time": "avui, 10:58",
  "mock.recent2.name": "Anexo-II-memoria.pdf",
  "mock.recent2.time": "avui, 09:40",
  "mock.recent3.name": "Contrato-alquiler.pdf",
  "mock.recent3.time": "ahir, 17:20",
  "mock.stamp.name": "ADA LOVELACE BYRON",
  "mock.stamp.id": "DNI 99999999R",
  "mock.stamp.date": "31/08/2026 11:04",
  "mock.pageOf": "de 27",
  "mock.zoom": "100 %",
  "mock.file.unsigned": "Solicitud-subvencion.pdf",
  "mock.file.signed": "Solicitud-subvencion-firmado.pdf",
  "mock.file.meta": "27 pàgines · 2,4 MB",
  "mock.panel.certificate": "Certificat",
  "mock.panel.summary": "Resum",
  "mock.chooseCert": "Triar certificat",
  "mock.cert.chosen.detail": "99999999R · AC FNMT Usuarios · Firefox",
  "mock.summary.format": "PAdES",
  "mock.summary.count": "2 signatures",
  "mock.summary.mine": "La teva",
  "mock.summary.sig1.detail": "99999999R · avui, 11:04",
  "mock.summary.sig2.name": "SERVICIO DE GESTION TRIBUTARIA",
  "mock.summary.sig2.detail": "Q2800001A · 26/08/2026 09:12",
  "mock.visible.title": "Signatura visible",
  "mock.visible.toggle": "Estampar un requadre de signatura al document",
  "mock.visible.hint.off": "S'activa en triar certificat",
  "mock.visible.hint.on": "Pàgina 3 · arrossega'l per col·locar-lo",
  "mock.saveIn": "Es desarà a",
  "mock.saveIn.dir": "…/Documentos/",
  "mock.action.sign": "Signar el document",
  "mock.action.open": "Obrir el PDF",
  "mock.certlist.available": "Disponibles",
  "mock.certlist.unusable": "No utilitzables",
  "mock.certlist.cert2.name": "GRACE HOPPER MURRAY",
  "mock.certlist.cert2.detail": "88888888T · AC Representación · Instal·lat a rFirma",
  "mock.certlist.cert3.name": "ALAN TURING MATHISON",
  "mock.certlist.cert3.detail": "77777777P · AC FNMT Usuarios · Chrome",
  "mock.certlist.cert3.state": "Revocat el 12 de gener de 2026",
  "mock.pin.title": "Introdueix el PIN",
  "mock.pin.subject": "ADA LOVELACE BYRON · 99999999R",
  "mock.pin.label": "PIN",
  "mock.pin.cancel": "Cancel·lar",
  "mock.pin.sign": "Signar",
  "mock.signing.title": "Signant el document…",
  "mock.signing.presign": "Preparant la signatura",
  "mock.signing.presign.note": "(presignatura)",
  "mock.signing.sign": "Signant",
  "mock.signing.assemble": "Muntant el PDF",
  "mock.signing.assemble.note": "(postsignatura)",

  "pillars.kicker": "Per què rFirma",
  "pillars.title": "Quatre decisions de fons",
  "pillars.body":
    "No és una capa de pintura sobre AutoFirma: és una altra arquitectura, amb el mateix motor criptogràfic.",
  "pillars.native.title": "Rendiment natiu",
  "pillars.native.body":
    "Escriptori en Tauri v2, Rust i React. Sense JVM i sense servidors locals a l'escolta.",
  "pillars.keys.title": "La clau privada no surt del sistema",
  "pillars.keys.body":
    "La signatura es fa al teu equip, a través dels magatzems del sistema i dels mòduls PKCS#11. Java no la veu mai.",
  "pillars.crypto.title": "Criptografia oficial",
  "pillars.crypto.body":
    "CAdES, PAdES, XAdES i FacturaE surten del codi de <code>clienteafirma</code>, compilat a binari natiu amb GraalVM.",
  "pillars.stamp.title": "Signatura visible arrossegable",
  "pillars.stamp.body":
    "Col·loca i dimensiona la rúbrica sobre la pàgina, amb previsualització fidel. Sense coordenades a cegues.",

  "comparison.kicker": "Comparativa",
  "comparison.title": "AutoFirma i rFirma, fet a fet",
  "comparison.body":
    "Només diferències comprovables al codi, a la interfície i a la distribució de cada projecte. Sense xifres que ningú ha mesurat.",
  "comparison.head.aspect": "Aspecte",
  "comparison.head.autofirma": "AutoFirma (oficial)",
  "comparison.head.rfirma": "rFirma",
  "comparison.arch.label": "Arquitectura",
  "comparison.arch.autofirma": "Java Swing sobre JVM",
  "comparison.arch.rfirma":
    "Tauri v2 (Rust + React) amb el motor de clienteafirma en GraalVM Native Image",
  "comparison.keys.label": "On es processa la clau privada",
  "comparison.keys.autofirma": "Al procés Java",
  "comparison.keys.rfirma": "Al magatzem del sistema o al mòdul PKCS#11; no en surt",
  "comparison.store.label": "Certificats en fitxer (<code>.p12</code>)",
  "comparison.store.autofirma":
    "Es registra la ruta del fitxer en un diàleg de magatzems amb sis opcions",
  "comparison.store.rfirma":
    "Magatzem propi: «Afegir…» copia el certificat i del fitxer no es desa res, ni la ruta",
  "comparison.stores.label": "Cerca de certificats",
  "comparison.stores.autofirma": "Cal triar un magatzem i només ensenya aquest",
  "comparison.stores.rfirma":
    "Escorcolla el sistema, els perfils del navegador, els mòduls PKCS#11 i el magatzem propi, i ho ajunta en una llista",
  "comparison.stamp.label": "Col·locació de la signatura visible",
  "comparison.stamp.autofirma": "Coordenades o requadre sense context",
  "comparison.stamp.rfirma": "Arrossegament sobre la pàgina amb previsualització fidel",
  "comparison.sede.label": "Signatura des d'una seu electrònica",
  "comparison.sede.autofirma": "Diàleg de selecció de certificat",
  "comparison.sede.rfirma":
    "Finestra de consentiment: qui demana, què se signa i amb quin certificat, abans de signar",
  "comparison.ca.label": "Confiança del navegador en el servidor local",
  "comparison.ca.autofirma": "L'instal·lador registra la CA al sistema, amb privilegis",
  "comparison.ca.rfirma": "L'aplicació registra la seva CA als magatzems NSS de la persona, sense root",
  "comparison.lang.label": "Idiomes",
  "comparison.lang.autofirma": "Castellà i cooficials, amb les cadenes dels diàlegs Swing",
  "comparison.lang.rfirma":
    "Castellà, català, euskara, gallec i anglès, amb catàleg propi i canvi des de Preferències",
  "comparison.privacy.label": "Privadesa",
  "comparison.privacy.autofirma": "—",
  "comparison.privacy.rfirma":
    "Recents i últim certificat es poden apagar i buidar; l'única connexió sortint és la comprovació de versió, i es pot apagar",
  "comparison.updates.label": "Canal d'actualització",
  "comparison.updates.autofirma": "Descàrrega manual de <code>.deb</code> o <code>.rpm</code>",
  "comparison.updates.rfirma": "Repositoris natius: Flatpak, APT i DNF",
  "comparison.desktop.label": "Integració amb l'escriptori",
  "comparison.desktop.autofirma": "Aparença pròpia de Swing",
  "comparison.desktop.rfirma":
    "Interfície que segueix les convencions de l'escriptori, tema clar i fosc, contrast AA",

  "install.kicker": "Instal·lació",
  "install.title": "Un repositori, i les actualitzacions arriben soles",
  "install.body":
    "Els canals de rFirma són repositoris natius. Un cop afegit el del teu sistema, els pedaços de seguretat s'instal·len amb el gestor de paquets.",
  "install.tablist.aria": "Canals de distribució",
  "install.copy": "Copiar",
  "install.copied": "Copiat",
  "install.copy.flatpak.aria": "Copiar l'ordre de Flatpak",
  "install.copy.apt.aria": "Copiar les ordres per a APT",
  "install.copy.dnf.aria": "Copiar les ordres per a DNF",
  "install.flatpak.body":
    "Instal·lació recomanada per a qualsevol distribució de Linux. Es resol des del remot ostree propi de rFirma. Requereix el runtime <code>org.gnome.Platform</code> de Flathub.",
  "install.flatpak.tip":
    "També pots descarregar i instal·lar amb doble clic el fitxer <a href=\"https://rfirma.sgomez.me/rfirma.flatpakref\">rfirma.flatpakref</a> si el teu escriptori ho admet.",
  "install.apt.body":
    "Per a Debian, Ubuntu i distribucions derivades. Configura el repositori mitjançant el format modern <code>deb822</code> amb la clau GPG verificada a <code>/usr/share/keyrings/</code>.",
  "install.dnf.body":
    "Per a Fedora i derivades basades en paquets RPM. Configura el repositori amb comprovació criptogràfica estricta de metadades i paquets (<code>gpgcheck=1</code> i <code>repo_gpgcheck=1</code>).",
  "install.soon": "En desenvolupament",
  "install.windows.title": "Suport per a Windows en preparació",
  "install.windows.body":
    "La integració nativa per a Windows es troba actualment en desenvolupament. Farà servir directament el magatzem de certificats de Windows (MS-CAPI / CNG) per signar amb el teu certificat personal o DNIe sense necessitat de programari intermedi.",
  "install.windows.note":
    "Estarà disponible com a instal·lador <code>.msi</code> i a través de <code>winget</code>. Pots seguir l'avenç del projecte a <a href=\"https://github.com/sgomez/rfirma\" target=\"_blank\" rel=\"noopener noreferrer\">GitHub</a>.",
  "install.macos.title": "Suport per a macOS en preparació",
  "install.macos.body":
    "La versió nativa per a macOS està en fase de desenvolupament actiu. S'integrarà amb el Keychain d'Apple i CryptoTokenKit per a un accés fluid i segur a les identitats digitals del sistema.",
  "install.macos.note":
    "Es distribuirà com a imatge de disc <code>.dmg</code> i mitjançant fórmula de <code>Homebrew</code>. Pots seguir l'avenç del projecte a <a href=\"https://github.com/sgomez/rfirma\" target=\"_blank\" rel=\"noopener noreferrer\">GitHub</a>.",

  "transparency.kicker": "Transparència",
  "transparency.title": "Tot el codi, a la vista",
  "transparency.body":
    "rFirma és programari lliure i auditable sota llicència <strong>EUPL 1.2</strong>. El codi i el motor criptogràfic viuen en dos repositoris públics.",
  "transparency.repo1.title": "Aplicació i pont FFI",
  "transparency.repo1.body":
    "A <a href=\"https://github.com/sgomez/rfirma\" target=\"_blank\" rel=\"noopener noreferrer\">sgomez/rfirma</a> hi ha la interfície en Tauri (Rust + React), el pont FFI (<code>rfirma-native-bridge</code>) i l'empaquetament.",
  "transparency.repo2.title": "Motor criptogràfic original",
  "transparency.repo2.body":
    "La lògica de signatura consumeix els artefactes de <a href=\"https://github.com/ctt-gob-es/clienteafirma\" target=\"_blank\" rel=\"noopener noreferrer\">ctt-gob-es/clienteafirma</a>, compilats en una biblioteca nativa amb GraalVM Native Image.",
  "transparency.key.title": "Clau de signatura de paquets",
  "transparency.key.body":
    "Clau pública a <a href=\"https://rfirma.sgomez.me/rfirma.asc\">rfirma.asc</a>. Comprova la seva empremta després de descarregar-la amb <code>gpg --show-keys rfirma.asc</code>:",

  "footer.tagline":
    "Signatura electrònica nativa per a l'escriptori, amb el motor criptogràfic oficial i sense Java al teu equip.",
  "footer.license": "Llicència EUPL 1.2",
  "footer.col.project": "Projecte",
  "footer.repo": "Repositori a GitHub",
  "footer.releases": "Releases",
  "footer.gpg": "Clau GPG pública",
  "footer.col.origin": "Origen",
  "footer.clienteafirma": "clienteafirma",
  "footer.comparison": "Diferències amb AutoFirma",
};
