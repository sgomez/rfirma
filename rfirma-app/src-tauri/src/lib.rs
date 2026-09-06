//! rfirma: firma y cofirma de PDFs en PAdES con firma visible.
//!
//! El backend está en tres alturas, y se leen de arriba abajo:
//!
//! - [`commands`], las veintidós órdenes de Tauri. Desempaquetan el estado, llaman a
//!   un caso de uso y traducen el resultado. No deciden nada (ID-79).
//! - [`app`], los casos de uso: qué certificados hay, cómo se planifica y se
//!   entrega una firma, qué se recuerda entre sesiones. Es la interfaz por la
//!   que se prueba (ID-77).
//! - Los módulos de dominio e infraestructura, que no saben que las dos
//!   alturas de arriba existen (ID-81): [`pkcs11`], la capa que habla con el
//!   token; [`rubric`], la normalización y el almacén de la imagen de la firma;
//!   [`signing`], las reglas puras de la firma y el sello de sesión; [`ffi`] y
//!   [`isolate`], la frontera con la librería nativa y el hilo que la posee;
//!   [`memory`], lo que se recuerda entre sesiones; [`destination`], por dónde
//!   entra el documento y dónde cae el firmado; [`dropped`], qué se abre de lo
//!   que se suelta en la ventana; [`tls`], la CA local y el certificado del
//!   servidor local que el canal necesita; [`desktop`], el canal de
//!   distribución y quién dice el escritorio que atiende `afirma://`; y
//!   [`paths`], el único sitio que sabe qué sistema operativo hay debajo.
//!
//! Si te encuentras escribiendo Rust que sabe qué es un PDF, te has salido de
//! estos módulos.

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

/// La escotilla para apuntar a **otro** módulo PKCS#11.
///
/// Cuando está puesta manda ella sola y se ignoran los candidatos de
/// [`pkcs11::stores`]: quien la exporta quiere ese módulo. Es de lo que
/// dependen las pruebas de grada B y C contra SoftHSM.
pub const PKCS11_MODULE_VARIABLE: &str = "RFIRMA_PKCS11_MODULE";

