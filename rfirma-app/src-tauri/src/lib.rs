//! Composición y arranque de la aplicación Tauri: construye las raíces de los cinco contextos y las registra.

pub mod crossing;
pub mod desktop;
pub mod documents;
pub mod identity;
pub mod memory_error;
pub mod signing;
pub mod site;
pub mod startup_dialog;
pub mod startup_failure;

#[cfg(doctest)]
mod compile_fail;
mod event_loop;

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

pub use desktop::adapters::terminal::run_the_command_line;
use desktop::application::invocation::{Invocation, Role};
use desktop::DesktopRoot;
use documents::DocumentsRoot;
use event_loop::{build_and_run, Serving};
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
    pub dialogs: Arc<documents::adapters::dialogs::RealPortalDialogs>,
    pub prompter: Arc<signing::adapters::gtk_prompter::NativePinDialog>,
}

/// Compone las cinco raíces de producción sobre las rutas de esta máquina, con la invocación
/// que traía el escritorio.
pub fn roots(paths: desktop::adapters::paths::Paths) -> Roots {
    composed_roots(paths, Some(desktop::adapters::process::this_invocation()))
}

/// Compone las cinco raíces, con la invocación pendiente del escritorio o vacía en rol de sede.
fn composed_roots(paths: desktop::adapters::paths::Paths, invocation: Option<Invocation>) -> Roots {
    let memory = Arc::new(signing::adapters::memory::Memory::at(&paths));
    let ca_store = site::adapters::tls::LocalCaStore::of(&paths);
    let dialogs = Arc::new(documents::adapters::dialogs::RealPortalDialogs::default());
    let prompter = Arc::new(signing::adapters::gtk_prompter::NativePinDialog::new());
    let identity = IdentityRoot {
        token: Box::new(identity::adapters::DesktopToken),
        stores: identity::adapters::desktop_stores(),
        installed_certificates: paths.installed_certificates_dir(),
        listed: identity::application::certificates::ListedCertificates::new(),
        installed_copies: identity::application::certificates::ListedCertificates::new(),
        memory: memory.clone(),
        folder: Arc::new(identity::adapters::folder::RealInstalledFolder),
        prompter: prompter.clone(),
        keyring: Arc::new(|| {
            identity::adapters::DesktopKeyring::new()
                .map(|keyring| Box::new(keyring) as Box<dyn identity::ports::Keyring + Send + Sync>)
        }),
    };
    let documents = DocumentsRoot {
        documents_folder: desktop::adapters::paths::documents_folder().unwrap_or_default(),
        rubric: documents::adapters::rubric::RubricStore::at(paths.rubric_path()),
        opened: documents::application::documents::OpenedDocuments::new(),
        single_destinations: documents::application::single_destination::SingleDestinations::new(),
        memory: memory.clone(),
        files: Arc::new(documents::adapters::files::RealFiles),
        portal: dialogs.clone(),
    };
    let desktop = DesktopRoot {
        pending_invocation: match invocation {
            Some(invocation) => desktop::application::invocation::PendingInvocation::of(invocation),
            None => desktop::application::invocation::PendingInvocation::default(),
        },
        memory: memory.clone(),
        paths,
    };
    let signing = SigningRoot {
        memory,
        isolate: signing::adapters::isolate::Isolate::start(),
        session: signing::application::session::SigningSession::default(),
        files: Arc::new(signing::adapters::files::RealDocumentBytes),
        prompter: prompter.clone(),
    };
    let site = SiteRoot {
        errand: site::application::errand::LiveErrand::default(),
        held_channel: site::application::startup::HeldChannel::default(),
        trust: site::application::startup::LocalCaTrust {
            store: Box::new(ca_store.clone()),
            profiles: site::adapters::trust_profiles(),
            stores: site::adapters::desktop_trust_stores(),
        },
        ca_store,
        codecs: site::application::site::CodecTable {
            v4: Arc::new(site::adapters::codec::V4Codec),
            v3: Arc::new(site::adapters::codec_v3::V3Codec),
            v1: Arc::new(|version| {
                Arc::new(site::adapters::codec_v1::V1Codec::new(version))
                    as site::application::errand::NegotiatedCodec
            }),
            relay: Arc::new(|key, version| {
                Arc::new(site::adapters::codec_relay::RelayCodec::new(key, version))
                    as site::application::errand::NegotiatedCodec
            }),
        },
        scratch_dir: std::env::temp_dir(),
        scratch: Arc::new(site::adapters::scratch::RealScratch),
        batch: Arc::new(site::adapters::batch_services::RelayBatchServices::default()),
        triphase: Arc::new(site::adapters::triphase_server::HttpTriphaseServer::default()),
        portal: dialogs.clone(),
    };
    Roots {
        identity,
        documents,
        desktop,
        signing,
        site,
        dialogs,
        prompter,
    }
}

