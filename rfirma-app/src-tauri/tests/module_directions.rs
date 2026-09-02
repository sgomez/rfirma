//! **La dirección de las dependencias entre módulos, escrita como prueba**
//! (ID-81, ID-82, ID-83).
//!
//! **Grada A**: lee ficheros del repositorio y nada más. Sin token, sin
//! librería nativa y sin red.
//!
//! Rust no vigila los ciclos *dentro* de un crate: `signing` podía importar
//! `ffi` mientras `ffi` importaba `signing`, y `destination` importar `memory`
//! mientras `memory` importaba `destination`, y todo compilaba. Por eso los dos
//! ciclos vivieron ahí sin que nadie se enterara hasta el #136, y por eso hace
//! falta una guarda: sin ella la pregunta «¿dónde pongo este tipo?» vuelve a no
//! tener respuesta deducible en cuanto alguien añada el `use` cómodo.
//!
//! Lo que se comprueba son las **dos direcciones prohibidas**, no las
//! permitidas: `ffi` sigue importando `signing::session_seal` y `memory` sigue
//! importando `destination`, que son infraestructura mirando al dominio y son
//! las direcciones correctas.
//!
//! Se mira **la mitad de producción** de cada módulo, cortando por `mod tests`.
//! Un `use` dentro de `#[cfg(test)]` no participa en el grafo que se compila y
//! no cierra ningún ciclo: `destination/portal.rs` comprueba contra
//! `memory::RecentDocument` que el identificador del portal sigue a la ruta y no
//! al inodo, y esa comprobación es de las dos cosas a la vez.
//!
//! Si esta prueba te ha puesto el PR en rojo, el `use` que has añadido no es el
//! problema: es el síntoma. Lo que has escrito pertenece al otro lado de la
//! flecha —normalmente a `app/`, que es donde viven las decisiones—.

use std::fs;
use std::path::Path;

/// Un módulo que no puede nombrar a otro, y por qué.
struct Direction {
    /// La carpeta que no debe importar, relativa a `src/`.
    module: &'static str,
    /// El `crate::<nombre>` que no puede aparecer dentro.
    forbidden: &'static str,
    /// La decisión que lo dice, para que el fallo se lea sin abrir el issue.
    reason: &'static str,
}

const DIRECTIONS: [Direction; 2] = [
    Direction {
        module: "signing",
        forbidden: "ffi",
        reason: "el ciclo trifásico es un caso de uso y vive en `app/cycle.rs`; \
                 `signing/` son reglas puras y no cruza la frontera nativa (ID-82)",
    },
    Direction {
        module: "destination",
        forbidden: "memory",
        reason: "`DestinationFolder` es un concepto del destino y vive en \
                 `destination/`; desenvolver la configuración lo hace `app/` (ID-83)",
    },
];

/// Los ficheros `.rs` de una carpeta de `src/`, con su nombre para el mensaje.
fn sources_of(module: &str) -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(module);
    let entries = fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("deberia poder leerse {}: {error}", root.display()));

    let mut found = Vec::new();
    for entry in entries {
        let path = entry.expect("deberia poder leerse la entrada").path();
        if path.extension().is_some_and(|extension| extension == "rs") {
            let source = fs::read_to_string(&path).expect("deberia poder leerse el modulo");
            found.push((
                format!("{module}/{}", file_name(&path)),
                production_half(&source),
            ));
        }
    }

    assert!(!found.is_empty(), "`{module}/` no tiene ningun fichero .rs");
    found
}

/// La mitad de producción de un módulo: lo que hay antes de `mod tests`.
fn production_half(source: &str) -> String {
    source
        .split_once("\nmod tests {")
        .map(|(before, _)| before)
        .unwrap_or(source)
        .to_owned()
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn no_domain_module_imports_the_one_it_must_not() {
    for Direction {
        module,
        forbidden,
        reason,
    } in DIRECTIONS
    {
        // Solo se miran las líneas `use`: el nombre prohibido aparece también en
        // los comentarios de contrato, y ahí decir «no importa `ffi`» es
        // justamente lo contrario de importarlo.
        let needle = format!("crate::{forbidden}");
        for (file, source) in sources_of(module) {
            let offending: Vec<&str> = source
                .lines()
                .filter(|line| line.trim_start().starts_with("use ") && line.contains(&needle))
                .collect();

            assert!(
                offending.is_empty(),
                "`{file}` importa `crate::{forbidden}`: {reason}\n{}",
                offending.join("\n")
            );
        }
    }
}
