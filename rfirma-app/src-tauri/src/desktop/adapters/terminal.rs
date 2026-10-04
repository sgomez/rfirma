//! La entrada de la línea de órdenes, `run_the_command_line`, y los adaptadores de sus puertos (`SeenStores`, `ProcessTerminal`, `ProcessDescriptors`, `RootsSigner`); solo toca Tauri para pasarle el contexto al elector de `-certgui`, que es quien abre la única ventana.

use std::collections::BTreeMap;
use std::io::{IsTerminal, Write};
use std::path::PathBuf;

use crate::crossing::Failure;
use crate::desktop::adapters::command_line_ports::{
    DiskFiles, EngineReading, NativeEngine, NativeFilter, NativeVerifier, SystemTimeZone,
};
use crate::desktop::adapters::handover::SpawnedDesktop;
use crate::desktop::adapters::paths::Paths;
use crate::desktop::adapters::site_window_picker::SiteWindowPicker;
use crate::desktop::application::command_line::{attend, CommandLinePorts, FAILED};
use crate::desktop::domain::sign_arguments::Algorithm;
use crate::desktop::ports::{
    AskedSecret, CertificateStores, CommandLineSigning, DocumentSigner, OfferedCertificate,
    SecretDescriptor, Terminal,
};
use crate::documents::domain::document::Document;
use crate::identity::adapters::failures::situation_name;
use crate::identity::adapters::pkcs11::stores::discovered_module_named;
use crate::identity::adapters::{desktop_stores, DesktopToken};
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::holder::prompted_holder_of;
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::{SecretName, StoreSecret};
use crate::identity::domain::store::{Store, StoreClass};
use crate::identity::ports::{prompted_until_accepted, PromptedError, SecretPromptRequest, Token};
use crate::identity::{every_store, IdentityRoot};
use crate::signing::domain::bridge::BridgeError;
use crate::signing::domain::{to_java_properties, Language};
use crate::signing::ports::Signer;
use crate::signing::{DeclaredByTheSite, SigningRoot};
use crate::site::domain::protocol::pairs_of;
use crate::site::domain::protocol::AskedAlgorithm;
use crate::site::ports::{composed_for, PolicyEngine};
use crate::Roots;

mod descriptor;
mod tty;

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

    fn class_of(&self, reference: &CertificateRef) -> StoreClass {
        let store = reference.store();
        self.stores
            .iter()
            .find(|seen| seen.path() == store.path() && seen.init_args() == store.init_args())
            .unwrap_or(&store)
            .class()
    }
}

/// La terminal del proceso: hay alguien al otro lado si la entrada estándar es una TTY.
pub struct ProcessTerminal;

impl Terminal for ProcessTerminal {
    fn is_interactive(&self) -> bool {
        std::io::stdin().is_terminal()
    }

    fn secret(&self, asked: &AskedSecret<'_>) -> Result<ProtectedSecret, String> {
        tty::typed_without_echo(&prompt_for(asked))
    }

    fn chosen(&self, offered: &[OfferedCertificate], preselected: usize) -> Result<usize, String> {
        tty::chosen_on_tty(offered, preselected)
    }
}

/// Los descriptores de este proceso: los que abrió quien lo lanzó.
pub struct ProcessDescriptors;

impl SecretDescriptor for ProcessDescriptors {
    fn read(&self, descriptor: u32) -> Result<ProtectedSecret, String> {
        descriptor::read_from(descriptor)
    }
}

fn prompt_for(asked: &AskedSecret<'_>) -> String {
    let name = match asked.name {
        SecretName::Pin => "PIN",
        _ => "Contraseña",
    };
    let again = if asked.incorrect {
        "rfirma: no es correcto; vuelve a intentarlo.\n"
    } else {
        ""
    };
    format!("{again}{name} de «{}»: ", asked.alias)
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

impl RootsSigner<'_> {
    fn expanded(
        &self,
        request: &CommandLineSigning<'_>,
    ) -> Result<BTreeMap<String, String>, BridgeError> {
        if request.format.signed_without_the_bridge() {
            return Ok(request.parameters.clone());
        }
        let expanded = self.signing.isolate.expand(
            &to_java_properties(request.parameters),
            request.format.name(),
            request.document_length,
        )?;
        Ok(pairs_of(&expanded).into_iter().collect())
    }
}

fn asked(algorithm: Algorithm) -> AskedAlgorithm {
    match algorithm {
        Algorithm::Sha512 => AskedAlgorithm::Sha512,
        Algorithm::Sha384 => AskedAlgorithm::Sha384,
        Algorithm::Sha256 => AskedAlgorithm::Sha256,
    }
}