/// Punto de entrada compartido por el binario y por las pruebas: decide el rol de proceso
/// (ADR-0024) y monta la raíz de composición que le corresponde, o atiende la orden de terminal.
pub fn run() {
    let invocation = desktop::adapters::process::this_invocation();
    let role = desktop::application::invocation::role_of(invocation.clone());
    // Una sola expansión por crate (ADR-0040).
    let context = tauri::generate_context!();
    if let Role::Terminal(_) = role {
        std::process::exit(run_the_command_line(&invocation.command_line, context));
    }

    if desktop::adapters::process::printed_the_informative_text(&invocation.command_line) {
        return;
    }

    desktop::adapters::process::make_the_command_line_readable();
    desktop::adapters::titlebar::keep_the_popovers_clear();

    let discarded = Role::said(&invocation);
    let paths = desktop::adapters::paths::Paths::from_environment().unwrap_or_else(|error| {
        startup_dialog::report_and_exit(&startup_failure::StartupFailure::new(
            startup_failure::Situation::HomeUnknown,
            error.to_string(),
        ))
    });

    run_the_window_role(role, paths, discarded, context);
}

fn run_the_window_role(
    role: Role,
    paths: desktop::adapters::paths::Paths,
    discarded: Vec<String>,
    context: tauri::Context<tauri::Wry>,
) {
    match role {
        Role::Desktop(invocation) => run_desktop(paths, invocation, context),
        Role::Site(url) => run_site(paths, url, discarded, context),
        Role::Foreign(url) => {
            eprintln!("rfirma: {url} no es una llamada afirma://; no se abre nada")
        }
        Role::Terminal(_) => unreachable!("el proceso de terminal ya ha salido"),
    }
}

/// Añade a un `Builder` lo que los dos roles gestionan por igual: las cinco raíces, los
/// complementos comunes, las órdenes de Tauri y el evento de arrastre (ADR-0024).
#[expect(clippy::too_many_lines)]
fn with_the_five_roots(
    builder: tauri::Builder<tauri::Wry>,
    roots: Roots,
) -> tauri::Builder<tauri::Wry> {
    use tauri::{Emitter, Manager};

    let Roots {
        identity,
        documents,
        desktop,
        signing,
        site,
        dialogs: _,
        prompter: _,
    } = roots;

    builder
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
            signing::adapters::tauri::remembered_visible_signature,
            signing::adapters::tauri::write_configuration,
            signing::adapters::tauri::forget_activity,
            documents::adapters::tauri::list_recents,
            documents::adapters::tauri::record_recent,
            documents::adapters::tauri::forget_recent,
            documents::adapters::tauri::choose_rubric,
            documents::adapters::tauri::read_rubric,
            documents::adapters::tauri::preview_destination,
            documents::adapters::tauri::choose_destination,
            documents::adapters::tauri::choose_single_destination,
            documents::adapters::tauri::open_signed_document,
            documents::adapters::tauri::open_signed_folder,
            signing::adapters::tauri::preview_signature,
            signing::adapters::tauri::pades_lower_left,
            desktop::adapters::tauri::read_invocation,
            desktop::adapters::tauri::apply_titlebar_state,
            desktop::adapters::tauri::check_for_new_version,
            desktop::adapters::tauri::install_new_version,
            desktop::adapters::tauri::measure_version,
            desktop::adapters::tauri::open_external_destination,
            desktop::adapters::tauri::read_status,
            desktop::adapters::tauri::measure_local_ca_certificate,
            desktop::adapters::tauri::install_local_ca_certificate,
            desktop::adapters::tauri::choose_site_signature_handler,
            desktop::adapters::tauri::withdraw_rfirma,
            signing::adapters::tauri::unregistered_signatures,
            signing::adapters::tauri::previous_signatures,
            identity::adapters::tauri::install_certificate,
            identity::adapters::tauri::remove_certificate,
            identity::adapters::tauri::empty_installed_store,
            site::adapters::tauri::close_site_window,
            site::adapters::tauri::site_identify,
            site::adapters::tauri::site_decline,
            site::adapters::tauri::site_begin_signing,
            site::adapters::tauri::site_confirm_signatures,
            site::adapters::tauri::site_mark_area,
            site::adapters::tauri::site_finish_signing,
            site::adapters::tauri::site_install_certificate,
            site::adapters::tauri::site_look_again,
            site::adapters::tauri::site_dismiss_the_warning,
            site::adapters::tauri::site_save_file,
            site::adapters::tauri::site_load_files,
            site::adapters::tauri::install_local_ca,
            site::adapters::tauri::read_site_errand,
        ])
}

