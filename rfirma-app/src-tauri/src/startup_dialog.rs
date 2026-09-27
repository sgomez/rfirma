//! Diálogo nativo GTK que enseña un fallo de arranque; capa fina, sin pruebas.

use crate::startup_failure::StartupFailure;
#[cfg(target_os = "linux")]
use crate::startup_failure::REPOSITORY_ADDRESS;

#[cfg(target_os = "linux")]
fn detail_text(failure: &StartupFailure) -> String {
    format!("{}\n\n{}", failure.detail(), REPOSITORY_ADDRESS)
}

#[cfg(target_os = "linux")]
fn show_gtk_dialog(failure: &StartupFailure) {
    use gtk::prelude::*;

    if gtk::init().is_err() {
        return;
    }

    let dialog = gtk::MessageDialog::new(
        None::<&gtk::Window>,
        gtk::DialogFlags::MODAL,
        gtk::MessageType::Error,
        gtk::ButtonsType::Close,
        failure.phrase(),
    );
    dialog.set_secondary_text(Some(&detail_text(failure)));
    dialog.set_secondary_use_markup(false);

    if let Some(label) = dialog
        .message_area()
        .downcast::<gtk::Box>()
        .ok()
        .and_then(|area| area.children().into_iter().nth(1))
        .and_then(|widget| widget.downcast::<gtk::Label>().ok())
    {
        label.set_selectable(true);
    }

    dialog.run();
    unsafe {
        dialog.destroy();
    }
}

/// Enseña el fallo de arranque en un diálogo nativo, escribe su detalle en `stderr` y sale del
/// proceso con código distinto de cero.
pub fn report_and_exit(failure: &StartupFailure) -> ! {
    eprintln!("rfirma: {failure}");
    #[cfg(target_os = "linux")]
    show_gtk_dialog(failure);
    #[cfg(windows)]
    show_windows_dialog_pending(failure);
    std::process::exit(1);
}

/// El diálogo nativo de Windows, que aún no existe: el fallo solo llega a `stderr`.
#[cfg(windows)]
fn show_windows_dialog_pending(_failure: &StartupFailure) {}
