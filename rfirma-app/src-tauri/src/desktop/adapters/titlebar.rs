//! La barra de título nativa de GTK de la ventana principal en Linux, y nada en el resto; solo se prueba cuándo aplicar un estado y qué dice cada reciente.

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

/// Qué hacer con un estado que llega a la barra.
#[cfg(any(target_os = "linux", test))]
#[derive(Debug, PartialEq, Eq)]
pub enum Arrival {
    /// Es el último aplicado: no se toca nada.
    Ignore,
    /// Hay un menú abierto: espera a que se cierre.
    Defer,
    /// Se aplica ya.
    Apply,
}

/// Decide qué hacer con un estado nuevo según el último aplicado y si hay un menú abierto.
#[cfg(any(target_os = "linux", test))]
pub fn decide<T: PartialEq>(applied: Option<&T>, arrived: &T, menu_open: bool) -> Arrival {
    if applied == Some(arrived) {
        Arrival::Ignore
    } else if menu_open {
        Arrival::Defer
    } else {
        Arrival::Apply
    }
}

/// El último estado aplicado a la barra y el que espera a que se cierre un menú.
#[cfg(any(target_os = "linux", test))]
pub struct Pacing<T> {
    applied: Option<T>,
    pending: Option<T>,
}

#[cfg(any(target_os = "linux", test))]
impl<T: PartialEq + Clone> Default for Pacing<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(target_os = "linux", test))]
impl<T: PartialEq + Clone> Pacing<T> {
    /// Una barra a la que aún no se le ha aplicado nada.
    pub const fn new() -> Self {
        Self {
            applied: None,
            pending: None,
        }
    }

    /// Recibe un estado y devuelve el que hay que aplicar ya, si hay.
    pub fn arrive(&mut self, arrived: T, menu_open: bool) -> Option<T> {
        match decide(self.applied.as_ref(), &arrived, menu_open) {
            Arrival::Ignore => {
                self.pending = None;
                None
            }
            Arrival::Defer => {
                self.pending = Some(arrived);
                None
            }
            Arrival::Apply => {
                self.pending = None;
                self.applied = Some(arrived.clone());
                Some(arrived)
            }
        }
    }

    /// Al cerrarse un menú, devuelve el estado pendiente que hay que aplicar, si hay.
    pub fn menu_closed(&mut self) -> Option<T> {
        let pending = self.pending.take()?;
        self.arrive(pending, false)
    }
}

/// La segunda línea de la fila de un reciente: su ubicación, «No se encuentra» o nada.
#[cfg(any(target_os = "linux", test))]
pub fn second_line<'a>(
    recent: &'a super::views::TitlebarRecentView,
    not_found: &'a str,
) -> &'a str {
    if recent.found {
        recent.location.as_deref().unwrap_or_default()
    } else {
        not_found
    }
}

/// Lo que la fila de un reciente anuncia al lector de pantalla y enseña en su tooltip.
#[cfg(any(target_os = "linux", test))]
pub fn announcement(recent: &super::views::TitlebarRecentView, not_found: &str) -> String {
    let signed = if recent.signed { " \u{2713}" } else { "" };
    match second_line(recent, not_found) {
        "" => format!("{}{signed}", recent.name),
        line => format!("{}{signed}, {line}", recent.name),
    }
}

#[cfg(target_os = "linux")]
mod gtk_titlebar {
    use std::cell::RefCell;

    use gtk::prelude::*;
    use tauri::Emitter;

    use super::super::views::{TitlebarActionView, TitlebarRecentView};
    use super::{announcement, second_line, Pacing, TitlebarStateView, TITLEBAR_ACTION};
    use crate::documents::domain::recents::CAPACITY as RECENTS_CAPACITY;

    const ACTIONS: [(&str, TitlebarActionView); 6] = [
        ("open", TitlebarActionView::Open),
        ("status", TitlebarActionView::Status),
        ("preferences", TitlebarActionView::Preferences),
        ("feedback", TitlebarActionView::Feedback),
        ("about", TitlebarActionView::About),
        ("clear-recents", TitlebarActionView::ClearRecents),
    ];

