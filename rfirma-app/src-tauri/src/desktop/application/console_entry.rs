//! Lo que hace el binario de consola de Windows con su invocación: atiende la orden de terminal, imprime la ayuda, la versión o el diagnóstico, o se la pasa a la ventana (ADR-0041).

use super::invocation::{
    debug_info_was_asked_for, help_was_asked_for, role_of, version_was_asked_for, Invocation, Role,
};

/// Destino de una invocación que llega al binario de consola.
#[derive(Debug, PartialEq, Eq)]
pub enum ConsoleEntry {
    /// La atiende como orden de terminal y termina con su código de salida.
    AttendsTheCommand,
    /// Imprime la ayuda o la versión y termina.
    PrintsTheInformativeText,
    /// Lanza la ventana de su misma carpeta con los mismos argumentos, sin esperarla.
    HandsOverToTheWindow,
}

/// Decide el destino con la misma decisión de rol que la ventana; nunca se queda con la sede.
pub fn console_entry_of(invocation: Invocation) -> ConsoleEntry {
    let informative = help_was_asked_for(&invocation.command_line)
        || version_was_asked_for(&invocation.command_line)
        || debug_info_was_asked_for(&invocation.command_line);
    match role_of(invocation) {
        Role::Terminal(_) => ConsoleEntry::AttendsTheCommand,
        _ if informative => ConsoleEntry::PrintsTheInformativeText,
        _ => ConsoleEntry::HandsOverToTheWindow,
    }
}

#[cfg(test)]
mod tests;
