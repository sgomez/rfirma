//! Composición y arranque de la aplicación Tauri: junta las raíces de los cinco contextos.

pub mod desktop;
pub mod documents;
pub mod identity;
pub mod signing;
pub mod site;

pub mod commands {
    pub mod failure;

    #[cfg(test)]
    mod guards;

    pub use failure::Failure;
}

#[cfg(test)]
pub(crate) mod fixtures;
#[cfg(test)]
mod tests;

use std::sync::Mutex;

use desktop::adapters::paths::Paths;
use documents::domain::destination::DestinationFolder;
use identity::application::listed::ListedCertificates;
use signing::adapters::store::{JsonFile, Loaded};
use signing::application::configuration_memory::Configuration;
use signing::application::state::{State, VersionCheck};
use signing::domain::memory_error::MemoryError;

/// Variable de entorno para sobreescribir el módulo PKCS#11.
pub const PKCS11_MODULE_VARIABLE: &str = "RFIRMA_PKCS11_MODULE";

/// Nombre del evento emitido cuando se suelta un documento en la ventana.
pub const DOCUMENT_DROPPED: &str = "document-dropped";

/// Entorno de composición que agrupa almacenes, configuración y persistencia.
pub struct Environment {
    /// Almacenes de certificados configurados.
    pub stores: Vec<crate::identity::adapters::pkcs11::Store>,
    /// Certificados del último listado.
    pub listed: ListedCertificates,
    /// Carpeta de documentos del usuario por omisión.
    pub documents_folder: std::path::PathBuf,
    /// Configuración en memoria viva compartida.
    pub configuration: Mutex<Configuration>,
    /// Acceso a la persistencia en disco (ADR-0010).
    pub memory: Memory,
    /// Almacén de la rúbrica (ADR-0012).
    pub rubric: crate::documents::adapters::rubric::RubricStore,
    /// Directorio de certificados de software instalados.
    pub installed_certificates: std::path::PathBuf,
}

impl Environment {
    /// Devuelve una instantánea de la configuración viva.
    pub fn configuration(&self) -> Configuration {
        lock(&self.configuration).clone()
    }

    /// Devuelve todos los almacenes disponibles incluyendo certificados instalados.
    pub fn all_stores(&self) -> Vec<crate::identity::adapters::pkcs11::Store> {
        let mut stores = self.stores.clone();
        if let Some(softoken) = crate::identity::adapters::pkcs11::stores::softoken() {
            stores.extend(crate::identity::adapters::pkcs11::stores::installed_stores(
                &softoken,
                &self.installed_certificates,
            ));
        }
        stores
    }
}

/// Resuelve la carpeta destino elegida o la carpeta de documentos por omisión.
pub fn chosen_folder(
    configuration: &Configuration,
    documents_folder: impl Into<std::path::PathBuf>,
) -> DestinationFolder {
    configuration
        .destination
        .clone()
        .unwrap_or_else(|| DestinationFolder::at(documents_folder))
}

/// Adquiere el cerrojo recuperando el valor si el mutex estaba envenenado.
pub fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Las dos memorias y sus dos soportes (ADR-0010).
#[derive(Clone, Debug, PartialEq)]
pub struct Memory {
    configuration: JsonFile<Configuration>,
    state: JsonFile<State>,
}

impl Memory {
    /// La memoria que vive en las rutas dadas.
    pub fn at(paths: &Paths) -> Self {
        Self {
            configuration: JsonFile::at(paths.config_file()),
            state: JsonFile::at(paths.state_file()),
        }
    }

    /// El soporte de la configuración.
    pub fn configuration_file(&self) -> &JsonFile<Configuration> {
        &self.configuration
    }

    /// El soporte del estado.
    pub fn state_file(&self) -> &JsonFile<State> {
        &self.state
    }

    /// La configuración guardada, o la de por omisión.
    pub fn configuration(&self) -> Result<Loaded<Configuration>, MemoryError> {
        self.configuration.load()
    }

    /// El estado guardado, o el vacío.
    pub fn state(&self) -> Result<Loaded<State>, MemoryError> {
        self.state.load()
    }

    /// Guarda la configuración y borra el estado si la actividad queda desactivada (ADR-0010).
    pub fn remember_configuration(&self, configuration: &Configuration) -> Result<(), MemoryError> {
        self.configuration.save(configuration)?;
        if !configuration.remember_activity {
            self.erase_activity_but_keep_the_exempt()?;
        }
        Ok(())
    }

    /// Guarda el estado según lo que permitan los dos interruptores (ADR-0010).
    pub fn remember_state(
        &self,
        configuration: &Configuration,
        state: &State,
    ) -> Result<(), MemoryError> {
        if !configuration.remember_activity {
            return self.erase_activity_but_keep_the_exempt();
        }
        if configuration.remember_visible_signature {
            return self.state.save(state);
        }
        let mut without_the_box = state.clone();
        without_the_box.visible_signature = None;
        without_the_box.recents.forget_placements();
        self.state.save(&without_the_box)
    }

