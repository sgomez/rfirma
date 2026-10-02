//! La línea de órdenes contra el token `rfirma-test` y el Almacén de rFirma: por su caso de uso con una terminal guionizada, y lanzando el binario `rfirma`.

use std::path::{Path, PathBuf};
use std::process::Command;

use rfirma_lib::desktop::adapters::command_line_ports::{DiskFiles, NativeVerifier};
use rfirma_lib::desktop::adapters::paths::Paths;
use rfirma_lib::desktop::adapters::terminal::{RootsSigner, SeenStores};
use rfirma_lib::desktop::application::command_line::{
    attend, CommandLinePorts, Outcome, FAILED, SUCCEEDED,
};
use rfirma_lib::desktop::ports::{DesktopHandover, Terminal};
use rfirma_lib::identity::adapters::folder::RealInstalledFolder;
use rfirma_lib::identity::adapters::pkcs11;
use rfirma_lib::identity::application::certificates;
use rfirma_lib::identity::domain::keyring::KeyringError;
use rfirma_lib::identity::domain::protected_secret::ProtectedSecret;
use rfirma_lib::identity::domain::store::Store;
use rfirma_lib::identity::every_store;
use rfirma_lib::identity::ports::Keyring;
use rfirma_lib::signing::adapters::isolate::Isolate;
use rfirma_lib::signing::domain::bridge::{BridgeError, Format, SignatureVerdict};
use rfirma_lib::site::ports::ValidationEngine;
use rfirma_lib::Roots;

use base64::Engine;
use std::sync::Arc;

const CARD_MODULE: &str = "/usr/lib/softhsm/libsofthsm2.so";
const CARD_ACTIVE: &str = "FNMT-ACTIVO-99999999R";
const KIT_PASSWORD: &str = "1234";

/// La terminal de las pruebas: contesta lo que diga su guion, y sin TTY si así se pide.
struct ScriptedTerminal {
    interactive: bool,
}

impl ScriptedTerminal {
    fn without_a_tty() -> Self {
        Self { interactive: false }
    }
}

impl Terminal for ScriptedTerminal {
    fn is_interactive(&self) -> bool {
        self.interactive
    }
}

struct FixedPinKeyring;

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

fn the_card_module() -> PathBuf {
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
fn the_softhsm_configuration() -> Option<PathBuf> {
    std::env::var_os("SOFTHSM2_CONF")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".config/softhsm2/softhsm2.conf"))
        })
        .filter(|configuration| configuration.is_file())
}

fn kit_p12() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("la raiz del repositorio")
        .join("testdata/fnmt/active-rsa.p12")
}