/// Punto de entrada compartido por el binario y por las pruebas.
pub fn run() {
    use app::errand::Transport as _;
    use tauri::{Emitter, Manager};

    // **Antes que nada**: si lo que se pide es la ayuda, se imprime y se
    // termina. No se monta ventana ni se toca la memoria, y tampoco se vuelve
    // a arrancar por los argumentos ilegibles: una bandera de ayuda es UTF-8
    // por construcción.
    if app::invocation::help_was_asked_for(
        std::env::args_os().map(|argument| argument.to_string_lossy().into_owned()),
    ) {
        println!("{}", app::invocation::HELP);
        return;
    }

    // **Lo primero de todo** (ID-236): el complemento de instancia única lee la
    // línea de órdenes con `std::env::args()`, que entra en pánico con un
    // argumento que no sea UTF-8, y lo hace dentro de su propio `setup`. Para
    // cuando mire, aquí ya se ha vuelto a arrancar con los argumentos
    // legibles; el caso normal no hace nada.
    app::invocation::make_the_command_line_readable();

    let paths = paths::Paths::from_environment().expect("debería saberse cuál es el HOME");

    // El material de la CA local y los perfiles NSS donde tiene que estar de
    // confianza (ID-329). Quién los recorre y en qué orden lo decide
    // [`app::startup::attend_startup`]: aquí solo se resuelven las rutas.
    let ca_store = tls::LocalCaStore::of(&paths);
    let nss_profiles = nss_profiles_of_this_home();

    // La invocación de **este** proceso, leída una sola vez: la recogen el
    // arranque —que decide si es de sede (ID-324)— y la ventana, con
    // `read_invocation` (ID-157).
    let invocation = app::invocation::Invocation::of_this_process();

    // Su copia para la segunda invocación, que atiende con el mismo transporte
    // (ID-327) desde el manejador del complemento de instancia única.
    let second_store = ca_store.clone();

    // Y su copia para la orden 36, que instala la CA local cuando la persona lo
    // pide desde la pantalla de reparación de la ventana de sede (ID-341).
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
        // Los almacenes se resuelven **aquí**, en el binario, y no en la
        // receta que lo arranca: es lo que hace que `just dev` encuentre el
        // token del anfitrión sin exportar nada, y con el mismo código que
        // corre instalado (ID-13).
        stores: pkcs11::stores::from_environment(),
        // El último listado de certificados, vacío hasta la primera búsqueda.
        // Es donde se quedan las referencias que la ventana no puede tener.
        listed: memory::ListedCertificates::new(),
        documents_folder: paths::documents_folder().unwrap_or_default(),
        configuration: std::sync::Mutex::new(configuration),
        // La memoria viaja con el entorno y no aparte: las órdenes que guardan
        // ajustes tienen que actualizar la copia viva y el fichero a la vez, y
        // dos estados separados invitan a hacer solo una de las dos cosas.
        memory,
        // Se copia, no se referencia (ID-33): el almacén vive en una ruta fija
        // del directorio de datos, resuelta aquí una sola vez.
        rubric: rubric::RubricStore::at(paths.rubric_path()),
        // Los `.p12` instalados viven en el directorio de datos y se releen en
        // cada listado, no aquí: instalar y quitar son gestos de esta misma
        // sesión (ID-192).
        installed_certificates: paths.installed_certificates_dir(),
    };

    tauri::Builder::default()
        // **Instancia única, con sustitución y sin preguntar** (ID-160). Dos
        // procesos escribiendo la misma memoria acaban con el último en cerrar
        // pisando al otro, y el `.deb` quita el aislamiento que hoy lo tapa a
        // medias. Va **el primero** de los complementos: es lo que exige el
        // suyo para que la segunda instancia se muera antes de montar nada.
        //
        // La excepción del ID-160 se decide en `app::invocation`, no aquí: con
        // una sesión de firma viva no se sustituye nada, porque es el único
        // estado donde perder el hilo cuesta un PIN.
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
                    // Por la **misma** puerta que el arrastre: el estado en que
                    // queda la ventana es el mismo, no uno parecido (ID-159).
                    //
                    // Sin ventana no hay a quién entregarle nada, y anotar el
                    // documento en `OpenedDocuments` antes de saberlo dejaría
                    // una entrada que no recoge nadie: primero la ventana.
                    app::invocation::SecondInvocation::ReplacesWhatWasThere(view) => {
                        let Some(window) = app.get_webview_window("main") else {
                            return;
                        };
                        // Traerla al frente ocurre también con la firma a
                        // medias: quien invoca quiere ver la aplicación, y
                        // enseñarle el PIN que dejó a medias es la respuesta
                        // correcta.
                        let _ = window.set_focus();
                        let _ = window.emit(commands::DOCUMENT_DROPPED, *view);
                    }
                    // Una invocación de sede **no sustituye nunca** lo que la
                    // ventana tuviera delante (ID-279): abre lo suyo, con el
                    // mismo transporte y el mismo trámite vivo que el arranque
                    // (ID-327). Qué pasa con un trámite ya vivo lo decide
                    // `LiveErrand::begin` y nadie más (ID-280), y aquí no se
                    // refresca la CA local: eso es del arranque y nunca de
                    // mitad de un trámite (ID-224, ID-329).
                    app::invocation::SecondInvocation::OpensItsOwnWindow(url) => {
                        let handle = app.clone();
                        let transport = the_transport(&second_store, app);
                        let attendance = app::startup::attend_site_launch(
                            &url,
                            &|ports, duty| transport.open(ports, duty),
                            &|_| commands::open_the_site_window(&handle),
                            app.state::<app::errand::LiveErrand>().inner(),
                            // A mitad de un trámite no se abre ni un almacén
                            // NSS (ID-224), así que de la CA local no se sabe
                            // nada aquí y no se cuenta como obstáculo.
                            app::startup::LocalCaReach::NotAnObstacle,
                        );
                        say(app::startup::hold_the_channel(
                            &app.state::<app::startup::HeldChannel>(),
                            attendance,
                        ));
                    }
                    // Y quien invoca sin nada que abrir quiere ver lo que ya
                    // había, que es lo que la ventana principal tiene delante.
                    app::invocation::SecondInvocation::NothingHappens => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.set_focus();
                        }
                    }
                }
            },
        ))
        // El diálogo de fichero se abre desde Rust (ID-63), así que el
        // complemento entra aquí y no en el frontal: la lista de permisos de la
        // ventana no crece por esto.
        .plugin(tauri_plugin_dialog::init())
        // Y el que abre el PDF firmado y su carpeta, por la misma puerta: bajo
        // el sandbox es el portal `OpenURI`, y es lo único que lleva al usuario
        // hasta un fichero cuya ruta nunca ve (ID-79, ID-85, ADR-0011).
        .plugin(tauri_plugin_opener::init())
        .manage(environment)
        // El hilo del isolate arranca con la ventana y **no abre la librería
        // todavía**: quien solo quiere mirar un PDF no paga el dlopen de 27,7
        // MB, y una librería que falta se cuenta como un error de firma y no
        // como una ventana que no abre.
        .manage(isolate::Isolate::start())
        .manage(commands::SigningSession::default())
        // Lo que traía la línea de órdenes que abrió esta ventana, hasta que la
        // ventana lo recoja con `read_invocation` (ID-157). Se lee aquí, en el
        // arranque, porque es el único momento en que los argumentos son los de
        // **esta** invocación.
        .manage(commands::PendingInvocation::of(invocation.clone()))
        // **El trámite de sede vivo del proceso** (ID-325, ID-280), con todo lo
        // que recuerda: qué se consintió y en qué momento está (RD-01). Vacío
        // en el arranque normal, que es la mayoría: sólo lo llena
        // `attend_launch`.
        .manage(app::errand::LiveErrand::default())
        // Y el canal abierto, sostenido: soltar un [`channel::OpenChannel`]
        // suelta con él su asa de apagado, y el servidor deja de aceptar
        // conexiones.
        .manage(app::startup::HeldChannel::default())
        // Dónde vive la CA local y en qué perfiles NSS tiene que estar, para
        // que la ventana de sede pueda instalarla sin volver a resolver rutas.
        .manage(local_ca_trust)
        // Los documentos abiertos, del identificador opaco al documento del
        // portal (ID-61). Vive mientras vive el proceso.
        .manage(memory::OpenedDocuments::new())
        // El arrastre entra por aquí y no por un `onDrop` del JSX (ID-67): con
        // `dragDropEnabled` —que es lo que hay— el WebView **no** dispara los
        // eventos de arrastre de HTML, así que un manejador en el frontal no se
        // ejecutaría nunca y parecería un fallo de la interfaz. Lo que llega
        // aquí son rutas del anfitrión, y se quedan aquí: lo que cruza es el
        // documento ya apuntado (ADR-0011).
        .on_window_event(|window, event| {
            // **Cerrar la ventana de sede sin haber contestado es cancelar**
            // (ID-340): la sede recibe `CANCEL` en el acto, sin esperar a nada.
            // Despues de contestar ya no queda asa por la que escribir, asi que
            // cerrarla entonces —que es lo que hace la orden 29— no manda nada.
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
            // Emitir puede fallar si la ventana se está cerrando, y entonces no
            // hay nadie a quien contarle nada: no es un motivo para tumbar la
            // aplicación mientras se va.
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
        // **El arranque, que es un adaptador y no decide nada** (ID-324, TD-70):
        // el caso de uso refresca la CA local, mira si la invocación es de sede
        // y dice qué ventana se abre. Aquí sólo se le dan los tres puertos y se
        // obedece lo que devuelve.
        .setup(move |app| {
            let handle = app.handle().clone();
            let transport = the_transport(&ca_store, &handle);
            let startup = app::startup::attend_startup(
                &invocation,
                app::startup::TrustAtStartup {
                    store: &ca_store,
                    profiles: &nss_profiles,
                    stores: &trust::NssTrustStores,
                },
                &|ports, duty| transport.open(ports, duty),
                &|_| commands::open_the_site_window(&handle),
                app.state::<app::errand::LiveErrand>().inner(),
            );

            // Qué se dice de la CA local es una regla y vive en `app::trust`,
            // probada allí sin arrancar Tauri; aquí sólo se imprime (#397).
            say(startup.said);

            match startup.opening {
                // **Con una invocación de sede la principal no se enseña**
                // (ID-328): la ventana `main` nace oculta por configuración, y
                // éste es el único sitio que la muestra.
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

/// **El transporte de producción** (ID-326): el `wss` sobre el *loopback*, que
/// entrega lo que llega al adaptador del trámite. Quien atiende la operación es
/// el adaptador (ID-330): arma la mesa del trámite desde el estado de Tauri, y
/// el canal no sabe nada de trámites.
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
