//! Los lectores de tarjetas en la aplicación: el vigilante de esta plataforma y el evento con el que cada ventana recibe sus noticias (ADR-0048).

use tauri::{Emitter as _, Manager as _};

use crate::identity::application::readers::follow_the_readers_apart;
use crate::identity::domain::readers::Reader;
use crate::identity::ports::{ReaderWatch, Relisting};
use crate::identity::IdentityRoot;

use super::views::ReaderNewsView;
use super::DesktopReaderWatch;

/// Nombre del evento con el que una ventana recibe el estado de los lectores y la lista en caliente.
pub const CARD_READERS: &str = "card-readers";

/// Un vigilante que no puede vigilar: no hay línea de lector, y la lista se busca como siempre.
#[derive(Default)]
pub struct UnavailableReaderWatch;

impl ReaderWatch for UnavailableReaderWatch {
    fn next_change(&mut self) -> Option<Vec<Reader>> {
        None
    }
}

/// Arranca en su propio hilo la lista en caliente de la ventana principal.
pub fn follow_the_readers_for_the_main_window(app: &tauri::AppHandle) {
    let lending = app.clone();
    follow_the_readers_for(app, "main", move |use_it| {
        use_it(&lending.state::<IdentityRoot>().card_listing());
    });
}

/// Arranca en su propio hilo la lista en caliente de una ventana, con quien vuelve a listar para ella.
pub fn follow_the_readers_for(
    app: &tauri::AppHandle,
    window: &'static str,
    lend: impl Fn(&mut dyn FnMut(&dyn Relisting)) + Send + 'static,
) {
    let announcing = app.clone();
    follow_the_readers_apart(Box::<DesktopReaderWatch>::default(), lend, move |news| {
        announcing
            .state::<IdentityRoot>()
            .reader_now
            .note(news.reader);
        if let Some(window) = announcing.get_webview_window(window) {
            let _ = window.emit(CARD_READERS, ReaderNewsView::from(news));
        }
    });
}

#[cfg(test)]
mod tests;
