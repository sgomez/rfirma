//! La barra de título nativa de GTK de la ventana principal en Linux, y nada en el resto; capa fina, sin pruebas.

use super::views::TitlebarStateView;

/// Nombre del evento con el que cada control de la barra llega a la ventana.
pub const TITLEBAR_ACTION: &str = "titlebar-action";

/// Si la ventana principal nace oculta para montarle la barra antes de mostrarla.
pub const BUILT_HIDDEN: bool = cfg!(target_os = "linux");

/// Monta la barra en la ventana principal, aún oculta, y la muestra.
#[cfg(target_os = "linux")]
pub fn mount_and_show(window: &tauri::WebviewWindow) {
    if let Ok(gtk_window) = window.gtk_window() {
        gtk_titlebar::mount(window, &gtk_window);
    }
    let _ = window.show();
}

/// Monta la barra en la ventana principal, aún oculta, y la muestra.
#[cfg(not(target_os = "linux"))]
pub fn mount_and_show(_window: &tauri::WebviewWindow) {}

/// Aplica a la barra el estado que manda la ventana; solo en el hilo principal.
#[cfg(target_os = "linux")]
pub fn apply(state: &TitlebarStateView) {
    gtk_titlebar::apply(state);
}

/// Aplica a la barra el estado que manda la ventana; solo en el hilo principal.
#[cfg(not(target_os = "linux"))]
pub fn apply(_state: &TitlebarStateView) {}

#[cfg(target_os = "linux")]
mod gtk_titlebar {
    use std::cell::RefCell;

    use gtk::prelude::*;
    use tauri::Emitter;

    use super::super::views::{TitlebarActionView, TitlebarRecentView};
    use super::{TitlebarStateView, TITLEBAR_ACTION};

    const MISSING_RECENT: &str = "missing-recent";
    const MISSING_ACTION: &str = "hdr.missing-recent";

    const ACTIONS: [(&str, TitlebarActionView); 6] = [
        ("open", TitlebarActionView::Open),
        ("status", TitlebarActionView::Status),
        ("preferences", TitlebarActionView::Preferences),
        ("feedback", TitlebarActionView::Feedback),
        ("about", TitlebarActionView::About),
        ("clear-recents", TitlebarActionView::ClearRecents),
    ];

    struct Widgets {
        split: gtk::Box,
        open: gtk::Button,
        recents: gtk::MenuButton,
        warning: gtk::Button,
        menu: gtk::MenuButton,
    }

    thread_local! {
        static WIDGETS: RefCell<Option<Widgets>> = const { RefCell::new(None) };
    }

    pub(super) fn mount(window: &tauri::WebviewWindow, gtk_window: &gtk::ApplicationWindow) {
        let header = gtk::HeaderBar::new();
        header.set_show_close_button(true);
        header.set_title(Some("rFirma"));
        gtk_window.set_titlebar(Some(&header));
        gtk_window.insert_action_group("hdr", Some(&actions(window)));

        let open = gtk::Button::new();
        open.set_focus_on_click(false);
        open.set_action_name(Some("hdr.open"));

        let recents = gtk::MenuButton::new();
        recents.set_focus_on_click(false);

        let split = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        split.style_context().add_class("linked");
        split.pack_start(&open, false, false, 0);
        split.pack_start(&recents, false, false, 0);

        let warning =
            gtk::Button::from_icon_name(Some("dialog-warning-symbolic"), gtk::IconSize::Button);
        warning.set_focus_on_click(false);
        warning.set_action_name(Some("hdr.status"));

        let menu = gtk::MenuButton::new();
        menu.set_focus_on_click(false);
        menu.set_image(Some(&gtk::Image::from_icon_name(
            Some("open-menu-symbolic"),
            gtk::IconSize::Button,
        )));

        header.pack_start(&split);
        header.pack_end(&menu);
        header.pack_end(&warning);
        header.show_all();
        split.set_visible(false);
        recents.set_visible(false);
        warning.set_visible(false);
        menu.set_visible(false);

        open_the_menu_on_f10(gtk_window, &menu);
        WIDGETS.with(|widgets| {
            *widgets.borrow_mut() = Some(Widgets {
                split,
                open,
                recents,
                warning,
                menu,
            });
        });
    }

