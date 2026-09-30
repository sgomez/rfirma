use gtk::prelude::*;
use serde::Serialize;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

#[derive(Serialize)]
struct DesktopInfo {
    variant: String,
    gnome: bool,
    xdg_current_desktop: String,
    decoration_layout: String,
    double_click: String,
    middle_click: String,
    right_click: String,
    backend: String,
}

fn variant() -> String {
    std::env::var("POC_VARIANT").unwrap_or_else(|_| "html".into())
}

fn is_gnome() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .split(':')
        .any(|d| d.eq_ignore_ascii_case("GNOME"))
}

#[tauri::command]
fn desktop_info(window: tauri::WebviewWindow) -> DesktopInfo {
    let s = gtk::Settings::default().unwrap();
    let backend = window
        .gtk_window()
        .ok()
        .map(|w| w.display().type_().name().to_string())
        .unwrap_or_default();
    DesktopInfo {
        variant: variant(),
        gnome: is_gnome(),
        xdg_current_desktop: std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default(),
        decoration_layout: s.gtk_decoration_layout().map(|g| g.to_string()).unwrap_or_default(),
        double_click: s.gtk_titlebar_double_click().map(|g| g.to_string()).unwrap_or_default(),
        middle_click: s.gtk_titlebar_middle_click().map(|g| g.to_string()).unwrap_or_default(),
        right_click: s.gtk_titlebar_right_click().map(|g| g.to_string()).unwrap_or_default(),
        backend,
    }
}

const HEADER_HEIGHT: f64 = 44.0;

fn attach_header_menu(window: &tauri::WebviewWindow) {
    let gtk_window = window.gtk_window().unwrap();
    let vbox = window.default_vbox().unwrap();
    let Some(webview) = vbox.children().into_iter().next() else { return };
    let gw = gtk_window.clone();
    webview.connect_button_press_event(move |_, event| {
        let (_, y) = event.position();
        if y < HEADER_HEIGHT && (event.button() == 3 || event.button() == 2) {
            eprintln!("poc: button {} on header at y={y}", event.button());
            if event.button() == 3 {
                if let Some(gdk_window) = gw.window() {
                    let mut ev: gtk::gdk::Event = (**event).clone();
                    let shown = gdk_window.show_window_menu(&mut ev);
                    eprintln!("poc: show_window_menu -> {shown}");
                    return gtk::glib::Propagation::Stop;
                }
            }
        }
        gtk::glib::Propagation::Proceed
    });
}

#[derive(Serialize, Clone)]
struct HeaderAction {
    action: String,
    path: Option<String>,
}

#[derive(serde::Deserialize)]
struct HeaderLabels {
    open: String,
    recent_title: String,
    preferences: String,
    about: String,
}

#[derive(serde::Deserialize)]
struct HeaderState {
    open_visible: bool,
    recents: Vec<String>,
    labels: HeaderLabels,
}

struct HeaderWidgets {
    split: gtk::Box,
    open: gtk::Button,
    arrow: gtk::MenuButton,
    menu: gtk::MenuButton,
}

thread_local! {
    static HEADER: std::cell::RefCell<Option<HeaderWidgets>> = const { std::cell::RefCell::new(None) };
}

fn emit_header(app: &tauri::AppHandle, action: &str, path: Option<String>) {
    use tauri::Emitter;
    let _ = app.emit("header", HeaderAction { action: action.into(), path });
}

fn recents_model(title: &str, recents: &[String]) -> gio::Menu {
    let section = gio::Menu::new();
    for path in recents {
        let item = gio::MenuItem::new(Some(path), None);
        item.set_action_and_target_value(Some("hdr.recent"), Some(&path.to_variant()));
        section.append_item(&item);
    }
    let model = gio::Menu::new();
    model.append_section(Some(title), &section);
    model
}

fn app_menu_model(preferences: &str, about: &str) -> gio::Menu {
    let model = gio::Menu::new();
    model.append(Some(preferences), Some("hdr.preferences"));
    model.append(Some(about), Some("hdr.about"));
    model
}

