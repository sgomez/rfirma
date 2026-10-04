//! La entrada del binario de consola de Windows: atiende la orden de terminal con su propio contexto de Tauri o pasa la invocación al `rfirma.exe` de su carpeta, sin esperarlo (ADR-0041).

use std::process::Command;

use crate::desktop::adapters::process::{printed_the_informative_text, this_invocation};
use crate::desktop::adapters::terminal::run_the_command_line;
use crate::desktop::application::console_entry::{console_entry_of, ConsoleEntry};

/// Atiende la invocación de este proceso de consola y devuelve su código de salida.
pub fn run_in_the_console() -> i32 {
    let invocation = this_invocation();
    match console_entry_of(invocation.clone()) {
        ConsoleEntry::AttendsTheCommand => {
            run_the_command_line(&invocation.command_line, crate::context())
        }
        ConsoleEntry::PrintsTheInformativeText => {
            printed_the_informative_text(&invocation.command_line);
            0
        }
        ConsoleEntry::HandsOverToTheWindow => hand_over_to_the_window(),
    }
}

fn hand_over_to_the_window() -> i32 {
    let window = format!("rfirma{}", std::env::consts::EXE_SUFFIX);
    let launched = std::env::current_exe().and_then(|executable| {
        Command::new(executable.with_file_name(window))
            .args(std::env::args_os().skip(1))
            .spawn()
    });
    match launched {
        Ok(_) => 0,
        Err(error) => {
            eprintln!("rfirma: no se puede abrir la ventana de rFirma ({error})");
            1
        }
    }
}
