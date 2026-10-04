//! Guarda de dirección de la interfaz: nada fuera de la raíz importa de los módulos de la ventana principal, y la colocación solo importa de sí misma, del sistema de diseño y del catálogo.

#[path = "ts_imports/support.rs"]
mod support;

use std::fs;

use support::{
    repository_root, specifier_of, src_folder_targeted, src_path_targeted, tracked_modules,
};

const SRC: &str = "rfirma-app/src";
const PLACEMENT_FOLDER: &str = "placement";
const PLACEMENT_ALLOWED_FOLDERS: [&str; 3] = ["placement", "design-system", "i18n"];

fn is_main_window_module(target: &[String]) -> bool {
    match target {
        [file] => file == "App" || file.starts_with("App."),
        _ => false,
    }
}

fn is_test_scaffolding(relative: &str) -> bool {
    let file = relative.rsplit('/').next().unwrap_or(relative);
    relative.split('/').any(|segment| segment == "testing")
        || file.contains(".test.")
        || file.contains(".testSupport.")
        || file.ends_with("Fixtures.ts")
        || file.ends_with("Fixtures.tsx")
}

fn relative_to_src(module: &str) -> &str {
    module
        .strip_prefix("rfirma-app/src/")
        .expect("el modulo deberia colgar de rfirma-app/src")
}

fn main_window_offences_in(module: &str, source: &str) -> Vec<String> {
    let relative = relative_to_src(module);
    if !relative.contains('/') || is_test_scaffolding(relative) {
        return Vec::new();
    }
    source
        .lines()
        .filter_map(|line| {
            let specifier = specifier_of(line)?;
            let target = src_path_targeted(module, &specifier)?;
            if !is_main_window_module(&target) {
                return None;
            }
            Some(format!(
                "`{module}` importa `{specifier}`, un modulo de la ventana principal: lo que \
                 necesitaba de ahi pertenece a su zona, y la raiz es la que cablea las zonas"
            ))
        })
        .collect()
}

fn placement_offences_in(module: &str, source: &str) -> Vec<String> {
    if relative_to_src(module).split('/').next() != Some(PLACEMENT_FOLDER) {
        return Vec::new();
    }
    source
        .lines()
        .filter_map(|line| {
            let specifier = specifier_of(line)?;
            let folder = src_folder_targeted(module, &specifier)?;
            if PLACEMENT_ALLOWED_FOLDERS.contains(&folder.as_str()) {
                return None;
            }
            Some(format!(
                "`{module}` importa `{specifier}`, de `src/{folder}`: la colocacion no conoce \
                 otras zonas, solo el sistema de diseño y el catalogo; lo que necesitaba entra \
                 como argumento"
            ))
        })
        .collect()
}

fn offences_in(module: &str, source: &str) -> Vec<String> {
    let mut found = main_window_offences_in(module, source);
    found.extend(placement_offences_in(module, source));
    found
}

#[test]
fn the_window_imports_follow_the_direction_of_the_zones() {
    let root = repository_root();
    let modules = tracked_modules(&root, SRC);
    assert!(
        modules.len() > 100,
        "el listado no ha encontrado la interfaz: {} ficheros",
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
        "{} import(s) contra la direccion de la interfaz:\n{}",
        offences.len(),
        offences.join("\n")
    );
}

#[test]
fn a_zone_importing_a_main_window_module_turns_red() {
    for (module, source) in [
        (
            "rfirma-app/src/signing/SignedPanel.tsx",
            "import { formatSignedTime } from \"../App.signingOrder\";",
        ),
        (
            "rfirma-app/src/viewer/Viewer.tsx",
            "import type { MainWindowPorts } from '../App.ports';",
        ),
        ("rfirma-app/src/sede/SedeWindow.tsx", "} from \"../App\";"),
        (
            "rfirma-app/src/documents/Tabs.tsx",
            "const view = import(\"../App.usePageGeometry\");",
        ),
    ] {
        assert_eq!(offences_in(module, source).len(), 1, "{module}: {source}");
    }
}

#[test]
fn the_root_the_tests_and_their_scaffolding_may_import_main_window_modules() {
    let source = "import type { MainWindowPorts } from \"../App.ports\";";
    for module in [
        "rfirma-app/src/testing/mainWindowDoubles.ts",
        "rfirma-app/src/viewer/testing/doubles.ts",
        "rfirma-app/src/signing/SignedPanel.test.tsx",
        "rfirma-app/src/signing/SigningPanel.testSupport.tsx",
        "rfirma-app/src/signing/panelFixtures.ts",
    ] {
        assert!(offences_in(module, source).is_empty(), "{module}");
    }
    let from_the_root = "import { chosenFrom } from \"./App.signingOrder\";";
    assert!(offences_in("rfirma-app/src/App.useSignFlow.ts", from_the_root).is_empty());
    assert!(offences_in("rfirma-app/src/main.tsx", "import App from \"./App\";").is_empty());
}

#[test]
fn the_placement_importing_another_zone_turns_red() {
    let module = "rfirma-app/src/placement/usePlacement.ts";
    for source in [
        "import { useSigning } from \"../signing/useSigning\";",
        "import type { Page } from '../viewer/page';",
        "} from \"../documents/open\";",
        "const view = import(\"../preferences/View\");",
        "import { tauri } from \"../tauri\";",
        "import type { Ports } from \"../App.ports\";",
    ] {
        assert_eq!(placement_offences_in(module, source).len(), 1, "{source}");
    }
}

#[test]
fn the_placement_may_import_itself_the_design_system_and_the_catalogue() {
    let module = "rfirma-app/src/placement/usePlacement.test.ts";
    for source in [
        "import { pageSets } from \"./pageSets\";",
        "import { Button } from \"../design-system/Button\";",
        "import i18n from \"../i18n/i18n\";",
        "import { renderHook } from \"@testing-library/react\";",
        "// import { x } from \"../signing/x\";",
        "const label = \"../signing/x\";",
    ] {
        assert!(placement_offences_in(module, source).is_empty(), "{source}");
    }
    let elsewhere = "import { x } from \"../signing/x\";";
    assert!(placement_offences_in("rfirma-app/src/viewer/Viewer.tsx", elsewhere).is_empty());
}
