import type { Dictionary } from "./es";

export const en: Dictionary = {
  "meta.title": "rFirma — Native electronic signature for the desktop",
  "meta.description":
    "Desktop application in Rust and React that replaces AutoFirma's Swing interface and local servers, with the Administration's official cryptography.",
  "meta.image.alt":
    "The rFirma logo on a green background with the domain rfirma.sgomez.me",

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

  "hero.kicker": "Alternative to AutoFirma",
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

  "how.kicker": "Step by step",
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
    "The PIN is requested in a native dialogue, not a web window, and is wiped from memory as soon as the signature is done.",
  "how.step4.title": "Signature complete",
  "how.step4.body": "The signed PDF is saved next to the original, and the summary shows its format.",
  "how.step1.alt": "rFirma with a PDF open that already carries a valid signature",
  "how.step2.alt": "Certificate picker open, with the search box and the available certificates",
  "how.step3.alt": "Native dialog to enter the certificate PIN",
  "how.step4.alt": "PDF signed in PAdES, with the signature summary",


  "pillars.kicker": "Why rFirma",
  "pillars.title": "Four design decisions",
  "pillars.body":
    "It replaces AutoFirma's Swing interface and local servers; the cryptographic engine is the same one, from clienteafirma.",
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
  "comparison.title": "AutoFirma versus rFirma",
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
  "comparison.pin.label": "PIN protection in memory",
  "comparison.pin.autofirma": "Only partly wiped, and it can end up on disk",
  "comparison.pin.rfirma": "Protected in memory, never written to disk and wiped after use",
  "comparison.dnie.label": "Signing with a Spanish DNIe",
  "comparison.dnie.autofirma": "Yes, via jMulticard",
  "comparison.dnie.rfirma": "In development",
  "comparison.store.label": "Certificates in a file (<code>.p12</code>)",
  "comparison.store.autofirma":
    "The file's path is registered in a store dialogue with six options",
  "comparison.store.rfirma": "Its own encrypted store, unlocked with your session",
  "comparison.stores.label": "Certificate lookup",
  "comparison.stores.autofirma": "Only searches the store you pick",
  "comparison.stores.rfirma": "All in one searchable list, one certificate per row",
  "comparison.stamp.label": "Visible signature placement",
  "comparison.stamp.autofirma": "Coordinates or a box with no context",
  "comparison.stamp.rfirma": "Drag it onto the page, with text templates and a faithful preview",
  "comparison.ca.label": "Browser trust in the local server",
  "comparison.ca.autofirma": "The installer registers the CA on the system, with privileges",
  "comparison.ca.rfirma": "The application registers its CA in the person's NSS stores, without root",
  "comparison.lang.label": "Languages",
  "comparison.lang.autofirma": "Spanish only, with the Swing dialogue strings",
  "comparison.lang.rfirma":
    "Spanish, Catalan, Basque, Galician and English, with its own catalogue and switching from Preferences",
  "comparison.privacy.label": "Document manager",
  "comparison.privacy.autofirma": "—",
  "comparison.privacy.rfirma":
    "Remembers recent documents and the last certificate used",
  "comparison.os.label": "Operating systems",
  "comparison.os.autofirma": "Windows, macOS, Linux, Android and iOS",
  "comparison.os.rfirma": "Linux; Windows and macOS, in development",
  "comparison.updates.label": "Update channel",
  "comparison.updates.autofirma": "Manual download of a <code>.deb</code> or <code>.rpm</code>",
  "comparison.updates.rfirma": "Native repositories: APT, DNF and Flatpak",
  "comparison.desktop.label": "Desktop integration",
  "comparison.desktop.autofirma": "Swing's own look",
  "comparison.desktop.rfirma": "Follows the desktop's style, with light and dark themes",

  "install.kicker": "Installation",
  "install.title": "One repository for your operating system",
  "install.body":
    "rFirma's channels are native repositories. Once your system's is added, security patches install through the package manager.",
  "install.tablist.aria": "Distribution channels",
  "install.copy": "Copy",
  "install.copied": "Copied",
  "install.copy.flatpak.aria": "Copy the Flatpak command",
  "install.copy.apt.aria": "Copy the APT commands",
  "install.copy.dnf.aria": "Copy the DNF commands",
  "install.flatpak.body":
    "Recommended for Linux distributions that use neither APT nor DNF. It resolves from rFirma's own ostree remote, and the <code>org.gnome.Platform</code> runtime is downloaded from Flathub with no setup. You only need <code>flatpak</code> and <code>xdg-desktop-portal</code> installed.",
  "install.flatpak.tip":
    "You can also download and double-click install the <a href=\"https://rfirma.sgomez.me/rfirma.flatpakref\">rfirma.flatpakref</a> file if your desktop supports it.",
  "install.apt.body":
    "For Debian, Ubuntu and derived distributions. Sets up the repository using the modern <code>deb822</code> format with the GPG key verified in <code>/usr/share/keyrings/</code>.",
  "install.dnf.body":
    "For Fedora and RPM-based derivatives. Sets up the repository with strict cryptographic checking of metadata and packages (<code>gpgcheck=1</code> and <code>repo_gpgcheck=1</code>).",
  "install.soon": "In development",
  "install.windows.title": "rFirma for Windows",
  "install.windows.body":
    "Per-user installer, no administrator rights needed. It uses the Windows certificate store (MS-CAPI / CNG) directly and, once installed, updates from within the app: every new version arrives with its minisign signature, which is checked before installing it.",
  "install.windows.download": "Download the Windows installer",
  "install.windows.note":
    "Verify what you download: get <code>SHA256SUMS</code> from the <a href=\"https://github.com/sgomez/rfirma/releases/latest\" target=\"_blank\" rel=\"noopener noreferrer\">Release</a> and check that the installer's hash matches its line. The installer is not Authenticode-signed, so SmartScreen will warn you when you open it.",
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