/// Una casa con un certificado del kit en el Almacén de rFirma, y su alias.
fn a_home_with_an_installed_certificate() -> (tempfile::TempDir, String) {
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

fn the_card_aliases() -> Vec<String> {
    pkcs11::list_certificates(Store::module(the_card_module()))
        .expect("el token de pruebas deberia listarse")
        .iter()
        .map(|certificate| certificate.reference().label().to_owned())
        .collect()
}

struct NoWindow;

impl DesktopHandover for NoWindow {
    fn hand_over(&self, _file: &Path) -> Result<(), String> {
        Err("estas pruebas no abren la ventana".to_owned())
    }
}

/// Un rFirma sin ventana bajo esa casa, que solo ve el token de pruebas y toma del llavero de pruebas el PIN del Almacén de rFirma.
fn the_roots_under(home: &Path) -> Roots {
    let mut roots = rfirma_lib::roots(Paths::under(home));
    roots.identity.stores = vec![Store::module(the_card_module())];
    roots.identity.keyring =
        Arc::new(|| Ok(Box::new(FixedPinKeyring) as Box<dyn Keyring + Send + Sync>));
    roots
}

fn attended_over(words: &[&str], home: &Path, terminal: &ScriptedTerminal) -> Outcome {
    attended_with_the_roots(words, &the_roots_under(home), terminal)
}

fn attended_with_the_roots(words: &[&str], roots: &Roots, terminal: &ScriptedTerminal) -> Outcome {
    let stores = SeenStores::over(roots.identity.all_stores());
    let ports = CommandLinePorts {
        stores: &stores,
        terminal,
        desktop: &NoWindow,
        files: &DiskFiles,
        verifier: &NativeVerifier,
        signer: &RootsSigner::of(roots),
    };
    let arguments: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
    attend(&arguments, &ports)
}

/// Un PDF de una página, sin firmas.
fn a_one_page_pdf() -> Vec<u8> {
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

fn verdict_of(roots: &Roots, signed: &[u8]) -> SignatureVerdict {
    roots
        .signing
        .isolate
        .verdict_of(
            &base64::engine::general_purpose::STANDARD.encode(signed),
            Format::Pades,
        )
        .expect("el validador del puente deberia contestar")
}

fn sorted_lines_of(stdout: &[u8]) -> Vec<String> {
    let mut lines: Vec<String> = String::from_utf8(stdout.to_vec())
        .expect("stdout en UTF-8")
        .lines()
        .map(str::to_owned)
        .collect();
    lines.sort();
    lines
}

fn every_alias_sorted(installed_alias: String) -> Vec<String> {
    let mut aliases = the_card_aliases();
    aliases.push(installed_alias);
    aliases.sort();
    aliases
}

#[test]
fn listaliases_lists_the_test_token_and_the_rfirma_store_one_alias_per_line() {
    let (home, installed_alias) = a_home_with_an_installed_certificate();

    let outcome = attended_over(
        &["listaliases"],
        home.path(),
        &ScriptedTerminal::without_a_tty(),
    );

    assert_eq!(outcome.exit_code, SUCCEEDED, "{:?}", outcome.stderr);
    let aliases = sorted_lines_of(&outcome.stdout);
    assert_eq!(aliases, every_alias_sorted(installed_alias));
    assert!(
        aliases.iter().any(|alias| alias == CARD_ACTIVE),
        "{aliases:?}"
    );
}

#[test]
fn listaliases_with_the_card_module_lists_only_the_token() {
    let (home, _installed_alias) = a_home_with_an_installed_certificate();
    let store = format!("pkcs11:{}", the_card_module().display());

    let outcome = attended_over(
        &["listaliases", "-store", &store],
        home.path(),
        &ScriptedTerminal::without_a_tty(),
    );

    assert_eq!(outcome.exit_code, SUCCEEDED, "{:?}", outcome.stderr);
    let mut aliases = the_card_aliases();
    aliases.sort();
    assert_eq!(sorted_lines_of(&outcome.stdout), aliases);
}

#[test]
fn listaliases_with_the_nss_family_lists_only_the_rfirma_store() {
    let (home, installed_alias) = a_home_with_an_installed_certificate();

    let outcome = attended_over(
        &["listaliases", "-store", "mozilla"],
        home.path(),
        &ScriptedTerminal::without_a_tty(),
    );

    assert_eq!(outcome.exit_code, SUCCEEDED, "{:?}", outcome.stderr);
    assert_eq!(sorted_lines_of(&outcome.stdout), vec![installed_alias]);
}

#[test]
fn listaliases_with_a_module_that_is_not_discovered_loads_nothing_and_fails() {
    let (home, _installed_alias) = a_home_with_an_installed_certificate();

    let outcome = attended_over(
        &["listaliases", "-store", "pkcs11:/usr/lib/no-existe.so"],
        home.path(),
        &ScriptedTerminal::without_a_tty(),
    );

    assert_eq!(outcome.exit_code, 1);
    assert!(outcome.stdout.is_empty());
}

#[test]
fn the_rfirma_binary_lists_aliases_without_a_window_and_with_a_clean_stdout() {
    let (home, installed_alias) = a_home_with_an_installed_certificate();

    let mut rfirma = Command::new(env!("CARGO_BIN_EXE_rfirma"));
    if let Some(configuration) = the_softhsm_configuration() {
        rfirma.env("SOFTHSM2_CONF", configuration);
    }
    let output = rfirma
        .arg("listaliases")
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path().join("config"))
        .env("XDG_STATE_HOME", home.path().join("state"))
        .env("XDG_DATA_HOME", home.path().join("data"))
        .env("RFIRMA_PKCS11_MODULE", the_card_module())
        .env("RUST_LOG", "trace")
        .output()
        .expect("el binario rfirma deberia lanzarse");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(SUCCEEDED), "{stderr}");
    assert_eq!(
        sorted_lines_of(&output.stdout),
        every_alias_sorted(installed_alias),
        "{stderr}"
    );
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn sign_with_an_alias_of_the_rfirma_store_signs_a_valid_pades_exactly_at_the_output() {
    let (home, installed_alias) = a_home_with_an_installed_certificate();
    let roots = the_roots_under(home.path());
    let input = home.path().join("documento.pdf");
    std::fs::write(&input, a_one_page_pdf()).expect("el PDF deberia escribirse");

    for (output, extra) in [
        ("firmado.pdf", &[][..]),
        (
            "firmado-sha256.pdf",
            &["-format", "pades", "-algorithm", "sha256"][..],
        ),
    ] {
        let output = home.path().join(output);
        std::fs::write(&output, b"lo que hubiera antes").expect("deberia escribirse");
        let mut words = vec![
            "sign",
            "-i",
            input.to_str().expect("ruta UTF-8"),
            "-o",
            output.to_str().expect("ruta UTF-8"),
            "-alias",
            &installed_alias,
        ];
        words.extend_from_slice(extra);

        let outcome = attended_with_the_roots(&words, &roots, &ScriptedTerminal::without_a_tty());

        assert_eq!(outcome.exit_code, SUCCEEDED, "{:?}", outcome.stderr);
        assert!(outcome.stdout.is_empty());
        assert!(!outcome.stderr.is_empty());
        let signed = std::fs::read(&output).expect("la firma deberia estar en -o");
        assert_eq!(verdict_of(&roots, &signed), SignatureVerdict::Valid);
    }
    assert_eq!(
        roots
            .identity
            .remembered_certificate()
            .map(|reference| reference.label().to_owned()),
        Some(installed_alias)
    );
    let state = roots
        .signing
        .memory
        .state()
        .expect("el estado deberia leerse")
        .into_value();
    assert!(state.recents.is_empty(), "no se apunta en los recientes");
}

#[test]
fn a_signature_the_bridge_cannot_make_fails_on_stderr_and_leaves_no_output_or_memory() {
    let (home, installed_alias) = a_home_with_an_installed_certificate();
    let mut roots = the_roots_under(home.path());
    roots.signing.isolate =
        Isolate::start_with(|| Err(BridgeError::Failed("sin puente en esta prueba".to_owned())));
    let input = home.path().join("documento.pdf");
    std::fs::write(&input, a_one_page_pdf()).expect("el PDF deberia escribirse");
    let output = home.path().join("firmado.pdf");

    let outcome = attended_with_the_roots(
        &[
            "sign",
            "-i",
            input.to_str().expect("ruta UTF-8"),
            "-o",
            output.to_str().expect("ruta UTF-8"),
            "-alias",
            &installed_alias,
        ],
        &roots,
        &ScriptedTerminal::without_a_tty(),
    );

    assert_eq!(outcome.exit_code, FAILED, "{:?}", outcome.stderr);
    assert!(
        outcome.stderr.concat().contains("sin puente"),
        "{:?}",
        outcome.stderr
    );
    assert!(outcome.stdout.is_empty());
    assert!(!output.exists());
    assert_eq!(roots.identity.remembered_certificate(), None);
}

#[test]
fn the_rfirma_binary_attends_sign_as_a_terminal_command_and_not_with_a_window() {
    let (home, _installed_alias) = a_home_with_an_installed_certificate();
    let input = home.path().join("documento.pdf");
    std::fs::write(&input, a_one_page_pdf()).expect("el PDF deberia escribirse");
    let output = home.path().join("firmado.pdf");

    let mut rfirma = Command::new(env!("CARGO_BIN_EXE_rfirma"));
    if let Some(configuration) = the_softhsm_configuration() {
        rfirma.env("SOFTHSM2_CONF", configuration);
    }
    let finished = rfirma
        .arg("sign")
        .arg("-i")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .args(["-alias", "nadie-con-este-alias"])
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path().join("config"))
        .env("XDG_STATE_HOME", home.path().join("state"))
        .env("XDG_DATA_HOME", home.path().join("data"))
        .env("RFIRMA_PKCS11_MODULE", the_card_module())
        .output()
        .expect("el binario rfirma deberia lanzarse");

    let stderr = String::from_utf8_lossy(&finished.stderr);
    assert_eq!(finished.status.code(), Some(FAILED), "{stderr}");
    assert!(stderr.contains("nadie-con-este-alias"), "{stderr}");
    assert!(finished.stdout.is_empty(), "{stderr}");
    assert!(!output.exists());
}
