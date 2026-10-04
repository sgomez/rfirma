//! El sistema de diseño de la ventana solo importa de sí mismo y del catálogo de cadenas, nunca de una carpeta de dominio.

#[path = "ts_imports/support.rs"]
mod support;

use std::fs;

use support::{repository_root, specifier_of, src_folder_targeted, tracked_modules};

const DESIGN_SYSTEM: &str = "rfirma-app/src/design-system";
const ALLOWED_FOLDERS: [&str; 2] = ["design-system", "i18n"];

fn offences_in(module: &str, source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| {
            let specifier = specifier_of(line)?;
            let folder = src_folder_targeted(module, &specifier)?;
            if ALLOWED_FOLDERS.contains(&folder.as_str()) {
                return None;
            }
            Some(format!(
                "`{module}` importa `{specifier}`, de `src/{folder}`: el sistema de diseño no \
                 conoce las carpetas de dominio; lo que necesitaba entra por props"
            ))
        })
        .collect()
}

#[test]
fn the_design_system_imports_only_from_itself_and_the_string_catalogue() {
    let root = repository_root();
    let modules = tracked_modules(&root, DESIGN_SYSTEM);
    assert!(
        modules.len() > 10,
        "el listado no ha encontrado el sistema de diseño: {} ficheros",
        modules.len()
    );

    let offences: Vec<String> = modules
        .iter()
        .flat_map(|module| {
            let source = fs::read_to_string(root.join(module))
                .unwrap_or_else(|error| panic!("deberia leerse {module}: {error}"));
            offences_in(module, &source)
        })
        .collect();

    assert!(
        offences.is_empty(),
        "{} import(s) del sistema de diseño apuntan a una carpeta de dominio:\n{}",
        offences.len(),
        offences.join("\n")
    );
}

#[test]
fn an_import_of_a_domain_folder_turns_red() {
    let module = "rfirma-app/src/design-system/Button.tsx";
    for source in [
        "import { useSigning } from \"../signing/useSigning\";",
        "import type { Thing } from '../documents/thing';",
        "} from \"../identity/certificate\";",
        "const view = import(\"../site/View\");",
        "import \"../preferences/side-effect\";",
        "import { x } from \"../main\";",
    ] {
        assert_eq!(offences_in(module, source).len(), 1, "{source}");
    }
}

#[test]
fn what_stays_inside_or_goes_to_the_catalogue_is_left_alone() {
    let module = "rfirma-app/src/design-system/Dialog.test.tsx";
    for source in [
        "import { Button } from \"./Button\";",
        "import i18n from \"../i18n/i18n\";",
        "import { decorator } from \"../../.storybook/decorators/dialogWindow\";",
        "import { render } from \"@testing-library/react\";",
        "// import { x } from \"../signing/x\";",
        "const label = \"../signing/x\";",
    ] {
        assert!(offences_in(module, source).is_empty(), "{source}");
    }
}