    const RECENTS_CONTENT_WIDTH: i32 = 368;
    const RECENTS_CHROME_HEIGHT: i32 = 120;
    const RECENTS_MIN_LIST_HEIGHT: i32 = 52;

    const TITLEBAR_POPOVER_CLASS: &str = "rfirma-titlebar-popover";

    const TITLEBAR_POPOVER_CSS: &str = "
        popover.rfirma-titlebar-popover {
            padding: 6px;
            border-radius: 12px;
        }
        popover.rfirma-titlebar-popover modelbutton {
            min-height: 32px;
            padding: 0 12px;
            margin: 1px 0;
            border-radius: 6px;
        }
        popover.rfirma-titlebar-popover modelbutton:hover,
        popover.rfirma-titlebar-popover list row:hover {
            background-color: alpha(currentColor, 0.08);
        }
        popover.rfirma-titlebar-popover separator {
            margin: 4px 0;
        }
        popover.rfirma-titlebar-popover .rfirma-recents-heading {
            padding: 6px 12px 4px;
            font-size: 13px;
            font-weight: bold;
        }
        popover.rfirma-titlebar-popover scrolledwindow,
        popover.rfirma-titlebar-popover viewport,
        popover.rfirma-titlebar-popover list {
            background-color: transparent;
            border: none;
        }
        popover.rfirma-titlebar-popover list row {
            padding: 6px 12px;
            margin: 1px 0;
            border-radius: 6px;
        }
        popover.rfirma-titlebar-popover list row:disabled {
            opacity: 0.45;
        }
        popover.rfirma-titlebar-popover .rfirma-recent-name {
            font-size: 13px;
        }
        popover.rfirma-titlebar-popover .rfirma-recent-location {
            font-size: 12px;
        }
    ";

    struct RecentRow {
        row: gtk::ListBoxRow,
        name: gtk::Label,
        second: gtk::Label,
    }

    struct Recents {
        heading: gtk::Label,
        rows: Vec<RecentRow>,
        clear: gtk::ModelButton,
    }

    struct Widgets {
        split: gtk::Box,
        open: gtk::Button,
        recents_button: gtk::MenuButton,
        recents: Recents,
        warning: gtk::Button,
        menu: gtk::MenuButton,
    }

    thread_local! {
        static WIDGETS: RefCell<Option<Widgets>> = const { RefCell::new(None) };
        static PACING: RefCell<Pacing<TitlebarStateView>> = const { RefCell::new(Pacing::new()) };
        static RECENT_PATHS: RefCell<Vec<Option<String>>> = const { RefCell::new(Vec::new()) };
    }

