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
mod gtk_titlebar;

#[cfg(test)]
mod tests;
