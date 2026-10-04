import type { Dictionary } from "./es";

export const ca: Dictionary = {
  "meta.title": "rFirma — Signatura electrònica nativa per a l'escriptori",
  "meta.description":
    "Aplicació d'escriptori en Rust i React que substitueix la interfície Swing i els servidors locals d'AutoFirma, amb la criptografia oficial de l'Administració.",
  "meta.image.alt": "Logotip de rFirma sobre fons verd amb el domini rfirma.sgomez.me",

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

  "hero.kicker": "Una alternativa a AutoFirma",
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

  "how.kicker": "Pas a pas",
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
    "El PIN es demana en un diàleg natiu, no en una finestra web, i s'esborra de la memòria en acabar la signatura.",
  "how.step4.title": "Signatura completada",
  "how.step4.body": "El PDF signat es desa al costat de l'original, i el resum n'indica el format.",
  "how.step1.alt": "rFirma amb un PDF obert que ja porta una signatura vàlida",
  "how.step2.alt": "Selector de certificat obert, amb el cercador i els certificats disponibles",
  "how.step3.alt": "Diàleg natiu per introduir el PIN del certificat",
  "how.step4.alt": "PDF signat en PAdES, amb el resum de la signatura",

  "pillars.kicker": "Per què rFirma",
  "pillars.title": "Quatre decisions de disseny",
  "pillars.body":
    "Substitueix la interfície Swing i els servidors locals d'AutoFirma; el motor criptogràfic és el mateix, el de clienteafirma.",
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
  "comparison.title": "AutoFirma versus rFirma",
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
  "comparison.pin.label": "Protecció del PIN en memòria",
  "comparison.pin.autofirma": "S'esborra només en part i pot acabar al disc",
  "comparison.pin.rfirma": "Protegit a la memòria, no va mai al disc i s'esborra després d'usar-lo",
  "comparison.dnie.label": "Signatura amb DNIe",
  "comparison.dnie.autofirma": "Sí, amb jMulticard",
  "comparison.dnie.rfirma": "En desenvolupament",
  "comparison.store.label": "Certificats en fitxer (<code>.p12</code>)",
  "comparison.store.autofirma":
    "Es registra la ruta del fitxer en un diàleg de magatzems amb sis opcions",
  "comparison.store.rfirma": "Magatzem propi i xifrat, que s'obre amb la teva sessió",
  "comparison.stores.label": "Cerca de certificats",
  "comparison.stores.autofirma": "Només cerca al magatzem que triïs",
  "comparison.stores.rfirma": "Tots en una llista amb cercador, un certificat per fila",
  "comparison.stamp.label": "Col·locació de la signatura visible",
  "comparison.stamp.autofirma": "Coordenades o requadre sense context",
  "comparison.stamp.rfirma":
    "Arrossegament sobre la pàgina, amb models de text i previsualització fidel",
  "comparison.ca.label": "Confiança del navegador en el servidor local",
  "comparison.ca.autofirma": "L'instal·lador registra la CA al sistema, amb privilegis",
  "comparison.ca.rfirma":
    "L'aplicació registra la seva CA als magatzems NSS de la persona, sense root",
  "comparison.lang.label": "Idiomes",
  "comparison.lang.autofirma": "Només castellà, amb les cadenes dels diàlegs Swing",
  "comparison.lang.rfirma":
    "Castellà, català, euskara, gallec i anglès, amb catàleg propi i canvi des de Preferències",
  "comparison.privacy.label": "Gestor de documents",
  "comparison.privacy.autofirma": "—",
  "comparison.privacy.rfirma": "Recorda els documents recents i l'últim certificat utilitzat",
  "comparison.os.label": "Sistemes operatius",
  "comparison.os.autofirma": "Windows, macOS, Linux, Android i iOS",
  "comparison.os.rfirma": "Linux i Windows; macOS, en desenvolupament",
  "comparison.updates.label": "Canal d'actualització",
  "comparison.updates.autofirma": "Descàrrega manual de <code>.deb</code> o <code>.rpm</code>",
  "comparison.updates.rfirma":
    "Repositoris natius: APT, DNF i Flatpak; a Windows, des de la mateixa aplicació",
  "comparison.desktop.label": "Integració amb l'escriptori",
  "comparison.desktop.autofirma": "Aparença pròpia de Swing",
  "comparison.desktop.rfirma": "Segueix l'estil de l'escriptori, amb tema clar i fosc",

  "install.kicker": "Instal·lació",
  "install.title": "Un repositori per al teu sistema operatiu",
  "install.body":
    "Els canals de rFirma són repositoris natius. Un cop afegit el del teu sistema, els pedaços de seguretat s'instal·len amb el gestor de paquets.",
  "install.tablist.aria": "Canals de distribució",
  "install.copy": "Copiar",
  "install.copied": "Copiat",
  "install.copy.flatpak.aria": "Copiar l'ordre de Flatpak",
  "install.copy.apt.aria": "Copiar les ordres per a APT",
  "install.copy.dnf.aria": "Copiar les ordres per a DNF",
  "install.flatpak.body":
    "Recomanada per a les distribucions de Linux que no fan servir APT ni DNF. Es resol des del remot ostree propi de rFirma, i el runtime <code>org.gnome.Platform</code> es descarrega de Flathub sense configurar res. Només cal tenir instal·lats <code>flatpak</code> i <code>xdg-desktop-portal</code>.",
  "install.flatpak.tip":
    'També pots descarregar i instal·lar amb doble clic el fitxer <a href="https://rfirma.sgomez.me/rfirma.flatpakref">rfirma.flatpakref</a> si el teu escriptori ho admet.',
  "install.apt.body":
    "Per a Debian, Ubuntu i distribucions derivades. Configura el repositori mitjançant el format modern <code>deb822</code> amb la clau GPG verificada a <code>/usr/share/keyrings/</code>.",
  "install.dnf.body":
    "Per a Fedora i derivades basades en paquets RPM. Configura el repositori amb comprovació criptogràfica estricta de metadades i paquets (<code>gpgcheck=1</code> i <code>repo_gpgcheck=1</code>).",
  "install.soon": "En desenvolupament",
  "install.windows.title": "rFirma per a Windows",
  "install.windows.body":
    "Instal·lador per al teu usuari, sense permisos d'administrador. Fa servir directament l'emmagatzem de certificats de Windows (MS-CAPI / CNG) i, un cop instal·lat, s'actualitza des de la mateixa aplicació: cada versió nova arriba amb la seua signatura minisign, que es comprova abans d'instal·lar-la.",
  "install.windows.download": "Descarrega l'instal·lador de Windows",
  "install.windows.note":
    'Verifica el que descarregues: baixa <code>SHA256SUMS</code> de la <a href="https://github.com/sgomez/rfirma/releases/latest" target="_blank" rel="noopener noreferrer">Release</a> i comprova que el hash de l\'instal·lador coincideix amb la seua línia. L\'instal·lador no està signat amb Authenticode, així que SmartScreen t\'avisarà en obrir-lo.',
  "install.macos.title": "Suport per a macOS en preparació",
  "install.macos.body":
    "La versió nativa per a macOS està en fase de desenvolupament actiu. S'integrarà amb el Keychain d'Apple i CryptoTokenKit per a un accés fluid i segur a les identitats digitals del sistema.",
  "install.macos.note":
    'Es distribuirà com a imatge de disc <code>.dmg</code> i mitjançant fórmula de <code>Homebrew</code>. Pots seguir l\'avenç del projecte a <a href="https://github.com/sgomez/rfirma" target="_blank" rel="noopener noreferrer">GitHub</a>.',

  "transparency.kicker": "Transparència",
  "transparency.title": "Tot el codi, a la vista",
  "transparency.body":
    "rFirma és programari lliure i auditable sota llicència <strong>EUPL 1.2</strong>. El codi i el motor criptogràfic viuen en dos repositoris públics.",
  "transparency.repo1.title": "Aplicació i pont FFI",
  "transparency.repo1.body":
    'A <a href="https://github.com/sgomez/rfirma" target="_blank" rel="noopener noreferrer">sgomez/rfirma</a> hi ha la interfície en Tauri (Rust + React), el pont FFI (<code>rfirma-native-bridge</code>) i l\'empaquetament.',
  "transparency.repo2.title": "Motor criptogràfic original",
  "transparency.repo2.body":
    'La lògica de signatura consumeix els artefactes de <a href="https://github.com/ctt-gob-es/clienteafirma" target="_blank" rel="noopener noreferrer">ctt-gob-es/clienteafirma</a>, compilats en una biblioteca nativa amb GraalVM Native Image.',
  "transparency.key.title": "Clau de signatura de paquets",
  "transparency.key.body":
    'Clau pública a <a href="https://rfirma.sgomez.me/rfirma.asc">rfirma.asc</a>. Comprova la seva empremta després de descarregar-la amb <code>gpg --show-keys rfirma.asc</code>:',

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

  "notFound.title": "Pàgina no trobada — rFirma",
  "notFound.description": "L'adreça que has obert no existeix a rfirma.sgomez.me.",
  "notFound.heading": "Aquesta pàgina no existeix",
  "notFound.body": "Potser l'enllaç està mal escrit o la pàgina ha canviat de lloc.",
  "notFound.home": "Tornar a la portada",
};