    fn install_popover_style() {
        let Some(screen) = gtk::gdk::Screen::default() else {
            return;
        };
        let provider = gtk::CssProvider::new();
        if provider
            .load_from_data(TITLEBAR_POPOVER_CSS.as_bytes())
            .is_ok()
        {
            gtk::StyleContext::add_provider_for_screen(
                &screen,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }

    fn style_popover(button: &gtk::MenuButton) {
        if let Some(popover) = button.popover() {
            popover.style_context().add_class(TITLEBAR_POPOVER_CLASS);
        }
    }

    pub(super) fn mount(window: &tauri::WebviewWindow, gtk_window: &gtk::ApplicationWindow) {
        install_popover_style();
        let header = gtk::HeaderBar::new();
        header.set_show_close_button(true);
        header.set_title(Some("rFirma"));
        gtk_window.set_titlebar(Some(&header));
        gtk_window.insert_action_group("hdr", Some(&actions(window)));

        let open = gtk::Button::new();
        open.set_focus_on_click(false);
        open.set_action_name(Some("hdr.open"));

        let recents_button = gtk::MenuButton::new();
        recents_button.set_focus_on_click(false);
        let recents = recents_popover(window, &recents_button);

        let split = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        split.style_context().add_class("linked");
        split.pack_start(&open, false, false, 0);
        split.pack_start(&recents_button, false, false, 0);

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
        recents_button.set_visible(false);
        warning.set_visible(false);
        menu.set_visible(false);

        open_the_menu_on_f10(gtk_window, &menu);
        apply_the_pending_state_on_close(&recents_button);
        apply_the_pending_state_on_close(&menu);
        WIDGETS.with(|widgets| {
            *widgets.borrow_mut() = Some(Widgets {
                split,
                open,
                recents_button,
                recents,
                warning,
                menu,
            });
        });
    }

    fn recents_popover(window: &tauri::WebviewWindow, button: &gtk::MenuButton) -> Recents {
        let heading = gtk::Label::new(None);
        heading.set_xalign(0.0);
        heading.style_context().add_class("rfirma-recents-heading");
        heading.style_context().add_class("dim-label");

        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::None);
        let rows: Vec<RecentRow> = (0..RECENTS_CAPACITY).map(|_| recent_row(&list)).collect();

        let scroller = gtk::ScrolledWindow::new(gtk::Adjustment::NONE, gtk::Adjustment::NONE);
        scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroller.set_propagate_natural_height(true);
        scroller.add(&list);

        let clear = gtk::ModelButton::new();
        clear.set_action_name(Some("hdr.clear-recents"));

        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.set_size_request(RECENTS_CONTENT_WIDTH, -1);
        content.pack_start(&heading, false, false, 0);
        content.pack_start(&scroller, true, true, 0);
        content.pack_start(
            &gtk::Separator::new(gtk::Orientation::Horizontal),
            false,
            false,
            0,
        );
        content.pack_start(&clear, false, false, 0);
        content.show_all();

        let popover = gtk::Popover::new(Some(button));
        popover.style_context().add_class(TITLEBAR_POPOVER_CLASS);
        popover.add(&content);
        button.set_popover(Some(&popover));

        open_the_recent_on_activation(window, &list, &popover);
        move_down_from_the_last_row_to(&list, &clear);
        fit_the_list_and_focus_it_on_open(button, &popover, &scroller, &list);

        Recents {
            heading,
            rows,
            clear,
        }
    }

    fn recent_row(list: &gtk::ListBox) -> RecentRow {
        let name = trimmed_label(gtk::pango::EllipsizeMode::Middle, "rfirma-recent-name");
        let second = trimmed_label(gtk::pango::EllipsizeMode::End, "rfirma-recent-location");
        second.style_context().add_class("dim-label");
        let lines = gtk::Box::new(gtk::Orientation::Vertical, 2);
        lines.pack_start(&name, false, false, 0);
        lines.pack_start(&second, false, false, 0);
        let row = gtk::ListBoxRow::new();
        row.add(&lines);
        list.add(&row);
        RecentRow { row, name, second }
    }

    fn trimmed_label(ellipsis: gtk::pango::EllipsizeMode, class: &str) -> gtk::Label {
        let label = gtk::Label::new(None);
        label.set_xalign(0.0);
        label.set_hexpand(true);
        label.set_ellipsize(ellipsis);
        label.set_max_width_chars(1);
        label.style_context().add_class(class);
        label
    }

    fn open_the_recent_on_activation(
        window: &tauri::WebviewWindow,
        list: &gtk::ListBox,
        popover: &gtk::Popover,
    ) {
        let window = window.clone();
        let popover = popover.clone();
        list.connect_row_activated(move |_, row| {
            let path = usize::try_from(row.index()).ok().and_then(|index| {
                RECENT_PATHS.with(|paths| paths.borrow().get(index).cloned().flatten())
            });
            if let Some(path) = path {
                let _ = window.emit(TITLEBAR_ACTION, TitlebarActionView::Recent { path });
                popover.popdown();
            }
        });
    }

    fn move_down_from_the_last_row_to(list: &gtk::ListBox, clear: &gtk::ModelButton) {
        let clear = clear.clone();
        list.connect_keynav_failed(move |_, direction| {
            if direction == gtk::DirectionType::Down {
                clear.grab_focus();
                return gtk::glib::Propagation::Stop;
            }
            gtk::glib::Propagation::Proceed
        });
    }

    fn fit_the_list_and_focus_it_on_open(
        button: &gtk::MenuButton,
        popover: &gtk::Popover,
        scroller: &gtk::ScrolledWindow,
        list: &gtk::ListBox,
    ) {
        let button = button.clone();
        let scroller = scroller.clone();
        let list = list.clone();
        popover.connect_map(move |_| {
            scroller.set_max_content_height(room_below(&button));
            let first = list
                .children()
                .into_iter()
                .find(|row| row.is_visible() && row.is_sensitive());
            if let Some(row) = first {
                row.grab_focus();
            }
        });
    }

    fn room_below(button: &gtk::MenuButton) -> i32 {
        let Some(toplevel) = button.toplevel() else {
            return RECENTS_MIN_LIST_HEIGHT;
        };
        let bottom = button
            .translate_coordinates(&toplevel, 0, button.allocated_height())
            .map_or(0, |(_, y)| y);
        (toplevel.allocated_height() - bottom - RECENTS_CHROME_HEIGHT).max(RECENTS_MIN_LIST_HEIGHT)
    }

    pub(super) fn apply(state: &TitlebarStateView) {
        let Some(menu_open) = WIDGETS.with(|widgets| {
            widgets
                .borrow()
                .as_ref()
                .map(|widgets| widgets.recents_button.is_active() || widgets.menu.is_active())
        }) else {
            return;
        };
        let now = PACING.with(|pacing| pacing.borrow_mut().arrive(state.clone(), menu_open));
        if let Some(state) = now {
            render(&state);
        }
    }

    fn apply_the_pending_state_on_close(button: &gtk::MenuButton) {
        button.connect_toggled(|button| {
            if button.is_active() {
                return;
            }
            let pending = PACING.with(|pacing| pacing.borrow_mut().menu_closed());
            if let Some(state) = pending {
                render(&state);
            }
        });
    }

    fn render(state: &TitlebarStateView) {
        WIDGETS.with(|widgets| {
            let widgets = widgets.borrow();
            let Some(widgets) = widgets.as_ref() else {
                return;
            };
            let labels = &state.labels;
            widgets.open.set_label(&labels.open);
            widgets.open.set_tooltip_text(Some(&labels.open_tooltip));
            widgets.split.set_visible(state.open_visible);
            widgets
                .recents_button
                .set_tooltip_text(Some(&labels.recents));
            render_recents(&widgets.recents, state);
            widgets
                .recents_button
                .set_visible(!state.recents.is_empty());
            name(&widgets.warning, &labels.warning);
            widgets.warning.set_visible(state.warning_visible);
            name(&widgets.menu, &labels.menu);
            widgets.menu.set_menu_model(Some(&menu_model(state)));
            style_popover(&widgets.menu);
            widgets.menu.set_visible(true);
        });
    }

    fn render_recents(recents: &Recents, state: &TitlebarStateView) {
        let labels = &state.labels;
        recents.heading.set_text(&labels.recents);
        recents.clear.set_text(Some(&labels.clear_recents));
        for (index, row) in recents.rows.iter().enumerate() {
            match state.recents.get(index) {
                Some(recent) => fill_row(row, recent, &labels.not_found),
                None => row.row.set_visible(false),
            }
        }
        RECENT_PATHS.with(|paths| {
            *paths.borrow_mut() = state
                .recents
                .iter()
                .map(|recent| recent.found.then(|| recent.path.clone()))
                .collect();
        });
    }

    fn fill_row(row: &RecentRow, recent: &TitlebarRecentView, not_found: &str) {
        row.name.set_markup(&name_markup(recent));
        row.second.set_text(second_line(recent, not_found));
        name(&row.row, &announcement(recent, not_found));
        row.row.set_sensitive(recent.found);
        row.row.set_visible(true);
    }

    fn name_markup(recent: &TitlebarRecentView) -> String {
        let name = gtk::glib::markup_escape_text(&recent.name);
        if recent.signed {
            format!("{name} <span alpha=\"60%\">\u{2713}</span>")
        } else {
            name.to_string()
        }
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
        group
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

#[cfg(test)]
mod tests;
