//! Ningún módulo de producción de la interfaz importa de una carpeta `testing/`; las pruebas y las historias sí pueden.

#[allow(dead_code)]
#[path = "ts_imports/support.rs"]
mod support;

use std::fs;

use support::{repository_root, specifier_of, src_path_targeted, tracked_modules};

const SRC: &str = "rfirma-app/src";

fn is_test_side(module: &str) -> bool {
    let file = module.rsplit('/').next().unwrap_or(module);
    module.split('/').any(|segment| segment == "testing")
        || file.contains(".test.")
        || file.contains(".stories.")
        || file == "test-setup.ts"
}

fn offences_in(module: &str, source: &str) -> Vec<String> {
    if is_test_side(module) {
        return Vec::new();
    }
    source
        .lines()
        .filter_map(|line| {
            let specifier = specifier_of(line)?;
            let target = src_path_targeted(module, &specifier)?;
            if !target.iter().any(|segment| segment == "testing") {
                return None;
            }
            Some(format!(
                "`{module}` importa `{specifier}`, de una carpeta `testing/`: el soporte de \
                 prueba no entra en producción"
            ))
        })
        .collect()
}

#[test]
fn production_modules_import_nothing_from_a_testing_folder() {
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
        "{} import(s) de producción apuntan a `testing/`:\n{}",
        offences.len(),
        offences.join("\n")
    );
}

#[test]
fn a_production_module_importing_a_testing_folder_turns_red() {
    let module = "rfirma-app/src/signing/SigningPanel.tsx";
    for source in [
        "import { rubric } from \"./testing/fixtures\";",
        "import type { Thing } from '../testing/render';",
        "} from \"../viewer/testing/fixtures\";",
        "const harness = import(\"./testing/harness\");",
    ] {
        assert_eq!(offences_in(module, source).len(), 1, "{source}");
    }
}

#[test]
fn tests_stories_and_testing_folders_may_import_testing_folders() {
    let source = "import { rubric } from \"./testing/fixtures\";";
    for module in [
        "rfirma-app/src/signing/SigningPanel.test.tsx",
        "rfirma-app/src/signing/SigningPanel.stories.tsx",
        "rfirma-app/src/signing/testing/harness.tsx",
        "rfirma-app/src/testing/mainWindowDoubles.ts",
        "rfirma-app/src/test-setup.ts",
    ] {
        assert!(offences_in(module, source).is_empty(), "{module}");
    }
}

#[test]
fn libraries_and_comments_are_left_alone() {
    let module = "rfirma-app/src/signing/SigningPanel.tsx";
    for source in [
        "import { render } from \"@testing-library/react\";",
        "// import { x } from \"./testing/x\";",
        "const label = \"./testing/x\";",
    ] {
        assert!(offences_in(module, source).is_empty(), "{source}");
    }
}
