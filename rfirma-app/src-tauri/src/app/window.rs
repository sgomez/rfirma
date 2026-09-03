//! El tamaño de la ventana entre sesiones, y si estaba maximizada (ID-72,
//! ID-73).
//!
//! **No mira ningún interruptor**: [`crate::memory::Memory::remember_window`]
//! escribe siempre, y por eso este módulo tampoco recibe la
//! [`Configuration`](crate::memory::Configuration) que los demás casos de uso
//! del estado sí piden. Es la única memoria exenta de «Recordar mi
//! actividad», y una firma que la pidiera sugeriría que hay algo que mirar.
//!
//! **La posición no se guarda en ningún campo**: en Wayland el cliente no
//! puede pedirla al compositor, así que unas coordenadas guardadas serían una
//! promesa incumplida (ADR-0010, enmienda).

use crate::memory::{Memory, WindowMemory};

/// El ancho con el que abre la ventana cuando no hay nada guardado, o cuando
/// lo guardado no se pudo leer. El mismo valor que `tauri.conf.json`.
pub const DEFAULT_WIDTH: f64 = 1280.0;
/// El alto con el que abre la ventana cuando no hay nada guardado. El mismo
/// valor que `tauri.conf.json`.
pub const DEFAULT_HEIGHT: f64 = 720.0;
/// El ancho mínimo al que se puede encoger la ventana. El mismo valor que
/// `tauri.conf.json`. Por debajo de esta cifra el visor deja de ser la
/// región principal, con 660 px fijos entre bandeja y panel.
pub const MIN_WIDTH: f64 = 1100.0;
/// El alto mínimo al que se puede encoger la ventana. El mismo valor que
/// `tauri.conf.json`.
pub const MIN_HEIGHT: f64 = 560.0;

/// Con qué tamaño abrir la ventana: el recordado, o el de por omisión.
///
/// Un primer arranque sin `state.json` —o uno con un `state.json` que no se
/// pudo leer— vuelve al tamaño de por omisión sin ruido: no es un fallo, es
/// que no había nada que recordar todavía.
pub fn initial_window(memory: &Memory) -> WindowMemory {
    memory
        .state()
        .ok()
        .and_then(|loaded| loaded.into_value().window)
        .unwrap_or(default_window())
}

/// El tamaño de por omisión.
pub fn default_window() -> WindowMemory {
    WindowMemory {
        width: DEFAULT_WIDTH,
        height: DEFAULT_HEIGHT,
        maximized: false,
    }
}

/// Lo que recordar cuando la ventana cambia de tamaño o se maximiza.
///
/// `logical_size` es el tamaño **restaurado** —el que tendría la ventana si
/// no estuviera maximizada—, en píxeles lógicos. Mientras la ventana está
/// maximizada esa medida no la sabe quien llama, porque lo único que reporta
/// el sistema de ventanas es el tamaño de la pantalla; por eso, maximizada,
/// se conserva el ancho y el alto que ya hubiera guardados y solo cambia el
/// interruptor de `maximized`.
pub fn resized(memory: &Memory, maximized: bool, logical_size: Option<(f64, f64)>) {
    let remembered = default_window();
    let previous = memory
        .state()
        .ok()
        .and_then(|loaded| loaded.into_value().window)
        .unwrap_or(remembered);

    let (width, height) = if maximized {
        (previous.width, previous.height)
    } else {
        logical_size.unwrap_or((previous.width, previous.height))
    };

    let _ = memory.remember_window(WindowMemory {
        width,
        height,
        maximized,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::fixtures::a_memory;

    #[test]
    fn a_first_run_opens_at_the_default_size() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());

        assert_eq!(initial_window(&memory), default_window());
    }

    #[test]
    fn a_remembered_size_comes_back() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());
        let window = WindowMemory {
            width: 1024.0,
            height: 768.0,
            maximized: false,
        };
        memory.remember_window(window).expect("deberia guardarse");

        assert_eq!(initial_window(&memory), window);
    }

    #[test]
    fn a_remembered_maximized_window_comes_back_maximized() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());
        let window = WindowMemory {
            width: 1024.0,
            height: 768.0,
            maximized: true,
        };
        memory.remember_window(window).expect("deberia guardarse");

        assert_eq!(initial_window(&memory), window);
    }

    #[test]
    fn resizing_while_not_maximized_records_the_new_size() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());

        resized(&memory, false, Some((1200.0, 650.0)));

        assert_eq!(
            initial_window(&memory),
            WindowMemory {
                width: 1200.0,
                height: 650.0,
                maximized: false,
            }
        );
    }

    /// Maximizar no pisa el tamaño restaurado: si se reabriera sin maximizar
    /// habría que volver a ese tamaño, no al de la pantalla entera.
    #[test]
    fn maximizing_keeps_the_previously_remembered_size() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());
        resized(&memory, false, Some((1200.0, 650.0)));

        resized(&memory, true, None);

        assert_eq!(
            initial_window(&memory),
            WindowMemory {
                width: 1200.0,
                height: 650.0,
                maximized: true,
            }
        );
    }
}
