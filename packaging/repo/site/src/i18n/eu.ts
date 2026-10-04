import type { Dictionary } from "./es";

export const eu: Dictionary = {
  "meta.title": "rFirma: sinadura elektronikoa Javarik gabe, AutoFirmaren alternatiba",
  "meta.description":
    "Sinatu PDFak zure ziurtagiri digitalarekin edo NANe elektronikoarekin Linux, Windows eta macOS sistemetan. AutoFirma eskatzen duten egoitzetan dabil, Javarik gabe. Alfa bertsioa.",
  "meta.image.alt": "rFirmaren logotipoa hondo berdearen gainean, rfirma.sgomez.me domeinuarekin",

  "notice.aria": "Titulartasun ofizialari buruzko oharra",
  "notice.badge": "Oharra",
  "notice.official":
    "<strong>rFirma ez da Administrazio Publikoaren produktu ofiziala.</strong> Kode irekiko proiektu independentea da, EUPL 1.2 lizentziapean.",

  "nav.aria": "Nabigazio nagusia",
  "nav.home.aria": "rFirma — hasiera",
  "nav.badge.alpha": "Alfa",
  "nav.how": "Horrela sinatzen da",
  "nav.features": "Ezaugarriak",
  "nav.comparison": "Konparaketa",
  "nav.install": "Instalazioa",
  "nav.manual": "Eskuliburua",
  "nav.transparency": "Gardentasuna",
  "nav.github": "GitHub",

  "lang.aria": "Hizkuntza",
  "lang.current": "Euskara",
  "lang.es": "Español",
  "lang.ca": "Català",
  "lang.eu": "Euskara",
  "lang.gl": "Galego",
  "lang.en": "English",

  "hero.kicker": "AutoFirmaren alternatiba",
  "hero.title.line1": "rFirma: jatorrizko sinadura elektronikoa,",
  "hero.title.line2": "Javarik gabe.",
  "hero.body":
    "Itxaronaldirik gabe. AutoFirmaren Swing interfazea eta tokiko zerbitzariak ordezten dituen Rust eta React aplikazioa, Administrazioaren kriptografia ofizialarekin.",
  "hero.cta.primary": "Instalatu rFirma",
  "hero.cta.secondary": "Ikusi nola sinatzen den",
  "hero.trust.oss": "Kode irekia, EUPL 1.2",
  "hero.trust.keys": "Zure gakoa ez da zure ordenagailutik ateratzen",
  "hero.trust.formats": "CAdES, PAdES, XAdES eta FacturaE",
  "hero.trust.langs": "Bost hizkuntzatan",
  "hero.window.title": "rFirma — Solicitud-subvencion.pdf",
  "hero.screenshot.alt":
    "rFirmaren leiho nagusia: dokumentuaren ikustailea, sinadura ikusgaiaren kokapena eta sinadura-panel alboragarria",
  "hero.card.cert.label": "Ziurtagiria",
  "hero.card.cert.value": "ADA LOVELACE BYRON",
  "hero.card.cert.issuer": "AC FNMT Usuarios",
  "hero.card.phase.label": "Fasea",
  "hero.card.phase.value": "Muntatzen",
  "hero.card.stamp.label": "Sinadura ikusgaia",
  "hero.card.stamp.value": "3. orrialdean kokatuta",
  "hero.card.format.value": "PAdES · 2 sinadura",

  "formats.aria": "Formatu eta biltegi bateragarriak",

  "how.kicker": "Urratsez urrats",
  "how.title": "Horrela sinatzen da dokumentu bat",
  "how.tablist.aria": "Sinaduraren urratsak",
  "how.step1.title": "Dokumentua kargatuta",
  "how.step1.body":
    "rFirmak PDFa irekitzen du eta duena erakusten du: orrialdeak, tamaina eta dagoeneko dituen sinadurak. Aurretiko bat badago, zurea kosinadura izango da.",
  "how.step2.title": "Aukeratu ziurtagiria",
  "how.step2.body":
    "Zerrenda sistemaren biltegiekin, nabigatzailearen profilekin eta PKCS#11 moduluekin osatzen da. Iraungitakoak eta errebokatutakoak erakusten dira, baina ezin dira erabili.",
  "how.step3.title": "Sartu PINa",
  "how.step3.body":
    "PINa elkarrizketa-koadro natibo batean eskatzen da, ez web-leiho batean, eta sinadura amaitu bezain laster memoriatik ezabatzen da.",
  "how.step4.title": "Sinadura osatuta",
  "how.step4.body":
    "Sinatutako PDFa jatorrizkoaren ondoan gordetzen da, eta laburpenak bere formatua adierazten du.",
  "how.step1.alt": "rFirma, baliozko sinadura bat duen PDF bat irekita duela",
  "how.step2.alt":
    "Ziurtagiri-hautatzailea irekita, bilatzailearekin eta ziurtagiri erabilgarriekin",
  "how.step3.alt": "PINa sartzeko elkarrizketa-koadro natiboa",
  "how.step4.alt": "PAdES formatuan sinatutako PDFa, sinaduraren laburpenarekin",

  "pillars.kicker": "Zergatik rFirma",
  "pillars.title": "Lau diseinu-erabaki",
  "pillars.body":
    "AutoFirmaren Swing interfazea eta tokiko zerbitzariak ordezten ditu; motor kriptografikoa berdina da, clienteafirmarena.",
  "pillars.native.title": "Errendimendu nagusia",
  "pillars.native.body":
    "Mahaigainerako aplikazioa Tauri v2, Rust eta React erabiliz. JVMrik gabe eta entzuten dagoen tokiko zerbitzaririk gabe.",
  "pillars.keys.title": "Gakoa ez da sistematik ateratzen",
  "pillars.keys.body":
    "Sinadura zure ordenagailuan egiten da, sistemaren biltegien eta PKCS#11 moduluen bidez. Javak ez du inoiz ikusten.",
  "pillars.crypto.title": "Kriptografia ofiziala",
  "pillars.crypto.body":
    "CAdES, PAdES, XAdES eta FacturaE <code>clienteafirma</code>ren kodetik ateratzen dira, GraalVMrekin bitarra natibora konpilatuta.",
  "pillars.stamp.title": "Sinadura ikusgai arrastagarria",
  "pillars.stamp.body":
    "Kokatu eta neurtu errubrika orrialdearen gainean, aurrebista fidagarriarekin. Koordenatu itsurik gabe.",

  "comparison.kicker": "Konparaketa",
  "comparison.title": "AutoFirma eta rFirma, aurrez aurre",
  "comparison.head.aspect": "Alderdia",
  "comparison.head.autofirma": "AutoFirma (ofiziala)",
  "comparison.head.rfirma": "rFirma",
  "comparison.arch.label": "Arkitektura",
  "comparison.arch.autofirma": "Java Swing, JVMren gainean",
  "comparison.arch.rfirma":
    "Tauri v2 (Rust + React), clienteafirmaren motorra GraalVM Native Image bidez",
  "comparison.keys.label": "Non prozesatzen den gakoa",
  "comparison.keys.autofirma": "Java prozesuan",
  "comparison.keys.rfirma": "Sistemaren biltegian edo PKCS#11 moduluan; ez da handik ateratzen",
  "comparison.pin.label": "PINaren babesa memorian",
  "comparison.pin.autofirma": "Zati batean bakarrik ezabatzen da, eta diskora irits daiteke",
  "comparison.pin.rfirma":
    "Memorian babestuta, ez da inoiz diskora joaten eta erabili ondoren ezabatzen da",
  "comparison.dnie.label": "DNIe-arekin sinatzea",
  "comparison.dnie.autofirma": "Bai, jMulticard bidez",
  "comparison.dnie.rfirma": "Garapenean",
  "comparison.store.label": "Fitxategiko ziurtagiriak (<code>.p12</code>)",
  "comparison.store.autofirma":
    "Fitxategiaren bidea sei aukerako biltegi-elkarrizketa batean gordetzen da",
  "comparison.store.rfirma": "Biltegi propio eta zifratua, zure saioarekin irekitzen dena",
  "comparison.stores.label": "Ziurtagirien bilaketa",
  "comparison.stores.autofirma": "Aukeratutako biltegian bakarrik bilatzen du",
  "comparison.stores.rfirma":
    "Guztiak bilatzailedun zerrenda batean, ziurtagiri bat errenkada bakoitzeko",
  "comparison.stamp.label": "Sinadura ikusgaiaren kokapena",
  "comparison.stamp.autofirma": "Koordenatuak edo laukia, testuingururik gabe",
  "comparison.stamp.rfirma": "Orrira arrastatuta, testu-ereduekin eta aurrebista fidelarekin",
  "comparison.ca.label": "Nabigatzaileak tokiko zerbitzariaz duen konfiantza",
  "comparison.ca.autofirma": "Instalatzaileak CA sisteman erregistratzen du, pribilegioekin",
  "comparison.ca.rfirma":
    "Aplikazioak bere CA pertsonaren NSS biltegietan erregistratzen du, root gabe",
  "comparison.lang.label": "Hizkuntzak",
  "comparison.lang.autofirma": "Gaztelania soilik, Swing elkarrizketen kateekin",
  "comparison.lang.rfirma":
    "Gaztelania, katalana, euskara, galiziera eta ingelesa, norberaren katalogoarekin eta Hobespenetatik aldatuz",
  "comparison.privacy.label": "Dokumentu-kudeatzailea",
  "comparison.privacy.autofirma": "—",
  "comparison.privacy.rfirma":
    "Azken dokumentuak eta erabilitako azken ziurtagiria gogoratzen ditu",
  "comparison.os.label": "Sistema eragileak",
  "comparison.os.autofirma": "Windows, macOS, Linux, Android eta iOS",
  "comparison.os.rfirma": "Linux eta Windows; macOS, garapenean",
  "comparison.updates.label": "Eguneratze-bidea",
  "comparison.updates.autofirma": "<code>.deb</code> edo <code>.rpm</code> eskuz deskargatuta",
  "comparison.updates.rfirma":
    "Jatorrizko biltegiak: APT, DNF eta Flatpak; Windowsen, aplikaziotik bertatik",
  "comparison.desktop.label": "Mahaigainarekiko integrazioa",
  "comparison.desktop.autofirma": "Swingen itxura propioa",
  "comparison.desktop.rfirma": "Mahaigainaren estiloa jarraitzen du, gai argi eta ilunarekin",

  "install.kicker": "Instalazioa",
  "install.title": "Zure sistema eragilearentzako biltegia",
  "install.body":
    "rFirmaren bideak jatorrizko biltegiak dira. Zure sistemarena gehitu ondoren, segurtasun-adabakiak paketeen kudeatzailearekin instalatzen dira.",
  "install.tablist.aria": "Banaketa-bideak",
  "install.copy": "Kopiatu",
  "install.copied": "Kopiatuta",
  "install.copy.flatpak.aria": "Kopiatu Flatpaken agindua",
  "install.copy.apt.aria": "Kopiatu APTrako aginduak",
  "install.copy.dnf.aria": "Kopiatu DNFrako aginduak",
  "install.flatpak.body":
    "APT edo DNF erabiltzen ez duten Linux banaketentzat gomendatua. rFirmaren ostree biltegi propiotik ebazten da, eta Flathub-eko <code>org.gnome.Platform</code> runtimea ezer konfiguratu gabe deskargatzen da. <code>flatpak</code> eta <code>xdg-desktop-portal</code> instalatuta izatea besterik ez duzu behar.",
  "install.flatpak.tip":
    '<a href="https://rfirma.sgomez.me/rfirma.flatpakref">rfirma.flatpakref</a> fitxategia deskargatu eta klik bikoitzarekin ere insta dezakezu, zure mahaigainak onartzen badu.',
  "install.apt.body":
    "Debian, Ubuntu eta eratorritako banaketentzat. Biltegia <code>deb822</code> formatu modernoarekin konfiguratzen du, GPG gakoa <code>/usr/share/keyrings/</code> gunean egiaztatuta.",
  "install.dnf.body":
    "Fedora eta RPM paketeetan oinarritutako eratorrientzat. Biltegia metadatuen eta paketeen egiaztapen kriptografiko zorrotzarekin konfiguratzen du (<code>gpgcheck=1</code> eta <code>repo_gpgcheck=1</code>).",
  "install.soon": "Garatzen",
  "install.windows.title": "rFirma Windowsentzat",
  "install.windows.body":
    "Zure erabiltzailearentzako instalatzailea, administratzaile-baimenik gabe. Windowseko ziurtagiri-biltegia (MS-CAPI / CNG) zuzenean erabiltzen du eta, instalatu ondoren, aplikazioak berak eguneratzen du: bertsio berri bakoitza bere minisign sinadurarekin dator, eta instalatu aurretik egiaztatzen da.",
  "install.windows.download": "Deskargatu Windowseko instalatzailea",
  "install.windows.note":
    'Egiaztatu deskargatzen duzuna: jaitsi <code>SHA256SUMS</code> <a href="https://github.com/sgomez/rfirma/releases/latest" target="_blank" rel="noopener noreferrer">Release</a>-tik eta begiratu instalatzailearen hash-a bere lerroarekin bat datorrela. Instalatzailea ez dago Authenticode-rekin sinatuta, beraz SmartScreen-ek abisua emango dizu irekitzean.',
  "install.macos.title": "macOSentzako euskarria prestatzen",
  "install.macos.body":
    "macOSentzako bertsio natiboa garapen aktiboan dago. Apple-ren Keychain eta CryptoTokenKit-ekin integratuko da, sistemaren identitate digitaletara sarbide fluidoa eta segurua izateko.",
  "install.macos.note":
    '<code>.dmg</code> disko-irudi gisa eta <code>Homebrew</code> formularen bidez banatuko da. Proiektuaren aurrerapena <a href="https://github.com/sgomez/rfirma" target="_blank" rel="noopener noreferrer">GitHub</a>en jarrai dezakezu.',

  "transparency.kicker": "Gardentasuna",
  "transparency.title": "Kode osoa, ikusgai",
  "transparency.body":
    "rFirma software librea eta auditagarria da, <strong>EUPL 1.2</strong> lizentziapean. Kodea eta motor kriptografikoa bi biltegi publikotan bizi dira.",
  "transparency.repo1.title": "Aplikazioa eta FFI zubia",
  "transparency.repo1.body":
    '<a href="https://github.com/sgomez/rfirma" target="_blank" rel="noopener noreferrer">sgomez/rfirma</a>n daude Tauriko interfazea (Rust + React), FFI zubia (<code>rfirma-native-bridge</code>) eta enpaketatzea.',
  "transparency.repo2.title": "Jatorrizko motor kriptografikoa",
  "transparency.repo2.body":
    'Sinadura-logikak <a href="https://github.com/ctt-gob-es/clienteafirma" target="_blank" rel="noopener noreferrer">ctt-gob-es/clienteafirma</a>ren artefaktuak erabiltzen ditu, GraalVM Native Image bidez jatorrizko liburutegi batean konpilatuta.',
  "transparency.key.title": "Paketeak sinatzeko gakoa",
  "transparency.key.body":
    'Gako publikoa <a href="https://rfirma.sgomez.me/rfirma.asc">rfirma.asc</a>en. Egiaztatu bere hatz-marka deskargatu ondoren, <code>gpg --show-keys rfirma.asc</code> erabiliz:',

  "footer.tagline":
    "Mahaigainerako jatorrizko sinadura elektronikoa, motor kriptografiko ofizialarekin eta zure ordenagailuan Javarik gabe.",
  "footer.license": "EUPL 1.2 lizentzia",
  "footer.col.project": "Proiektua",
  "footer.repo": "Biltegia GitHuben",
  "footer.releases": "Releases",
  "footer.gpg": "GPG gako publikoa",
  "footer.col.origin": "Jatorria",
  "footer.clienteafirma": "clienteafirma",
  "footer.comparison": "AutoFirmarekiko desberdintasunak",

  "notFound.title": "Ez da orria aurkitu — rFirma",
  "notFound.description": "Ireki duzun helbidea ez dago rfirma.sgomez.me webgunean.",
  "notFound.heading": "Orri hau ez dago",
  "notFound.body": "Baliteke esteka gaizki idatzita egotea edo orria lekuz aldatu izana.",
  "notFound.home": "Itzuli hasierara",
};
