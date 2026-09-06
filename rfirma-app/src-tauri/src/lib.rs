//! Composición y arranque de la aplicación Tauri.

pub mod app;
pub mod channel;
pub mod commands;
pub mod desktop;
pub mod destination;
pub mod dropped;
pub mod ffi;
pub mod isolate;
pub mod memory;
pub mod paths;
pub mod pkcs11;
pub mod protocol;
pub mod releases;
pub mod rubric;
pub mod signing;
pub mod tls;
pub mod trust;

/// Variable de entorno para sobreescribir el módulo PKCS#11.
pub const PKCS11_MODULE_VARIABLE: &str = "RFIRMA_PKCS11_MODULE";

/// Punto de entrada compartido por el binario y por las pruebas.
pub fn run() {
    use app::errand::Transport as _;
    use tauri::{Emitter, Manager};

    if app::invocation::help_was_asked_for(
        std::env::args_os().map(|argument| argument.to_string_lossy().into_owned()),
    ) {
        println!("{}", app::invocation::HELP);
        return;
    }

    app::invocation::make_the_command_line_readable();

    let paths = paths::Paths::from_environment().expect("debería saberse cuál es el HOME");

    let ca_store = tls::LocalCaStore::of(&paths);
    let nss_profiles = nss_profiles_of_this_home();

    let invocation = app::invocation::Invocation::of_this_process();
    let second_store = ca_store.clone();

    let local_ca_trust = app::startup::LocalCaTrust {
        store: ca_store.clone(),
        profiles: nss_profiles.clone(),
    };

    let memory = memory::Memory::at(&paths);
    let configuration = memory
        .configuration()
        .map(memory::Loaded::into_value)
        .unwrap_or_default();
    let environment = app::Environment {
        stores: pkcs11::stores::from_environment(),
        listed: memory::ListedCertificates::new(),
        documents_folder: paths::documents_folder().unwrap_or_default(),
        configuration: std::sync::Mutex::new(configuration),
        memory,
        rubric: rubric::RubricStore::at(paths.rubric_path()),
        installed_certificates: paths.installed_certificates_dir(),
    };

    tauri::Builder::default()
        // Instancia única (ADR-0010).
        .plugin(tauri_plugin_single_instance::init(
            move |app, command_line, folder| {
                use tauri::Manager as _;
                let invocation = app::invocation::Invocation {
                    command_line,
                    folder: std::path::PathBuf::from(folder),
                };
                let session = app.state::<commands::SigningSession>();
                let opened = app.state::<memory::OpenedDocuments>();
                let substitution = commands::second_invocation(
                    &invocation,
                    &opened,
                    app::signing::is_live(&session),
                );
                match substitution {
                    app::invocation::SecondInvocation::ReplacesWhatWasThere(view) => {
                        let Some(window) = app.get_webview_window("main") else {
                            return;
                        };
                        let _ = window.set_focus();
                        let _ = window.emit(commands::DOCUMENT_DROPPED, *view);
                    }
                    app::invocation::SecondInvocation::OpensItsOwnWindow(url) => {
                        let handle = app.clone();
                        let transport = the_transport(&second_store, app);
                        let attendance = app::startup::attend_site_launch(
                            &url,
                            &|ports, duty| transport.open(ports, duty),
                            &|_| commands::open_the_site_window(&handle),
                            app.state::<app::errand::LiveErrand>().inner(),
                            // A mitad de un trámite no se toca la CA local (ADR-0005).
                            app::startup::LocalCaReach::NotAnObstacle,
                        );
                        say(app::startup::hold_the_channel(
                            &app.state::<app::startup::HeldChannel>(),
                            attendance,
                        ));
                    }
                    app::invocation::SecondInvocation::NothingHappens => {
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
        .manage(isolate::Isolate::start())
        .manage(commands::SigningSession::default())
        .manage(commands::PendingInvocation::of(invocation.clone()))
        .manage(app::errand::LiveErrand::default())
        .manage(app::startup::HeldChannel::default())
        .manage(local_ca_trust)
        .manage(memory::OpenedDocuments::new())
        // En Tauri el arrastre se gestiona por evento de ventana (ADR-0011).
        .on_window_event(|window, event| {
            if window.label() == commands::SITE_WINDOW
                && matches!(event, tauri::WindowEvent::CloseRequested { .. })
            {
                app::errand::decline(&window.state::<app::errand::LiveErrand>());
                return;
            }

            let tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) = event
            else {
                return;
            };
            let opened = window.state::<memory::OpenedDocuments>();
            let Some(dropped) = commands::dropped_document(paths, &opened) else {
                return;
            };
            let _ = window.emit(commands::DOCUMENT_DROPPED, dropped);
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_certificates,
            commands::begin_signing,
            commands::sign_with_pin,
            commands::finish_signing,
            commands::cancel_signing,
            commands::open_document,
            commands::read_document,
            commands::read_configuration,
            commands::write_configuration,
            commands::forget_activity,
            commands::list_recents,
            commands::record_recent,
            commands::forget_recent,
            commands::choose_rubric,
            commands::read_rubric,
            commands::preview_destination,
            commands::choose_destination,
            commands::open_signed_document,
            commands::open_signed_folder,
            commands::preview_signature,
            commands::pades_lower_left,
            commands::read_invocation,
            commands::check_for_new_version,
            commands::url_handlers,
            commands::choose_url_handler,
            commands::unregistered_signatures,
            commands::install_certificate,
            commands::remove_certificate,
            commands::close_site_window,
            commands::site_identify,
            commands::site_decline,
            commands::site_begin_signing,
            commands::site_finish_signing,
            commands::site_install_certificate,
            commands::site_look_again,
            commands::install_local_ca,
            commands::read_site_errand,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            let transport = the_transport(&ca_store, &handle);
            let nss_stores = trust::NssTrustStores::new(pkcs11::RealNssHost);
            let startup = app::startup::attend_startup(
                &invocation,
                app::startup::TrustAtStartup {
                    store: &ca_store,
                    profiles: &nss_profiles,
                    stores: &nss_stores,
                },
                &|ports, duty| transport.open(ports, duty),
                &|_| commands::open_the_site_window(&handle),
                app.state::<app::errand::LiveErrand>().inner(),
            );

            say(startup.said);

            match startup.opening {
                app::startup::Opening::TheMainWindow => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                    }
                }
                app::startup::Opening::TheSiteErrand(attendance) => {
                    say(app::startup::hold_the_channel(
                        &app.state::<app::startup::HeldChannel>(),
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
        .map(|home| pkcs11::stores::nss_profiles(&home))
        .unwrap_or_default()
}

/// Transporte de producción sobre loopback wss.
fn the_transport(store: &tls::LocalCaStore, app: &tauri::AppHandle) -> app::transport::LoopbackWss {
    let handle = app.clone();
    app::transport::LoopbackWss::new(
        store.clone(),
        std::sync::Arc::new(move |url, reply| {
            commands::attend_site_operation(&handle, url, reply);
        }),
    )
}

/// Lo que los casos de uso dejan dicho para `stderr`, impreso y nada más.
fn say(lines: Vec<String>) {
    for line in lines {
        eprintln!("{line}");
    }
}
