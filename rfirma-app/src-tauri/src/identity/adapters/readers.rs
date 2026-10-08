//! Los lectores de tarjetas en la aplicación: el vigilante de esta plataforma y el evento con el que la ventana principal recibe sus noticias (ADR-0048).

use tauri::{Emitter as _, Manager as _};

use crate::identity::application::readers::follow_the_readers_apart;
use crate::identity::domain::readers::Reader;
use crate::identity::ports::ReaderWatch;
use crate::identity::IdentityRoot;

use super::views::ReaderNewsView;
use super::DesktopReaderWatch;

/// Nombre del evento con el que la ventana principal recibe el estado de los lectores y la lista en caliente.
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
    let announcing = app.clone();
    follow_the_readers_apart(
        Box::new(DesktopReaderWatch::default()),
        move |use_it| use_it(&lending.state::<IdentityRoot>().card_listing()),
        move |news| {
            if let Some(window) = announcing.get_webview_window("main") {
                let _ = window.emit(CARD_READERS, ReaderNewsView::from(news));
            }
        },
    );
}

#[cfg(test)]
mod tests;
