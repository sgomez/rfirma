import type { Dictionary } from "./es";

export const en: Dictionary = {
  "meta.title": "rFirma — Native electronic signature for the desktop",
  "meta.description":
    "Desktop application in Rust and React that replaces AutoFirma's Swing interface and local servers, with the Administration's official cryptography.",

  "notice.aria": "Notice about official ownership",
  "notice.badge": "Notice",
  "notice.official":
    "<strong>rFirma is not an official product of the Public Administration.</strong> It's an independent open-source project under the EUPL 1.2 licence.",

  "nav.aria": "Main navigation",
  "nav.home.aria": "rFirma — home",
  "nav.badge.alpha": "Alpha",
  "nav.how": "How signing works",
  "nav.features": "Features",
  "nav.comparison": "Comparison",
  "nav.install": "Installation",
  "nav.transparency": "Transparency",
  "nav.github": "GitHub",

  "lang.aria": "Language",
  "lang.current": "English",
  "lang.es": "Español",
  "lang.ca": "Català",
  "lang.eu": "Euskara",
  "lang.gl": "Galego",
  "lang.en": "English",

  "hero.kicker": "Independent alternative to AutoFirma",
  "hero.title.line1": "Native electronic signature.",
  "hero.title.line2": "No Java, no waiting.",
  "hero.body":
    "Desktop application in Rust and React that replaces AutoFirma's Swing interface and local servers, with the Administration's official cryptography.",
  "hero.cta.primary": "Install rFirma",
  "hero.cta.secondary": "See how signing works",
  "hero.trust.oss": "Open source, EUPL 1.2",
  "hero.trust.keys": "The private key never leaves your computer",
  "hero.trust.formats": "CAdES, PAdES, XAdES and FacturaE",
  "hero.trust.langs": "In five languages",
  "hero.window.title": "rFirma — Solicitud-subvencion.pdf",
  "hero.screenshot.alt":
    "rFirma main window: document viewer, visible signature placement and side signature panel",
  "hero.card.cert.label": "Certificate",
  "hero.card.cert.value": "ADA LOVELACE BYRON",
  "hero.card.cert.issuer": "AC FNMT Usuarios",
  "hero.card.phase.label": "Stage",
  "hero.card.phase.value": "Assembling",
  "hero.card.stamp.label": "Visible signature",
  "hero.card.stamp.value": "Placed on page 3",
  "hero.card.format.value": "PAdES · 2 signatures",

  "formats.aria": "Supported formats and stores",

  "how.kicker": "Five real screens",
  "how.title": "How a document gets signed",
  "how.tablist.aria": "Signing steps",
  "how.step1.title": "Document loaded",
  "how.step1.body":
    "rFirma opens the PDF and shows what it contains: pages, size and the signatures it already carries. If there's an earlier one, yours will be a co-signature.",
  "how.step2.title": "Choose certificate",
  "how.step2.body":
    "The list is built from the system stores, the browser profiles and the PKCS#11 modules. Expired and revoked ones are shown, but can't be used.",
  "how.step3.title": "Enter the PIN",
  "how.step3.body":
    "The PIN is asked for by the certificate store, not by rFirma. The private key stays inside it for the whole operation.",
  "how.step4.title": "Signing",
  "how.step4.body":
    "Presignature, signature and assembly of the PDF. Each stage is visible while it completes.",
  "how.step5.title": "Signature complete",
  "how.step5.body":
    "The signed PDF is saved next to the original. The summary shows the PAdES format and every signature on the document.",

  "mock.aria": "Mock-up of the rFirma window while signing",
  "mock.badge.signed": "Signed",
  "mock.badge.unsigned": "Unsigned",
  "mock.dropzone": "Drag a PDF here, or click to open one",
  "mock.recents": "Recent",
  "mock.recent1.time": "today, 10:58",
  "mock.recent2.name": "Anexo-II-memoria.pdf",
  "mock.recent2.time": "today, 09:40",
  "mock.recent3.name": "Contrato-alquiler.pdf",
  "mock.recent3.time": "yesterday, 17:20",
  "mock.stamp.name": "ADA LOVELACE BYRON",
  "mock.stamp.id": "DNI 99999999R",
  "mock.stamp.date": "31/08/2026 11:04",
  "mock.pageOf": "of 27",
  "mock.zoom": "100 %",
  "mock.file.unsigned": "Solicitud-subvencion.pdf",
  "mock.file.signed": "Solicitud-subvencion-firmado.pdf",
  "mock.file.meta": "27 pages · 2.4 MB",
  "mock.panel.certificate": "Certificate",
  "mock.panel.summary": "Summary",
  "mock.chooseCert": "Choose certificate",
  "mock.cert.chosen.detail": "99999999R · AC FNMT Usuarios · Firefox",
  "mock.summary.format": "PAdES",
  "mock.summary.count": "2 signatures",
  "mock.summary.mine": "Yours",
  "mock.summary.sig1.detail": "99999999R · today, 11:04",
  "mock.summary.sig2.name": "SERVICIO DE GESTION TRIBUTARIA",
  "mock.summary.sig2.detail": "Q2800001A · 26/08/2026 09:12",
  "mock.visible.title": "Visible signature",
  "mock.visible.toggle": "Stamp a signature box on the document",
  "mock.visible.hint.off": "Turns on once you choose a certificate",
  "mock.visible.hint.on": "Page 3 · drag to place it",
  "mock.saveIn": "It will be saved in",
  "mock.saveIn.dir": "…/Documents/",
  "mock.action.sign": "Sign document",
  "mock.action.open": "Open the PDF",
  "mock.certlist.available": "Available",
  "mock.certlist.unusable": "Unusable",
  "mock.certlist.cert2.name": "GRACE HOPPER MURRAY",
  "mock.certlist.cert2.detail": "88888888T · AC Representación · Installed in rFirma",
  "mock.certlist.cert3.name": "ALAN TURING MATHISON",
  "mock.certlist.cert3.detail": "77777777P · AC FNMT Usuarios · Chrome",
  "mock.certlist.cert3.state": "Revoked on 12 January 2026",
  "mock.pin.title": "Enter the PIN",
  "mock.pin.subject": "ADA LOVELACE BYRON · 99999999R",
  "mock.pin.label": "PIN",
  "mock.pin.cancel": "Cancel",
  "mock.pin.sign": "Sign",
  "mock.signing.title": "Signing the document…",
  "mock.signing.presign": "Preparing the signature",
  "mock.signing.presign.note": "(presignature)",
  "mock.signing.sign": "Signing",
  "mock.signing.assemble": "Assembling the PDF",
  "mock.signing.assemble.note": "(postsignature)",

  "pillars.kicker": "Why rFirma",
  "pillars.title": "Four decisions that matter",
  "pillars.body":
    "It's not a coat of paint over AutoFirma: it's a different architecture, with the same cryptographic engine.",
  "pillars.native.title": "Native performance",
  "pillars.native.body":
    "Desktop app in Tauri v2, Rust and React. No JVM and no local servers listening.",
  "pillars.keys.title": "The private key never leaves the system",
  "pillars.keys.body":
    "Signing happens on your computer, through the system stores and the PKCS#11 modules. Java never sees it.",
  "pillars.crypto.title": "Official cryptography",
  "pillars.crypto.body":
    "CAdES, PAdES, XAdES and FacturaE come from the <code>clienteafirma</code> code, compiled to a native binary with GraalVM.",
  "pillars.stamp.title": "Draggable visible signature",
  "pillars.stamp.body":
    "Place and size the rubric on the page, with a faithful preview. No blind coordinates.",

  "comparison.kicker": "Comparison",
  "comparison.title": "AutoFirma and rFirma, fact by fact",
  "comparison.body":
    "Only differences you can check in the code, in the interface and in each project's distribution. No figures nobody measured.",
  "comparison.head.aspect": "Aspect",
  "comparison.head.autofirma": "AutoFirma (official)",
  "comparison.head.rfirma": "rFirma",
  "comparison.arch.label": "Architecture",
  "comparison.arch.autofirma": "Java Swing on the JVM",
  "comparison.arch.rfirma":
    "Tauri v2 (Rust + React) with the clienteafirma engine on GraalVM Native Image",
  "comparison.keys.label": "Where the private key is processed",
  "comparison.keys.autofirma": "In the Java process",
  "comparison.keys.rfirma": "In the system store or the PKCS#11 module; it never leaves it",
  "comparison.store.label": "Certificates in a file (<code>.p12</code>)",
  "comparison.store.autofirma":
    "The file's path is registered in a store dialogue with six options",
  "comparison.store.rfirma":
    "Its own store: \"Add…\" copies the certificate, and nothing from the file is kept, not even the path",
  "comparison.stores.label": "Certificate lookup",
  "comparison.stores.autofirma": "You have to pick a store, and it only shows that one",
  "comparison.stores.rfirma":
    "Sweeps the system, the browser profiles, the PKCS#11 modules and its own store, and joins them into a single list",
  "comparison.stamp.label": "Visible signature placement",
  "comparison.stamp.autofirma": "Coordinates or a box with no context",
  "comparison.stamp.rfirma": "Drag on the page with a faithful preview",
  "comparison.sede.label": "Signing from a government site",
  "comparison.sede.autofirma": "Certificate selection dialogue",
  "comparison.sede.rfirma":
    "Consent window: who's asking, what's being signed and with which certificate, before signing",
  "comparison.ca.label": "Browser trust in the local server",
  "comparison.ca.autofirma": "The installer registers the CA on the system, with privileges",
  "comparison.ca.rfirma": "The application registers its CA in the person's NSS stores, without root",
  "comparison.lang.label": "Languages",
  "comparison.lang.autofirma": "Spanish and the co-official languages, with the Swing dialogue strings",
  "comparison.lang.rfirma":
    "Spanish, Catalan, Basque, Galician and English, with its own catalogue and switching from Preferences",
  "comparison.privacy.label": "Privacy",
  "comparison.privacy.autofirma": "—",
  "comparison.privacy.rfirma":
    "Recents and the last certificate can be turned off and cleared; the only outgoing connection is the version check, and it can be turned off",
  "comparison.updates.label": "Update channel",
  "comparison.updates.autofirma": "Manual download of a <code>.deb</code> or <code>.rpm</code>",
  "comparison.updates.rfirma": "Native repositories: Flatpak, APT and DNF",
  "comparison.desktop.label": "Desktop integration",
  "comparison.desktop.autofirma": "Swing's own look",
  "comparison.desktop.rfirma":
    "Interface that follows the desktop's conventions, light and dark theme, AA contrast",

  "install.kicker": "Installation",
  "install.title": "One repository, and updates arrive on their own",
  "install.body":
    "rFirma's channels are native repositories. Once your system's is added, security patches install through the package manager.",
  "install.alert.title": "The packages aren't published yet",
  "install.alert.body":
    "The repositories are still being prepared. The commands below won't work until the first version (v0.4) is published. Development is tracked on <a href=\"https://github.com/sgomez/rfirma\" target=\"_blank\" rel=\"noopener noreferrer\">GitHub</a>.",
  "install.tablist.aria": "Distribution channels",
  "install.copy": "Copy",
  "install.copied": "Copied",
  "install.copy.flatpak.aria": "Copy the Flatpak command",
  "install.copy.apt.aria": "Copy the APT commands",
  "install.copy.dnf.aria": "Copy the DNF commands",
  "install.flatpak.body":
    "Recommended installation for any Linux distribution. It resolves from rFirma's own ostree remote. Requires the <code>org.gnome.Platform</code> runtime from Flathub.",
  "install.flatpak.tip":
    "You can also download and double-click install the <a href=\"https://rfirma.sgomez.me/rfirma.flatpakref\">rfirma.flatpakref</a> file if your desktop supports it.",
  "install.apt.body":
    "For Debian, Ubuntu and derived distributions. Sets up the repository using the modern <code>deb822</code> format with the GPG key verified in <code>/usr/share/keyrings/</code>.",
  "install.dnf.body":
    "For Fedora and RPM-based derivatives. Sets up the repository with strict cryptographic checking of metadata and packages (<code>gpgcheck=1</code> and <code>repo_gpgcheck=1</code>).",
  "install.soon": "In development",
  "install.windows.title": "Windows support in the works",
  "install.windows.body":
    "Native integration for Windows is currently in development. It will use the Windows certificate store (MS-CAPI / CNG) directly to sign with your personal certificate or DNIe without any intermediate software.",
  "install.windows.note":
    "It will be available as an <code>.msi</code> installer and through <code>winget</code>. You can follow the project's progress on <a href=\"https://github.com/sgomez/rfirma\" target=\"_blank\" rel=\"noopener noreferrer\">GitHub</a>.",
  "install.macos.title": "macOS support in the works",
  "install.macos.body":
    "The native version for macOS is in active development. It will integrate with Apple's Keychain and CryptoTokenKit for smooth, secure access to the system's digital identities.",
  "install.macos.note":
    "It will be distributed as a <code>.dmg</code> disk image and through a <code>Homebrew</code> formula. You can follow the project's progress on <a href=\"https://github.com/sgomez/rfirma\" target=\"_blank\" rel=\"noopener noreferrer\">GitHub</a>.",

  "transparency.kicker": "Transparency",
  "transparency.title": "All the code, in the open",
  "transparency.body":
    "rFirma is free, auditable software under the <strong>EUPL 1.2</strong> licence. The code and the cryptographic engine live in two public repositories.",
  "transparency.repo1.title": "Application and FFI bridge",
  "transparency.repo1.body":
    "<a href=\"https://github.com/sgomez/rfirma\" target=\"_blank\" rel=\"noopener noreferrer\">sgomez/rfirma</a> holds the Tauri interface (Rust + React), the FFI bridge (<code>rfirma-native-bridge</code>) and the packaging.",
  "transparency.repo2.title": "Original cryptographic engine",
  "transparency.repo2.body":
    "The signing logic consumes the artefacts from <a href=\"https://github.com/ctt-gob-es/clienteafirma\" target=\"_blank\" rel=\"noopener noreferrer\">ctt-gob-es/clienteafirma</a>, compiled into a native library with GraalVM Native Image.",
  "transparency.key.title": "Package signing key",
  "transparency.key.body":
    "Public key at <a href=\"https://rfirma.sgomez.me/rfirma.asc\">rfirma.asc</a>. Check its fingerprint after downloading it with <code>gpg --show-keys rfirma.asc</code>:",

  "footer.tagline":
    "Native electronic signature for the desktop, with the official cryptographic engine and no Java on your computer.",
  "footer.license": "EUPL 1.2 licence",
  "footer.col.project": "Project",
  "footer.repo": "Repository on GitHub",
  "footer.releases": "Releases",
  "footer.gpg": "Public GPG key",
  "footer.col.origin": "Origin",
  "footer.clienteafirma": "clienteafirma",
  "footer.comparison": "Differences from AutoFirma",
};