impl RootsSigner<'_> {
    fn signed_with_the_typed_secret(
        &self,
        signer: &dyn Signer,
        request: &CommandLineSigning<'_>,
    ) -> Result<(), String> {
        if let Some(secret) = request.typed_in_the_window {
            return self
                .signing
                .sign_on_token(signer, secret)
                .map_err(|failure| Failure::from(failure).detail);
        }
        if let Some(descriptor) = request.password_fd {
            let secret = request.descriptor.read(descriptor)?;
            return self
                .signing
                .sign_on_token(signer, &secret)
                .map_err(|failure| Failure::from(failure).detail);
        }
        if request.terminal.is_interactive() {
            self.signed_with_the_secret_typed_on_the_tty(signer, request)
        } else {
            self.signed_with_the_desktop_dialog(signer, request)
        }
    }

    fn signed_with_the_secret_typed_on_the_tty(
        &self,
        signer: &dyn Signer,
        request: &CommandLineSigning<'_>,
    ) -> Result<(), String> {
        let reference = request.certificate.reference();
        let mut asked = AskedSecret {
            name: SecretName::of(reference.store().class()),
            alias: reference.label(),
            incorrect: false,
        };
        loop {
            let typed = request.terminal.secret(&asked)?;
            let Err(failure) = self.signing.sign_on_token(signer, &typed) else {
                return Ok(());
            };
            let failure = Failure::from(failure);
            if failure.situation != situation_name(Situation::IncorrectPin) {
                return Err(failure.detail);
            }
            asked.incorrect = true;
        }
    }
}

impl RootsSigner<'_> {
    fn signed_with_the_desktop_dialog(
        &self,
        signer: &dyn Signer,
        request: &CommandLineSigning<'_>,
    ) -> Result<(), String> {
        let prompt = SecretPromptRequest {
            secret: SecretName::of(request.certificate.reference().store().class()),
            holder: prompted_holder_of(request.certificate.der()),
            language: Language::Spanish,
            incorrect_secret: false,
            origin_window: None,
        };
        prompted_until_accepted(
            self.identity.prompter.as_ref(),
            prompt,
            |typed| self.signing.sign_on_token(signer, typed),
            |failure| Failure::from(failure).situation == situation_name(Situation::IncorrectPin),
        )
        .map(|_| ())
        .map_err(|error| match error {
            PromptedError::Prompt(prompt) => format!(
                "no hay terminal ni --password-fd con el que pedir el PIN, y el diálogo de escritorio falla ({prompt})"
            ),
            PromptedError::Attempt(failure) => Failure::from(failure).detail,
        })
    }
}

impl RootsSigner<'_> {
    fn begun(
        &self,
        request: &CommandLineSigning<'_>,
        signer: &dyn Signer,
    ) -> Result<StoreSecret, String> {
        let algorithm = composed_for(asked(request.algorithm), request.certificate.key_kind())
            .map_err(|error| error.detail().to_owned())?;
        let parameters = self.expanded(request).map_err(|error| error.to_string())?;
        self.signing
            .begin_for_the_site(
                &request.input.display().to_string(),
                Document::passing_through(request.input),
                request.certificate,
                DeclaredByTheSite {
                    format: request.format,
                    algorithm,
                    operation: request.operation,
                    parameters: &parameters,
                    allow_unregistered_signatures: false,
                },
                signer,
            )
            .map_err(|failure| Failure::from(failure).detail)
    }

    fn signed_on_the_token(
        &self,
        secret: StoreSecret,
        signer: &dyn Signer,
        request: &CommandLineSigning<'_>,
    ) -> Result<(), String> {
        if secret == StoreSecret::TypedOnScreen {
            return self.signed_with_the_typed_secret(signer, request);
        }
        self.signing
            .sign_on_token(signer, &ProtectedSecret::new(b""))
            .map_err(|failure| Failure::from(failure).detail)
    }
}

impl DocumentSigner for RootsSigner<'_> {
    fn sign(&self, request: &CommandLineSigning<'_>) -> Result<Vec<u8>, String> {
        if !request.certificate.status().is_usable() {
            return Err("el certificado no está vigente".to_owned());
        }
        let signer = self.identity.signer();
        let secret = self.begun(request, &signer)?;
        self.signed_on_the_token(secret, &signer, request)?;
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

    fn remembered(&self) -> Option<CertificateRef> {
        self.identity.remembered_certificate()
    }
}

struct Homeless;

impl DocumentSigner for Homeless {
    fn sign(&self, _request: &CommandLineSigning<'_>) -> Result<Vec<u8>, String> {
        Err("no se sabe cuál es la carpeta personal".to_owned())
    }

    fn remember(&self, _certificate: &TokenCertificate) {}

    fn remembered(&self) -> Option<CertificateRef> {
        None
    }
}

/// Atiende la línea de órdenes de este argv, con el ejecutable delante, y devuelve el código de salida.
pub fn run_the_command_line(argv: &[String], context: tauri::Context<tauri::Wry>) -> i32 {
    let roots = Paths::from_environment().ok().map(crate::roots);
    let signer = roots.as_ref().map(RootsSigner::of);
    let window = SiteWindowPicker::over(roots.as_ref().map(|roots| &roots.identity), context);
    let ports = CommandLinePorts {
        stores: &SeenStores::of_this_machine(),
        terminal: &ProcessTerminal,
        descriptor: &ProcessDescriptors,
        desktop: &SpawnedDesktop,
        filter: &NativeFilter,
        files: &DiskFiles,
        verifier: &NativeVerifier,
        reader: &EngineReading::over(&NativeEngine),
        time_zone: &SystemTimeZone,
        language: Language::first_of(sys_locale::get_locales()),
        signer: match &signer {
            Some(signer) => signer,
            None => &Homeless,
        },
        window: &window,
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