/// Barre las carpetas de paso abandonadas y crea la de este proceso, con el prefijo del rol
/// dado (ADR-0024).
fn own_scratch(role: &str) -> site::adapters::scratch::ProcessFolder {
    let temp = std::env::temp_dir();
    site::adapters::scratch::sweep(&temp, &["site", "desktop"]);
    site::adapters::scratch::own_folder(&temp, role).unwrap_or_else(|error| {
        startup_dialog::report_and_exit(&startup_failure::StartupFailure::new(
            startup_failure::Situation::ScratchFolderUnusable,
            error.to_string(),
        ))
    })
}

/// Rol escritorio: instancia única (ADR-0010) y ventana principal. No construye transporte.
fn run_desktop(
    paths: desktop::adapters::paths::Paths,
    invocation: Invocation,
    context: tauri::Context<tauri::Wry>,
) {
    let scratch = own_scratch("desktop");
    let mut roots = composed_roots(paths, Some(invocation));
    roots.site.scratch_dir = scratch.path().to_path_buf();

    let builder = tauri::Builder::default().plugin(tauri_plugin_single_instance::init(
        move |app, command_line, folder| {
            use tauri::{Emitter, Manager as _};
            let invocation = Invocation {
                command_line,
                folder: std::path::PathBuf::from(folder),
            };
            let substitution = desktop::application::invocation::second_invocation(
                &invocation,
                app.state::<SigningRoot>().is_live(),
            );
            match substitution {
                desktop::application::invocation::SecondInvocation::ReplacesWhatWasThere(paths) => {
                    let Some(window) = app.get_webview_window("main") else {
                        return;
                    };
                    let _ = window.set_focus();
                    let Some(told) = app.state::<DocumentsRoot>().what_was_dropped(&paths) else {
                        return;
                    };
                    let _ = window.emit(
                        DOCUMENT_DROPPED,
                        documents::adapters::views::DroppedDocumentView::from(told),
                    );
                }
                desktop::application::invocation::SecondInvocation::NothingHappens => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.set_focus();
                    } else {
                        open_the_main_window(app);
                    }
                }
            }
        },
    ));

    let dialogs = roots.dialogs.clone();
    let prompter = roots.prompter.clone();
    let serving = Arc::new(Serving::default());
    let builder = desktop::adapters::installer::with_the_updater(builder, context.config());
    let builder = with_the_five_roots(builder, roots).manage(scratch).setup({
        let serving = Arc::clone(&serving);
        move |app| {
            if serving.only_for_links.load(Ordering::SeqCst) {
                app.handle().exit(0);
                return Ok(());
            }
            dialogs.attach(app.handle().clone());
            prompter.attach(app.handle().clone());
            open_the_main_window(app.handle());
            serving.window_is_up.store(true, Ordering::SeqCst);
            Ok(())
        }
    });
    build_and_run(builder, context, &serving);
}

