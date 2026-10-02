//! El caso de uso de la línea de órdenes: atiende una orden y dice qué sale por cada flujo, sin escribir en ninguno.

use std::path::Path;

use crate::desktop::domain::command_line::{
    command_of, file_for_the_window, is_a_help_flag, parameter_left_out, Command, Refusal,
};
use crate::desktop::domain::sign_arguments::parse_sign_arguments;
use crate::desktop::ports::DesktopHandover;

/// El código de salida de una orden que termina bien.
pub const SUCCEEDED: i32 = 0;

/// El código de salida de una orden que falla al atenderse.
pub const FAILED: i32 = 1;

/// El código de salida de una línea de órdenes que no se atiende tal como llega.
pub const REFUSED: i32 = 2;

/// Lo que una orden deja al terminar: código de salida, bytes de stdout y líneas de stderr.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<String>,
}

impl Outcome {
    fn refused(refusal: &Refusal) -> Self {
        Self {
            exit_code: REFUSED,
            stdout: Vec::new(),
            stderr: vec![
                format!("rfirma: {refusal}"),
                "rfirma: «rfirma --help» describe las órdenes y sus parámetros".to_owned(),
            ],
        }
    }

    fn not_yet_available(command: Command) -> Self {
        Self {
            exit_code: FAILED,
            stdout: Vec::new(),
            stderr: vec![format!(
                "rfirma: la orden «{}» todavía no está disponible en esta versión",
                command.name()
            )],
        }
    }
}

/// Atiende los argumentos que siguen al ejecutable, empezando por la orden.
pub fn attend(arguments: &[String], desktop: &dyn DesktopHandover) -> Outcome {
    let command = match command_of(arguments) {
        Ok(command) => command,
        Err(refusal) => return Outcome::refused(&refusal),
    };
    if arguments
        .iter()
        .skip(1)
        .any(|argument| is_a_help_flag(argument))
    {
        return Outcome {
            exit_code: SUCCEEDED,
            stdout: command.syntax().unwrap_or_default().as_bytes().to_vec(),
            stderr: Vec::new(),
        };
    }
    if let Some(refusal) = parameter_left_out(arguments) {
        return Outcome::refused(&refusal);
    }
    if matches!(command, Command::Sign | Command::Cosign) {
        if let Err(refusal) = parse_sign_arguments(&arguments[1..]) {
            return Outcome::refused(&Refusal::InvalidArguments(refusal));
        }
    }
    match file_for_the_window(command, arguments) {
        Err(refusal) => Outcome::refused(&refusal),
        Ok(Some(file)) => hand_over_to_the_window(desktop, file),
        Ok(None) => Outcome::not_yet_available(command),
    }
}

fn hand_over_to_the_window(desktop: &dyn DesktopHandover, file: &str) -> Outcome {
    match desktop.hand_over(Path::new(file)) {
        Ok(()) => Outcome::default(),
        Err(reason) => Outcome {
            exit_code: FAILED,
            stdout: Vec::new(),
            stderr: vec![format!("rfirma: no se puede abrir la ventana ({reason})")],
        },
    }
}

#[cfg(test)]
mod tests;
