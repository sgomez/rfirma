//! Lo que este proceso sabe de sí mismo: su línea de órdenes, su carpeta y su relanzamiento.

use crate::desktop::application::invocation::{
    arguments_before_the_single_instance, Arguments, Invocation,
};

/// La invocación con la que arrancó este proceso.
pub fn this_invocation() -> Invocation {
    Invocation {
        command_line: std::env::args_os()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect(),
        folder: std::env::current_dir().unwrap_or_default(),
    }
}

/// Los argumentos de este proceso tal como los dio el sistema.
pub fn these_arguments() -> Vec<String> {
    std::env::args_os()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect()
}

/// Asegura que los argumentos de la línea de órdenes tengan codificación UTF-8 válida.
pub fn make_the_command_line_readable() {
    let Arguments::RerunWith(arguments) = arguments_before_the_single_instance(std::env::args_os())
    else {
        return;
    };
    let executable = match std::env::current_exe() {
        Ok(executable) => executable,
        Err(error) => {
            eprintln!(
                "rfirma: no se puede releer la línea de órdenes ilegible \
                 ({error}); el arranque sigue con ella tal cual"
            );
            return;
        }
    };
    match std::process::Command::new(executable)
        .args(arguments.iter().skip(1))
        .spawn()
    {
        Ok(_) => std::process::exit(0),
        Err(error) => eprintln!(
            "rfirma: no se puede volver a arrancar con la línea de órdenes ya \
             legible ({error}); el arranque sigue con ella tal cual"
        ),
    }
}
