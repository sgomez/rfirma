//! La entrada de la línea de órdenes: compone sus puertos y escribe en stdout y stderr lo que deja el caso de uso, sin Tauri ni ventana.

use std::collections::BTreeMap;
use std::io::{IsTerminal, Write};
use std::path::PathBuf;

use crate::crossing::Failure;
use crate::desktop::adapters::command_line_ports::{DiskFiles, NativeVerifier};
use crate::desktop::adapters::handover::SpawnedDesktop;
use crate::desktop::adapters::paths::Paths;
use crate::desktop::application::command_line::{attend, CommandLinePorts, FAILED};
use crate::desktop::domain::sign_arguments::Algorithm;
use crate::desktop::ports::{CertificateStores, CommandLineSigning, DocumentSigner, Terminal};
use crate::documents::domain::document::Document;
use crate::identity::adapters::pkcs11::stores::discovered_module_named;
use crate::identity::adapters::{desktop_stores, DesktopToken};
use crate::identity::domain::certificate::TokenCertificate;
use crate::identity::domain::error::TokenError;
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::StoreSecret;
use crate::identity::domain::store::Store;
use crate::identity::ports::Token;
use crate::identity::{every_store, IdentityRoot};
use crate::signing::domain::bridge::SignatureOperation;
use crate::signing::{DeclaredByTheSite, SigningRoot};
use crate::site::domain::protocol::AskedAlgorithm;
use crate::site::ports::composed_for;
use crate::Roots;

/// Los almacenes que se recorren con el token de esta plataforma.
pub struct SeenStores {
    stores: Vec<Store>,
}

impl SeenStores {
    /// Exactamente estos almacenes.
    pub fn over(stores: Vec<Store>) -> Self {
        Self { stores }
    }

    /// Los almacenes de esta máquina, con el Almacén de rFirma si se sabe dónde vive.
    pub fn of_this_machine() -> Self {
        let configured = desktop_stores();
        match Paths::from_environment() {
            Ok(paths) => Self::over(every_store(configured, &paths.installed_certificates_dir())),
            Err(_) => Self::over(configured),
        }
    }
}

impl CertificateStores for SeenStores {
    fn certificates(&self) -> Result<Vec<TokenCertificate>, TokenError> {
        DesktopToken.list_across(&self.stores)
    }

    fn discovered_module(&self, library: &str) -> Option<PathBuf> {
        discovered_module_named(&self.stores, library)
    }
}

/// La terminal del proceso: hay alguien al otro lado si la entrada estándar es una TTY.
pub struct ProcessTerminal;

impl Terminal for ProcessTerminal {
    fn is_interactive(&self) -> bool {
        std::io::stdin().is_terminal()
    }
}

/// La firma de la sede sobre las raíces de identidad y de firma, sin ventana.
pub struct RootsSigner<'a> {
    identity: &'a IdentityRoot,
    signing: &'a SigningRoot,
}

impl<'a> RootsSigner<'a> {
    /// El firmante de esas raíces.
    pub fn of(roots: &'a Roots) -> Self {
        Self {
            identity: &roots.identity,
            signing: &roots.signing,
        }
    }
}

fn asked(algorithm: Algorithm) -> AskedAlgorithm {
    match algorithm {
        Algorithm::Sha512 => AskedAlgorithm::Sha512,
        Algorithm::Sha384 => AskedAlgorithm::Sha384,
        Algorithm::Sha256 => AskedAlgorithm::Sha256,
    }
}

impl DocumentSigner for RootsSigner<'_> {
    fn sign(&self, request: &CommandLineSigning<'_>) -> Result<Vec<u8>, String> {
        if !request.certificate.status().is_usable() {
            return Err("el certificado no está vigente".to_owned());
        }
        let algorithm = composed_for(asked(request.algorithm), request.certificate.key_kind())
            .map_err(|error| error.detail().to_owned())?;
        let signer = self.identity.signer();
        let parameters = BTreeMap::new();
        let secret = self
            .signing
            .begin_for_the_site(
                &request.input.display().to_string(),
                Document::passing_through(request.input),
                request.certificate,
                DeclaredByTheSite {
                    format: request.format,
                    algorithm,
                    operation: SignatureOperation::Sign,
                    parameters: &parameters,
                    allow_unregistered_signatures: false,
                },
                &signer,
            )
            .map_err(|failure| Failure::from(failure).detail)?;
        if secret != StoreSecret::NotNeeded {
            return Err(
                "pedir el PIN en la terminal todavía no está disponible en esta versión".to_owned(),
            );
        }
        self.signing
            .sign_on_token(&signer, &ProtectedSecret::new(b""))
            .map_err(|failure| Failure::from(failure).detail)?;
        let signed = self
            .signing
            .finish()
            .map_err(|failure| Failure::from(failure).detail)?;
        Ok(signed.completed.into_signed_document())
    }

    fn remember(&self, certificate: &TokenCertificate) {
        self.identity
            .remember_the_certificate(certificate.reference());
    }
}

struct Homeless;

impl DocumentSigner for Homeless {
    fn sign(&self, _request: &CommandLineSigning<'_>) -> Result<Vec<u8>, String> {
        Err("no se sabe cuál es la carpeta personal".to_owned())
    }

    fn remember(&self, _certificate: &TokenCertificate) {}
}

/// Atiende la línea de órdenes de este argv, con el ejecutable delante, y devuelve el código de salida.
pub fn run_the_command_line(argv: &[String]) -> i32 {
    let roots = Paths::from_environment().ok().map(crate::roots);
    let signer = roots.as_ref().map(RootsSigner::of);
    let ports = CommandLinePorts {
        stores: &SeenStores::of_this_machine(),
        terminal: &ProcessTerminal,
        desktop: &SpawnedDesktop,
        files: &DiskFiles,
        verifier: &NativeVerifier,
        signer: match &signer {
            Some(signer) => signer,
            None => &Homeless,
        },
    };
    let outcome = attend(argv.get(1..).unwrap_or_default(), &ports);
    for line in &outcome.stderr {
        eprintln!("{line}");
    }
    let mut stdout = std::io::stdout().lock();
    if let Err(error) = stdout
        .write_all(&outcome.stdout)
        .and_then(|()| stdout.flush())
    {
        eprintln!("rfirma: no se puede escribir en la salida estándar ({error})");
        return FAILED;
    }
    outcome.exit_code
}