#[tauri::command]
fn set_header_state(state: HeaderState) {
    HEADER.with(|h| {
        let h = h.borrow();
        let Some(w) = h.as_ref() else { return };
        w.split.set_visible(state.open_visible);
        w.open.set_label(&state.labels.open);
        w.arrow.set_menu_model(Some(&recents_model(&state.labels.recent_title, &state.recents)));
        w.arrow.set_visible(!state.recents.is_empty());
        w.menu.set_menu_model(Some(&app_menu_model(&state.labels.preferences, &state.labels.about)));
    });
}

fn header_actions(app: &tauri::AppHandle) -> gio::SimpleActionGroup {
    let group = gio::SimpleActionGroup::new();
    for name in ["open", "preferences", "about"] {
        let action = gio::SimpleAction::new(name, None);
        let a = app.clone();
        action.connect_activate(move |_, _| emit_header(&a, name, None));
        group.add_action(&action);
    }
    let recent = gio::SimpleAction::new("recent", Some(gtk::glib::VariantTy::STRING));
    let a = app.clone();
    recent.connect_activate(move |_, p| emit_header(&a, "recent", p.and_then(|v| v.get::<String>())));
    group.add_action(&recent);
    group
}

fn native_headerbar(window: &tauri::WebviewWindow) {
    let gtk_window = window.gtk_window().unwrap();
    eprintln!(
        "poc: before set_titlebar realized={} existing_titlebar={:?}",
        gtk_window.is_realized(),
        gtk_window.titlebar().map(|t| t.type_().name().to_string())
    );
    let header = gtk::HeaderBar::new();
    header.set_show_close_button(true);
    header.set_title(Some("rFirma"));
    gtk_window.set_titlebar(Some(&header));
    gtk_window.insert_action_group("hdr", Some(&header_actions(window.app_handle())));

    let split = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    split.style_context().add_class("linked");
    let open = gtk::Button::with_label("Abrir PDF…");
    open.set_focus_on_click(false);
    open.set_action_name(Some("hdr.open"));
    let arrow = gtk::MenuButton::new();
    arrow.set_focus_on_click(false);
    MenuButtonExt::set_direction(&arrow, gtk::ArrowType::Down);
    arrow.set_menu_model(Some(&recents_model("Abiertos recientemente", &[])));
    split.add(&open);
    split.add(&arrow);

    let menu = gtk::MenuButton::new();
    menu.set_focus_on_click(false);
    menu.set_image(Some(&gtk::Image::from_icon_name(Some("open-menu-symbolic"), gtk::IconSize::Button)));
    menu.set_menu_model(Some(&app_menu_model("Preferencias…", "Acerca de rFirma")));

    header.pack_start(&split);
    header.pack_end(&menu);
    header.show_all();
    arrow.set_visible(false);

    HEADER.with(|h| *h.borrow_mut() = Some(HeaderWidgets { split, open, arrow, menu }));
}

fn main() {
    tauri::Builder::default()
        .on_window_event(|_, e| if let tauri::WindowEvent::Focused(f) = e { eprintln!("poc: Focused({f})") })
        .invoke_handler(tauri::generate_handler![desktop_info, set_header_state])
        .setup(|app| {
            let v = variant();
            let undecorate_at_build = v == "html" && is_gnome();
            let page = if v == "native" { "native.html" } else { "index.html" };
            let window = WebviewWindowBuilder::new(app, "main", WebviewUrl::App(page.into()))
                .title("rFirma")
                .inner_size(1100.0, 640.0)
                .decorations(!undecorate_at_build)
                .visible(v != "native")
                .build()?;
            match v.as_str() {
                "html" => attach_header_menu(&window),
                "html-runtime" => {
                    attach_header_menu(&window);
                    let w = window.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(1500));
                        eprintln!("poc: set_decorations(false) now");
                        let _ = w.set_decorations(false);
                    });
                }
                "native" => {
                    native_headerbar(&window);
                    window.show()?;
                }
                _ => {}
            }
            let _ = app.get_webview_window("main");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