    /// Olvida lo acumulado conservando los datos exentos (ADR-0010).
    pub fn forget_activity(&self) -> Result<(), MemoryError> {
        self.erase_activity_but_keep_the_exempt()
    }

    /// Guarda el registro de comprobación de versión sin depender de interruptores de actividad.
    pub fn remember_version_check(&self, check: VersionCheck) -> Result<(), MemoryError> {
        let mut state = self.state.load()?.into_value();
        state.version_check = Some(check);
        self.state.save(&state)
    }

    fn erase_activity_but_keep_the_exempt(&self) -> Result<(), MemoryError> {
        let mut kept = self
            .state
            .load()
            .map(Loaded::into_value)
            .unwrap_or_default();
        kept.forget_everything();
        self.state.erase()?;
        if kept.is_empty() {
            return Ok(());
        }
        self.state.save(&kept)
    }
}

/// Punto de entrada compartido por el binario y por las pruebas.
pub fn run() {
    use site::application::errand::Transport as _;
    use tauri::{Emitter, Manager};

    if desktop::application::invocation::help_was_asked_for(
        std::env::args_os().map(|argument| argument.to_string_lossy().into_owned()),
    ) {
        println!("{}", desktop::application::invocation::HELP);
        return;
    }

    desktop::application::invocation::make_the_command_line_readable();

    let paths = desktop::adapters::paths::Paths::from_environment()
        .expect("debería saberse cuál es el HOME");

    let ca_store = site::adapters::tls::LocalCaStore::of(&paths);
    let nss_profiles = nss_profiles_of_this_home();

    let invocation = desktop::application::invocation::Invocation::of_this_process();
    let second_store = ca_store.clone();

    let local_ca_trust = site::application::startup::LocalCaTrust {
        store: ca_store.clone(),
        profiles: nss_profiles.clone(),
    };

    let memory = Memory::at(&paths);
    let configuration = memory
        .configuration()
        .map(signing::adapters::store::Loaded::into_value)
        .unwrap_or_default();
    let environment = Environment {
        stores: identity::adapters::pkcs11::stores::from_environment(),
        listed: identity::application::listed::ListedCertificates::new(),
        documents_folder: desktop::adapters::paths::documents_folder().unwrap_or_default(),
        configuration: std::sync::Mutex::new(configuration),
        memory,
        rubric: documents::adapters::rubric::RubricStore::at(paths.rubric_path()),
        installed_certificates: paths.installed_certificates_dir(),
    };

    tauri::Builder::default()
        // Instancia única (ADR-0010).
        .plugin(tauri_plugin_single_instance::init(
            move |app, command_line, folder| {
                use tauri::Manager as _;
                let invocation = desktop::application::invocation::Invocation {
                    command_line,
                    folder: std::path::PathBuf::from(folder),
                };
                let session = app.state::<signing::application::session::SigningSession>();
                let opened = app.state::<documents::application::opened::OpenedDocuments>();
                let substitution = desktop::application::invocation::second_invocation(
                    &invocation,
                    &opened,
                    signing::application::session::is_live(&session),
                );
                match substitution {
                    desktop::application::invocation::SecondInvocation::ReplacesWhatWasThere(
                        view,
                    ) => {
                        let Some(window) = app.get_webview_window("main") else {
                            return;
                        };
                        let _ = window.set_focus();
                        let _ = window.emit(
                            DOCUMENT_DROPPED,
                            documents::adapters::views::DroppedDocumentView::from(*view),
                        );
                    }
                    desktop::application::invocation::SecondInvocation::OpensItsOwnWindow(url) => {
                        let handle = app.clone();
                        let transport = the_transport(&second_store, app);
                        let attendance = site::application::startup::attend_site_launch(
                            &url,
                            &|ports, duty| transport.open(ports, duty),
                            &|_| site::adapters::window::open_the_site_window(&handle),
                            app.state::<site::application::errand::LiveErrand>().inner(),
                            // A mitad de un trámite no se toca la CA local (ADR-0005).
                            site::application::startup::LocalCaReach::NotAnObstacle,
                        );
                        say(site::application::startup::hold_the_channel(
                            &app.state::<site::application::startup::HeldChannel>(),
                            attendance,
                        ));
                    }
                    desktop::application::invocation::SecondInvocation::NothingHappens => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.set_focus();
                        }
                    }
                }
            },
        ))
        .plugin(tauri_plugin_dialog::init())
        // El PDF firmado y su carpeta se abren a través del portal (ADR-0011).
        .plugin(tauri_plugin_opener::init())
        .manage(environment)
        .manage(signing::adapters::isolate::Isolate::start())
        .manage(signing::application::session::SigningSession::default())
        .manage(desktop::application::invocation::PendingInvocation::of(
            invocation.clone(),
        ))
        .manage(site::application::errand::LiveErrand::default())
        .manage(site::application::startup::HeldChannel::default())
        .manage(local_ca_trust)
        .manage(documents::application::opened::OpenedDocuments::new())
        // En Tauri el arrastre se gestiona por evento de ventana (ADR-0011).
        .on_window_event(|window, event| {
            if window.label() == site::adapters::window::SITE_WINDOW
                && matches!(event, tauri::WindowEvent::CloseRequested { .. })
            {
                site::application::errand::decline(
                    &window.state::<site::application::errand::LiveErrand>(),
                );
                return;
            }

            let tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) = event
            else {
                return;
            };
            let opened = window.state::<documents::application::opened::OpenedDocuments>();
            let Some(dropped) = documents::application::documents::dropped_document(paths, &opened)
            else {
                return;
            };
            let _ = window.emit(
                DOCUMENT_DROPPED,
                documents::adapters::views::DroppedDocumentView::from(dropped),
            );
        })
        .invoke_handler(tauri::generate_handler![
            identity::adapters::tauri::list_certificates,
            signing::adapters::tauri::begin_signing,
            signing::adapters::tauri::sign_with_pin,
            signing::adapters::tauri::finish_signing,
            signing::adapters::tauri::cancel_signing,
            documents::adapters::tauri::open_document,
            documents::adapters::tauri::read_document,
            signing::adapters::tauri::read_configuration,
            signing::adapters::tauri::write_configuration,
            signing::adapters::tauri::forget_activity,
            documents::adapters::tauri::list_recents,
            documents::adapters::tauri::record_recent,
            documents::adapters::tauri::forget_recent,
            documents::adapters::tauri::choose_rubric,
            documents::adapters::tauri::read_rubric,
            documents::adapters::tauri::preview_destination,
            documents::adapters::tauri::choose_destination,
            documents::adapters::tauri::open_signed_document,
            documents::adapters::tauri::open_signed_folder,
            signing::adapters::tauri::preview_signature,
            signing::adapters::tauri::pades_lower_left,
            desktop::adapters::tauri::read_invocation,
            desktop::adapters::tauri::check_for_new_version,
            desktop::adapters::tauri::url_handlers,
            desktop::adapters::tauri::choose_url_handler,
            signing::adapters::tauri::unregistered_signatures,
            identity::adapters::tauri::install_certificate,
            identity::adapters::tauri::remove_certificate,
            site::adapters::tauri::close_site_window,
            site::adapters::tauri::site_identify,
            site::adapters::tauri::site_decline,
            site::adapters::tauri::site_begin_signing,
            site::adapters::tauri::site_finish_signing,
            site::adapters::tauri::site_install_certificate,
            site::adapters::tauri::site_look_again,
            site::adapters::tauri::install_local_ca,
            site::adapters::tauri::read_site_errand,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            let transport = the_transport(&ca_store, &handle);
            let nss_stores =
                site::adapters::nss::NssTrustStores::new(identity::adapters::pkcs11::RealNssHost);
            let startup = site::application::startup::attend_startup(
                &invocation,
                site::application::startup::TrustAtStartup {
                    store: &ca_store,
                    profiles: &nss_profiles,
                    stores: &nss_stores,
                },
                &|ports, duty| transport.open(ports, duty),
                &|_| site::adapters::window::open_the_site_window(&handle),
                app.state::<site::application::errand::LiveErrand>().inner(),
            );

            say(startup.said);

            match startup.opening {
                site::application::startup::Opening::TheMainWindow => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                    }
                }
                site::application::startup::Opening::TheSiteErrand(attendance) => {
                    say(site::application::startup::hold_the_channel(
                        &app.state::<site::application::startup::HeldChannel>(),
                        attendance,
                    ));
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error arrancando la ventana de rfirma");
}

/// Los perfiles NSS de esta persona, o ninguno si no se sabe cuál es su `HOME`.
fn nss_profiles_of_this_home() -> Vec<std::path::PathBuf> {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .map(|home| identity::adapters::pkcs11::stores::nss_profiles(&home))
        .unwrap_or_default()
}

/// Transporte de producción sobre loopback wss.
fn the_transport(
    store: &site::adapters::tls::LocalCaStore,
    app: &tauri::AppHandle,
) -> site::adapters::transport::LoopbackWss {
    let handle = app.clone();
    site::adapters::transport::LoopbackWss::new(
        store.clone(),
        std::sync::Arc::new(move |url, reply| {
            site::adapters::window::attend_site_operation(&handle, url, reply);
        }),
    )
}

/// Lo que los casos de uso dejan dicho para `stderr`, impreso y nada más.
fn say(lines: Vec<String>) {
    for line in lines {
        eprintln!("{line}");
    }
}
