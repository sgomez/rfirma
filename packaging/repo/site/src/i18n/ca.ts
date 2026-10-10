import type { Dictionary } from "./es";

export const ca: Dictionary = {
  "meta.title": "rFirma: signatura electrònica sense Java, alternativa a AutoFirma",
  "meta.description":
    "Signa PDF amb el teu certificat digital o DNIe a Linux, Windows i macOS. Funciona a les seus que demanen AutoFirma, sense Java. Versió alfa.",
  "meta.image.alt": "Logotip de rFirma sobre fons verd amb el domini rfirma.sgomez.me",

  "notice.aria": "Avís sobre titularitat oficial",
  "notice.badge": "Avís",
  "notice.official":
    "<strong>rFirma no és un producte oficial de l'Administració Pública.</strong> És un projecte independent de codi obert sota llicència EUPL 1.2.",

  "nav.aria": "Navegació principal",
  "nav.home.aria": "rFirma — inici",
  "nav.badge.alpha": "Alfa",
  "nav.how": "Així es signa",
  "nav.comparison": "Comparativa",
  "nav.install": "Instal·lació",
  "nav.manual": "Manual",
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
  "hero.title.line1": "rFirma: signatura electrònica nativa,",
  "hero.title.line2": "sense Java.",
  "hero.body":
    "Sense espera. Aplicació d'escriptori en Rust i React que substitueix la interfície Swing i els servidors locals d'AutoFirma, amb la criptografia oficial de l'Administració.",
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
  "comparison.head.autofirma": "AutoFirma",
  "comparison.head.rfirma": "rFirma",
  "comparison.java.label": "Java",
  "comparison.java.autofirma": "Cal tenir-lo",
  "comparison.java.rfirma": "No el fa servir",
  "comparison.pin.label": "Introducció del PIN a Linux",
  "comparison.pin.autofirma": "Finestra de Java",
  "comparison.pin.rfirma": "Diàleg del sistema i memòria protegida",
  "comparison.dnie.label": "Signatura amb DNIe",
  "comparison.dnie.autofirma": "Sí, amb jMulticard",
  "comparison.dnie.rfirma": "Sí, el detecta en inserir-lo",
  "comparison.store.label": "Certificats <code>.p12</code>",
  "comparison.store.autofirma": "Tries el fitxer en un diàleg",
  "comparison.store.rfirma": "Magatzem xifrat propi",
  "comparison.stores.label": "Cerca de certificats",
  "comparison.stores.autofirma": "Per magatzems, d'un en un",
  "comparison.stores.rfirma": "Tots alhora, amb cercador",
  "comparison.stamp.label": "Signatura visible",
  "comparison.stamp.autofirma": "Amb coordenades",
  "comparison.stamp.rfirma": "L'arrossegues sobre la pàgina",
  "comparison.lang.label": "Idiomes",
  "comparison.lang.autofirma": "Castellà",
  "comparison.lang.rfirma": "Castellà, català, basc, gallec i anglès",
  "comparison.os.label": "Sistemes operatius",
  "comparison.os.autofirma": "Windows, macOS, Linux, Android i iOS",
  "comparison.os.rfirma": "Linux i Windows",
  "comparison.updates.label": "Actualitzacions",
  "comparison.updates.autofirma": "Descàrrega manual",
  "comparison.updates.rfirma": "Automàtiques",
  "comparison.desktop.label": "Aparença",
  "comparison.desktop.autofirma": "Finestres de Java",
  "comparison.desktop.rfirma": "Les del teu escriptori, amb tema clar i fosc",

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
    "Recomanada per a les distribucions de Linux que no fan servir APT ni DNF. Dones d'alta Flathub, d'on surt el runtime <code>org.gnome.Platform</code>, i el remot de rFirma, com un repositori més: així rFirma també apareix en cercar a GNOME Software. Només cal tenir instal·lats <code>flatpak</code> i <code>xdg-desktop-portal</code>.",
  "install.apt.body":
    "Per a Debian, Ubuntu i distribucions derivades. Configura el repositori mitjançant el format modern <code>deb822</code> amb la clau GPG verificada a <code>/usr/share/keyrings/</code>.",
  "install.dnf.body":
    "Per a Fedora i derivades basades en paquets RPM. Configura el repositori amb comprovació criptogràfica estricta de metadades i paquets (<code>gpgcheck=1</code> i <code>repo_gpgcheck=1</code>).",
  "install.manual":
    "Instruccions detallades, empremta de la clau i verificació, al manual (en castellà)",
  "install.uninstall":
    "Per no deixar el certificat de rFirma al navegador, prem <em>Retira el certificat</em> al panell d'estat abans de desinstal·lar.",
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