/// Rol sede: sin instancia única. Atiende el trámite y sostiene el único transporte del
/// proceso; `Opening::TheMainWindow` no puede darse con una URL de sede.
fn run_site(
    paths: desktop::adapters::paths::Paths,
    url: String,
    said_by_the_role: Vec<String>,
    context: tauri::Context<tauri::Wry>,
) {
    use tauri::Manager;

    let scratch = own_scratch("site");
    let mut roots = composed_roots(paths, None);
    roots.site.scratch_dir = scratch.path().to_path_buf();
    let dialogs = roots.dialogs.clone();
    let prompter = roots.prompter.clone();
    let builder =
        desktop::adapters::installer::with_the_updater(tauri::Builder::default(), context.config());

    let builder = with_the_five_roots(builder, roots)
        .manage(scratch)
        .setup(move |app| {
            dialogs.attach(app.handle().clone());
            prompter.attach(app.handle().clone());
            say(said_by_the_role);

            let handle = app.handle().clone();
            let window = Arc::new(site::adapters::window::TauriSiteWindow::new(handle.clone()));
            site::adapters::trace::note_the_launch(&url);

            let launch = {
                let (handle, window, url) = (handle.clone(), Arc::clone(&window), url.clone());
                move || attend_the_site_launch(&handle, &url, window)
            };
            site::application::startup::warn_before_launching(
                &url,
                window,
                &app.state::<SiteRoot>().errand,
                Box::new(launch),
            );

            Ok(())
        });
    build_and_run(builder, context, &Arc::new(Serving::from_the_start()));
}

/// Atiende la invocación de sede y sostiene su canal.
fn attend_the_site_launch(
    handle: &tauri::AppHandle,
    url: &str,
    window: Arc<site::adapters::window::TauriSiteWindow>,
) {
    use tauri::Manager;

    let site = handle.state::<SiteRoot>();
    let transport = the_transport(&site.ca_store, handle);
    let startup = site::application::startup::attend_startup(
        Some(url),
        site::application::startup::TrustAtStartup {
            store: site.trust.store.as_ref(),
            profiles: &site.trust.profiles,
            stores: site.trust.stores.as_ref(),
        },
        &site.codecs,
        &transport,
        window,
        &site.errand,
    );

    say(startup.said);

    let site::application::startup::Opening::TheSiteErrand(attendance) = startup.opening else {
        unreachable!("una URL de sede siempre atiende el trámite")
    };
    say(site::application::startup::hold_the_channel(
        &site.held_channel,
        attendance,
    ));
}

/// Abre la ventana principal de la aplicación.
fn open_the_main_window(app: &tauri::AppHandle) {
    use tauri::{Manager as _, WebviewUrl, WebviewWindowBuilder};

    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }

    let built = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("rFirma")
        .inner_size(1280.0, 720.0)
        .min_inner_size(1100.0, 560.0)
        .visible(!desktop::adapters::titlebar::BUILT_HIDDEN)
        .build();

    match built {
        Ok(window) => desktop::adapters::titlebar::mount_and_show(&window),
        Err(error) => eprintln!("rfirma: no se puede abrir la ventana principal ({error})"),
    }
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
    use tauri::Manager as _;

    let inbox = {
        let handle = app.clone();
        let arrived_handle = app.clone();
        let left_handle = app.clone();
        let idle_handle = app.clone();
        site::ports::Inbox::of(
            move || {
                arrived_handle
                    .state::<site::SiteRoot>()
                    .errand
                    .browser_arrived();
            },
            move |url, origin, reply| {
                site::adapters::window::attend_site_operation(&handle, url, origin, reply);
            },
        )
        .when_the_first_client_leaves(move || {
            left_handle
                .state::<site::SiteRoot>()
                .errand
                .the_first_client_left();
        })
        .when_the_channel_idles(move || {
            idle_handle
                .state::<site::SiteRoot>()
                .errand
                .the_channel_went_idle();
        })
    };

    let wss = site::adapters::transport::LoopbackWss::new(store.clone(), inbox.clone());

    let service = site::adapters::service::RawTlsService::new(store.clone(), inbox.clone());

    let relay = site::adapters::relay::Relay::new(
        Arc::new(site::adapters::servlets::RelayServlets::default()),
        inbox,
        {
            let handle = app.clone();
            Arc::new(move |refusal| {
                site::adapters::window::note_a_relay_failure(&handle, refusal);
            })
        },
        tauri::async_runtime::handle().inner().clone(),
    )
    .minding_the_live_errand({
        let handle = app.clone();
        move || handle.state::<site::SiteRoot>().errand.current().is_some()
    });

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
