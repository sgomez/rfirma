//! Lo que este proceso sabe de sí mismo: su línea de órdenes, su carpeta y su relanzamiento.

use crate::desktop::application::invocation::{
    arguments_before_the_single_instance, delivered_urls, informative_text, Arguments, Invocation,
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

/// Lanza un proceso de sede por cada URL `afirma://` que traiga el evento, y dice si este sigue.
pub fn launch_the_delivered_urls(event: &tauri::RunEvent, already_serving: bool) -> bool {
    let delivered = delivered_urls(urls_of(event), already_serving);
    for url in &delivered.site_launches {
        launch_a_site_process(url);
    }
    delivered.this_process_goes_on
}

#[cfg(target_os = "macos")]
fn urls_of(event: &tauri::RunEvent) -> Vec<String> {
    match event {
        tauri::RunEvent::Opened { urls } => urls.iter().map(ToString::to_string).collect(),
        _ => Vec::new(),
    }
}

#[cfg(not(target_os = "macos"))]
fn urls_of(_: &tauri::RunEvent) -> Vec<String> {
    Vec::new()
}

fn launch_a_site_process(url: &str) {
    let launched = std::env::current_exe()
        .and_then(|executable| std::process::Command::new(executable).arg(url).spawn());
    match launched {
        Ok(mut child) => {
            std::thread::spawn(move || child.wait());
        }
        Err(error) => eprintln!("rfirma: no se puede abrir la llamada de sede ({error})"),
    }
}

/// Imprime la ayuda o la versión si la línea de órdenes las pide, y dice si lo hizo.
pub fn printed_the_informative_text(command_line: &[String]) -> bool {
    let text = informative_text(command_line, env!("CARGO_PKG_VERSION"));
    text.inspect(|text| println!("{text}")).is_some()
}
