//! Composición y arranque de la aplicación Tauri: construye las raíces de los cinco contextos y las registra.

pub mod crossing;
pub mod desktop;
pub mod documents;
pub mod identity;
pub mod memory_error;
pub mod signing;
pub mod site;

#[cfg(doctest)]
mod compile_fail;

use std::sync::{Arc, Mutex};

use desktop::DesktopRoot;
use documents::DocumentsRoot;
use identity::IdentityRoot;
use signing::SigningRoot;
use site::SiteRoot;

/// Variable de entorno para sobreescribir el módulo PKCS#11.
pub const PKCS11_MODULE_VARIABLE: &str = "RFIRMA_PKCS11_MODULE";

/// Nombre del evento emitido cuando se suelta un documento en la ventana.
pub const DOCUMENT_DROPPED: &str = "document-dropped";

/// Adquiere el cerrojo recuperando el valor si el mutex estaba envenenado.
pub fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Las cinco raíces, construidas en orden de dependencia sobre las mismas rutas y la misma memoria.
pub struct Roots {
    pub identity: IdentityRoot,
    pub documents: DocumentsRoot,
    pub desktop: DesktopRoot,
    pub signing: SigningRoot,
    pub site: SiteRoot,
}

/// Compone las cinco raíces de producción sobre las rutas de esta máquina.
pub fn roots(paths: desktop::adapters::paths::Paths) -> Roots {
    let memory = Arc::new(signing::adapters::memory::Memory::at(&paths));
    let ca_store = site::adapters::tls::LocalCaStore::of(&paths);
    let identity = IdentityRoot {
        token: Box::new(identity::adapters::pkcs11::RealToken),
        stores: identity::adapters::pkcs11::stores::from_environment(),
        installed_certificates: paths.installed_certificates_dir(),
        listed: identity::application::certificates::ListedCertificates::new(),
        memory: memory.clone(),
        folder: Arc::new(identity::adapters::folder::RealInstalledFolder),
    };
    let documents = DocumentsRoot {
        documents_folder: desktop::adapters::paths::documents_folder().unwrap_or_default(),
        rubric: documents::adapters::rubric::RubricStore::at(paths.rubric_path()),
        opened: documents::application::documents::OpenedDocuments::new(),
        memory: memory.clone(),
        files: Arc::new(documents::adapters::files::RealFiles),
    };
    let desktop = DesktopRoot {
        pending_invocation: desktop::application::invocation::PendingInvocation::of(
            desktop::adapters::process::this_invocation(),
        ),
        memory: memory.clone(),
        paths,
    };
    let signing = SigningRoot {
        memory,
        isolate: signing::adapters::isolate::Isolate::start(),
        session: signing::application::session::SigningSession::default(),
        files: Arc::new(signing::adapters::files::RealDocumentBytes),
    };
    let site = SiteRoot {
        errand: site::application::errand::LiveErrand::default(),
        held_channel: site::application::startup::HeldChannel::default(),
        trust: site::application::startup::LocalCaTrust {
            store: Box::new(ca_store.clone()),
            profiles: nss_profiles_of_this_home(),
            stores: Box::new(site::adapters::nss::NssTrustStores::new(
                identity::adapters::pkcs11::RealNssHost,
            )),
        },
        ca_store,
        codecs: site::application::site::CodecTable {
            v4: Arc::new(site::adapters::codec::V4Codec),
            v3: Arc::new(site::adapters::codec_v3::V3Codec),
            v1: Arc::new(site::adapters::codec_v1::V1Codec),
            relay: Arc::new(|key| {
                Arc::new(site::adapters::codec_relay::RelayCodec::new(key))
                    as site::application::errand::NegotiatedCodec
            }),
        },
        scratch_dir: std::env::temp_dir(),
        scratch: Arc::new(site::adapters::scratch::RealScratch),
    };
    Roots {
        identity,
        documents,
        desktop,
        signing,
        site,
    }
}

