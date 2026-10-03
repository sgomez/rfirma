//! Andamiaje compartido de las pruebas de la línea de órdenes: el token de pruebas, la casa con el Almacén de rFirma y la terminal de guion.

#![allow(dead_code, unused_imports)]
pub use std::cell::RefCell;
pub use std::collections::VecDeque;
pub use std::path::{Path, PathBuf};
pub use std::process::{Command, Output};

pub use rfirma_lib::desktop::adapters::command_line_ports::{
    DiskFiles, NativeFilter, NativeReader, NativeVerifier, SystemTimeZone,
};
pub use rfirma_lib::desktop::adapters::paths::Paths;
pub use rfirma_lib::desktop::adapters::terminal::{ProcessDescriptors, RootsSigner, SeenStores};
pub use rfirma_lib::desktop::application::command_line::{
    attend, CommandLinePorts, Outcome, FAILED, SUCCEEDED,
};
pub use rfirma_lib::desktop::ports::{
    AskedSecret, DesktopHandover, GraphicalPicker, OfferedCertificate, Terminal, WindowChoice,
    WindowOffer,
};
pub use rfirma_lib::identity::adapters::folder::RealInstalledFolder;
pub use rfirma_lib::identity::adapters::pkcs11;
pub use rfirma_lib::identity::application::certificates;
pub use rfirma_lib::identity::domain::keyring::KeyringError;
pub use rfirma_lib::identity::domain::protected_secret::ProtectedSecret;
pub use rfirma_lib::identity::domain::store::Store;
pub use rfirma_lib::identity::every_store;
pub use rfirma_lib::identity::ports::{Keyring, SecretPromptError};
pub use rfirma_lib::signing::adapters::gtk_prompter::MockSecretPrompter;
pub use rfirma_lib::signing::adapters::isolate::Isolate;
pub use rfirma_lib::signing::domain::bridge::{BridgeError, Format, SignatureVerdict};
pub use rfirma_lib::site::ports::ValidationEngine;
pub use rfirma_lib::Roots;

pub use base64::Engine;
pub use std::sync::Arc;

pub const CARD_MODULE: &str = "/usr/lib/softhsm/libsofthsm2.so";
pub const CARD_ACTIVE: &str = "FNMT-ACTIVO-99999999R";
pub const KIT_PASSWORD: &str = "1234";

/// La terminal de las pruebas: contesta lo que diga su guion, y sin TTY si así se pide.
pub struct ScriptedTerminal {
    interactive: bool,
    answers: RefCell<VecDeque<&'static str>>,
    asked: RefCell<Vec<bool>>,
    choice: Option<usize>,
    shown: RefCell<Vec<(Vec<OfferedCertificate>, usize)>>,
}

impl ScriptedTerminal {
    pub fn without_a_tty() -> Self {
        Self::answering(false, &[])
    }

    pub fn typing(answers: &[&'static str]) -> Self {
        Self::answering(true, answers)
    }

    pub fn answering(interactive: bool, answers: &[&'static str]) -> Self {
        Self {
            interactive,
            answers: RefCell::new(answers.iter().copied().collect()),
            asked: RefCell::new(Vec::new()),
            choice: None,
            shown: RefCell::new(Vec::new()),
        }
    }

    /// Elige esa posición de la lista, o la preseleccionada si no se dice ninguna.
    pub fn choosing(mut self, choice: Option<usize>) -> Self {
        self.choice = choice;
        self
    }

    pub fn lists_shown(&self) -> Vec<(Vec<OfferedCertificate>, usize)> {
        self.shown.borrow().clone()
    }

    pub fn retries_asked(&self) -> Vec<bool> {
        self.asked.borrow().clone()
    }
}

impl Terminal for ScriptedTerminal {
    fn is_interactive(&self) -> bool {
        self.interactive
    }

    fn secret(&self, asked: &AskedSecret<'_>) -> Result<ProtectedSecret, String> {
        assert!(self.interactive, "sin TTY no se pide nada a la terminal");
        self.asked.borrow_mut().push(asked.incorrect);
        self.answers
            .borrow_mut()
            .pop_front()
            .map(ProtectedSecret::from_str)
            .ok_or_else(|| "no se ha tecleado nada".to_owned())
    }

    fn chosen(&self, offered: &[OfferedCertificate], preselected: usize) -> Result<usize, String> {
        assert!(self.interactive, "sin TTY no se elige en la terminal");
        self.shown
            .borrow_mut()
            .push((offered.to_vec(), preselected));
        Ok(self.choice.unwrap_or(preselected))
    }
}

pub struct FixedPinKeyring;

impl Keyring for FixedPinKeyring {
    fn pin(&self) -> Result<ProtectedSecret, KeyringError> {
        Ok(ProtectedSecret::from_str(
            "pin-de-pruebas-del-almacen-de-rfirma",
        ))
    }

    fn create_pin(&self) -> Result<ProtectedSecret, KeyringError> {
        self.pin()
    }
}

pub fn the_card_module() -> PathBuf {
    let module = PathBuf::from(CARD_MODULE);
    assert!(
        module.is_file(),
        "falta el modulo PKCS#11 en {}. Estas pruebas necesitan SoftHSM:\n  \
         sudo apt install -y softhsm2 opensc\n  just certs install",
        module.display()
    );
    module
}

/// La configuración de SoftHSM de quien corre la prueba, que el binario no encontraría con otra casa.
pub fn the_softhsm_configuration() -> Option<PathBuf> {
    std::env::var_os("SOFTHSM2_CONF")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".config/softhsm2/softhsm2.conf"))
        })
        .filter(|configuration| configuration.is_file())
}