    pub(super) fn apply(state: &TitlebarStateView) {
        WIDGETS.with(|widgets| {
            let widgets = widgets.borrow();
            let Some(widgets) = widgets.as_ref() else {
                return;
            };
            let labels = &state.labels;
            widgets.open.set_label(&labels.open);
            widgets.open.set_tooltip_text(Some(&labels.open_tooltip));
            widgets.split.set_visible(state.open_visible);
            widgets.recents.set_tooltip_text(Some(&labels.recents));
            widgets.recents.set_menu_model(Some(&recents_model(state)));
            widgets.recents.set_visible(!state.recents.is_empty());
            name(&widgets.warning, &labels.warning);
            widgets.warning.set_visible(state.warning_visible);
            name(&widgets.menu, &labels.menu);
            widgets.menu.set_menu_model(Some(&menu_model(state)));
            widgets.menu.set_visible(true);
        });
    }

    fn actions(window: &tauri::WebviewWindow) -> gio::SimpleActionGroup {
        let group = gio::SimpleActionGroup::new();
        for (name, view) in ACTIONS {
            let action = gio::SimpleAction::new(name, None);
            let window = window.clone();
            action.connect_activate(move |_, _| {
                let _ = window.emit(TITLEBAR_ACTION, view.clone());
            });
            group.add_action(&action);
        }
        group.add_action(&recent_action(window));
        let missing = gio::SimpleAction::new(MISSING_RECENT, None);
        missing.set_enabled(false);
        group.add_action(&missing);
        group
    }

    fn recent_action(window: &tauri::WebviewWindow) -> gio::SimpleAction {
        let action = gio::SimpleAction::new("recent", Some(glib::VariantTy::STRING));
        let window = window.clone();
        action.connect_activate(move |_, target| {
            if let Some(path) = target.and_then(|target| target.get::<String>()) {
                let _ = window.emit(TITLEBAR_ACTION, TitlebarActionView::Recent { path });
            }
        });
        action
    }

    fn recents_model(state: &TitlebarStateView) -> gio::Menu {
        let entries = gio::Menu::new();
        for recent in &state.recents {
            let item =
                gio::MenuItem::new(Some(&recent_label(recent, &state.labels.not_found)), None);
            if recent.found {
                item.set_action_and_target_value(
                    Some("hdr.recent"),
                    Some(&recent.path.to_variant()),
                );
            } else {
                item.set_action_and_target_value(Some(MISSING_ACTION), None);
            }
            entries.append_item(&item);
        }
        let clear = gio::Menu::new();
        clear.append(Some(&state.labels.clear_recents), Some("hdr.clear-recents"));
        let model = gio::Menu::new();
        model.append_section(Some(&state.labels.recents), &entries);
        model.append_section(None, &clear);
        model
    }

    fn recent_label(recent: &TitlebarRecentView, not_found: &str) -> String {
        let text = if recent.found {
            let signed = if recent.signed { "\u{2713} " } else { "" };
            format!("{signed}{} \u{2014} {}", recent.name, recent.folder)
        } else {
            format!("{} \u{2014} {not_found}", recent.name)
        };
        text.replace('_', "__")
    }

    fn menu_model(state: &TitlebarStateView) -> gio::Menu {
        let labels = &state.labels;
        let installation = gio::Menu::new();
        installation.append(Some(&labels.status), Some("hdr.status"));
        let application = gio::Menu::new();
        application.append(Some(&labels.preferences), Some("hdr.preferences"));
        application.append(Some(&labels.feedback), Some("hdr.feedback"));
        application.append(Some(&labels.about), Some("hdr.about"));
        let model = gio::Menu::new();
        model.append_section(None, &installation);
        model.append_section(None, &application);
        model
    }

    fn name(widget: &impl IsA<gtk::Widget>, label: &str) {
        widget.set_tooltip_text(Some(label));
        if let Some(accessible) = widget.accessible() {
            gtk::atk::prelude::AtkObjectExt::set_name(&accessible, label);
        }
    }

    fn open_the_menu_on_f10(gtk_window: &gtk::ApplicationWindow, menu: &gtk::MenuButton) {
        let menu = menu.clone();
        gtk_window.connect_key_press_event(move |_, event| {
            let bare = !event
                .state()
                .intersects(gtk::accelerator_get_default_mod_mask());
            if event.keyval() == gtk::gdk::keys::constants::F10 && bare && menu.is_visible() {
                menu.set_active(true);
                return gtk::glib::Propagation::Stop;
            }
            gtk::glib::Propagation::Proceed
        });
    }
}