/// Punto de entrada compartido por el binario y por las pruebas.
pub fn run() {
    use tauri::{Emitter, Manager};

    if desktop::application::invocation::help_was_asked_for(
        desktop::adapters::process::these_arguments(),
    ) {
        println!("{}", desktop::application::invocation::HELP);
        return;
    }

    desktop::adapters::process::make_the_command_line_readable();

    let paths = desktop::adapters::paths::Paths::from_environment()
        .expect("debería saberse cuál es el HOME");
    let Roots {
        identity,
        documents,
        desktop,
        signing,
        site,
    } = roots(paths);
    let invocation = desktop::adapters::process::this_invocation();

    tauri::Builder::default()
        // Instancia única (ADR-0010).
        .plugin(tauri_plugin_single_instance::init(
            move |app, command_line, folder| {
                use tauri::Manager as _;
                let invocation = desktop::application::invocation::Invocation {
                    command_line,
                    folder: std::path::PathBuf::from(folder),
                };
                let substitution = desktop::application::invocation::second_invocation(
                    &invocation,
                    app.state::<SigningRoot>().is_live(),
                );
                match substitution {
                    desktop::application::invocation::SecondInvocation::ReplacesWhatWasThere(
                        paths,
                    ) => {
                        let Some(window) = app.get_webview_window("main") else {
                            return;
                        };
                        let _ = window.set_focus();
                        let Some(told) = app.state::<DocumentsRoot>().what_was_dropped(&paths)
                        else {
                            return;
                        };
                        let _ = window.emit(
                            DOCUMENT_DROPPED,
                            documents::adapters::views::DroppedDocumentView::from(told),
                        );
                    }
                    desktop::application::invocation::SecondInvocation::OpensItsOwnWindow(url) => {
                        let handle = app.clone();
                        let site = app.state::<SiteRoot>();
                        let transport = the_transport(&site.ca_store, &handle);
                        let attendance = site::application::startup::attend_site_launch(
                            &url,
                            &site.codecs,
                            &transport,
                            &|_| site::adapters::window::open_the_site_window(&handle),
                            &site.errand,
                            // A mitad de un trámite no se toca la CA local (ADR-0005).
                            site::application::startup::LocalCaReach::NotAnObstacle,
                        );
                        say(site::application::startup::hold_the_channel(
                            &site.held_channel,
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
        .plugin(tauri_plugin_opener::init())
        .manage(identity)
        .manage(documents)
        .manage(desktop)
        .manage(signing)
        .manage(site)
        .on_window_event(|window, event| {
            let tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) = event
            else {
                return;
            };
            let documents = window.state::<DocumentsRoot>();
            let Some(dropped) = documents.what_was_dropped(paths) else {
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
            let site = app.state::<SiteRoot>();
            let transport = the_transport(&site.ca_store, &handle);
            let startup = site::application::startup::attend_startup(
                invocation.site_launch(),
                site::application::startup::TrustAtStartup {
                    store: site.trust.store.as_ref(),
                    profiles: &site.trust.profiles,
                    stores: site.trust.stores.as_ref(),
                },
                &site.codecs,
                &transport,
                &|_| site::adapters::window::open_the_site_window(&handle),
                &site.errand,
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
                        &site.held_channel,
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

/// Transporte de producción: `wss` sobre loopback o servidor intermedio, según la ubicación de canal.
fn the_transport(
    store: &site::adapters::tls::LocalCaStore,
    app: &tauri::AppHandle,
) -> impl Fn(
    &site::domain::channel::ChannelLocation,
    site::domain::channel::ChannelDuty,
) -> Result<site::domain::channel::OpenChannel, site::domain::channel::ChannelError>
       + 'static {
    use site::application::errand::Transport as _;

    let inbox: site::application::errand::Inbox = {
        let handle = app.clone();
        Arc::new(move |url, reply| {
            site::adapters::window::attend_site_operation(&handle, url, reply);
        })
    };

    let wss = site::adapters::transport::LoopbackWss::new(store.clone(), inbox.clone());

    let service = site::adapters::service::RawTlsService::new(store.clone(), inbox.clone());

    let relay = site::adapters::relay::Relay::new(
        Arc::new(site::adapters::servlets::RelayServlets::default()),
        inbox,
        {
            let handle = app.clone();
            Arc::new(move || handle.exit(0))
        },
        {
            let handle = app.clone();
            Arc::new(move |refusal| {
                site::adapters::window::note_a_relay_failure(&handle, refusal);
            })
        },
    );

    move |location, duty| match location {
        site::domain::channel::ChannelLocation::Relay(_) => relay.open(location, duty),
        site::domain::channel::ChannelLocation::Service(_) => service.open(location, duty),
        _ => wss.open(location, duty),
    }
}

/// Lo que los casos de uso dejan dicho para `stderr`, impreso y nada más.
fn say(lines: Vec<String>) {
    for line in lines {
        eprintln!("{line}");
    }
}