pub fn kit_p12() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("la raiz del repositorio")
        .join("testdata/fnmt/active-rsa.p12")
}

/// Una casa con un certificado del kit en el Almacén de rFirma, y su alias.
pub fn a_home_with_an_installed_certificate() -> (tempfile::TempDir, String) {
    let home = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let installed = Paths::under(home.path()).installed_certificates_dir();
    let bytes = std::fs::read(kit_p12()).expect("el .p12 de pruebas deberia leerse");
    certificates::install_pkcs12(
        &pkcs11::RealToken,
        &RealInstalledFolder,
        &FixedPinKeyring,
        &installed,
        &bytes,
        KIT_PASSWORD,
    )
    .expect("el .p12 del kit deberia instalarse");
    let installed_stores = every_store(Vec::new(), &installed);
    assert!(
        !installed_stores.is_empty(),
        "falta libsoftokn3.so para abrir el Almacen de rFirma:\n  sudo apt install -y libnss3"
    );
    let alias = pkcs11::list_certificates_across(&installed_stores)
        .expect("el Almacen de rFirma deberia listarse")
        .first()
        .expect("el .p12 instalado deberia traer un certificado")
        .reference()
        .label()
        .to_owned();
    (home, alias)
}

pub fn the_card_aliases() -> Vec<String> {
    pkcs11::list_certificates(Store::module(the_card_module()))
        .expect("el token de pruebas deberia listarse")
        .iter()
        .map(|certificate| certificate.reference().label().to_owned())
        .collect()
}

pub struct NoWindow;

impl DesktopHandover for NoWindow {
    fn hand_over(
        &self,
        _file: &Path,
        _intent: rfirma_lib::desktop::domain::command_line::WindowIntent,
    ) -> Result<(), String> {
        Err("estas pruebas no abren la ventana".to_owned())
    }
}

/// Un rFirma sin ventana bajo esa casa, que solo ve el token de pruebas y toma del llavero de pruebas el PIN del Almacén de rFirma.
pub fn the_roots_under(home: &Path) -> Roots {
    let mut roots = rfirma_lib::roots(Paths::under(home));
    roots.identity.stores = vec![Store::module(the_card_module())];
    roots.identity.keyring =
        Arc::new(|| Ok(Box::new(FixedPinKeyring) as Box<dyn Keyring + Send + Sync>));
    roots
}

pub fn attended_over(words: &[&str], home: &Path, terminal: &ScriptedTerminal) -> Outcome {
    attended_with_the_roots(words, &the_roots_under(home), terminal)
}

/// La ventana de `-certgui` que estas pruebas no abren.
pub struct NoSiteWindow;

impl GraphicalPicker for NoSiteWindow {
    fn has_a_display(&self) -> bool {
        false
    }

    fn chosen(&self, _document: &Path, _offer: WindowOffer<'_>) -> Result<WindowChoice, String> {
        Err("estas pruebas no abren la ventana de sede".to_owned())
    }
}

pub fn attended_with_the_roots(
    words: &[&str],
    roots: &Roots,
    terminal: &ScriptedTerminal,
) -> Outcome {
    attended_with_the_window(words, roots, terminal, &NoSiteWindow)
}

pub fn attended_with_the_window(
    words: &[&str],
    roots: &Roots,
    terminal: &ScriptedTerminal,
    window: &dyn GraphicalPicker,
) -> Outcome {
    let stores = SeenStores::over(roots.identity.all_stores());
    let ports = CommandLinePorts {
        stores: &stores,
        terminal,
        descriptor: &ProcessDescriptors,
        desktop: &NoWindow,
        filter: &NativeFilter,
        files: &DiskFiles,
        verifier: &NativeVerifier,
        reader: &NativeReader,
        time_zone: &SystemTimeZone,
        signer: &RootsSigner::of(roots),
        window,
    };
    let arguments: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
    attend(&arguments, &ports)
}

/// Un PDF de una página, sin firmas.
pub fn a_one_page_pdf() -> Vec<u8> {
    let content = "BT /F1 24 Tf 72 700 Td (rfirma sign) Tj ET\n";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
            .to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{content}endstream",
            content.len()
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
    ];
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", index + 1).as_bytes());
    }
    let xref_at = pdf.len();
    pdf.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in &offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

pub fn verdict_of(roots: &Roots, signed: &[u8]) -> SignatureVerdict {
    roots
        .signing
        .isolate
        .verdict_of(
            &base64::engine::general_purpose::STANDARD.encode(signed),
            Format::Pades,
        )
        .expect("el validador del puente deberia contestar")
}

pub fn sorted_lines_of(stdout: &[u8]) -> Vec<String> {
    let mut lines: Vec<String> = String::from_utf8(stdout.to_vec())
        .expect("stdout en UTF-8")
        .lines()
        .map(str::to_owned)
        .collect();
    lines.sort();
    lines
}

pub fn every_alias_sorted(installed_alias: String) -> Vec<String> {
    let mut aliases = the_card_aliases();
    aliases.push(installed_alias);
    aliases.sort();
    aliases
}
