//! El veredicto de un caso del sondeo, escrito junto a la ficha del anexo A1 que resuelve.

use std::fs;
use std::path::Path;

const VERDICT_MARKER: &str = "* **Veredicto del sondeo";

/// Inserta `line` justo bajo `heading` en `path`, o la sustituye si ya había una del sondeo:
/// no altera nada más de la ficha.
pub fn record_verdict(path: &Path, heading: &str, line: &str) -> Result<(), String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("{} no se pudo leer: {error}", path.display()))?;
    let mut lines: Vec<&str> = contents.lines().collect();
    let heading_index = lines
        .iter()
        .position(|candidate| *candidate == heading)
        .ok_or_else(|| format!("{} no tiene la ficha «{heading}»", path.display()))?;

    let mut insertion = heading_index + 1;
    while lines
        .get(insertion)
        .is_some_and(|candidate| candidate.trim().is_empty())
    {
        insertion += 1;
    }
    if lines
        .get(insertion)
        .is_some_and(|candidate| candidate.starts_with(VERDICT_MARKER))
    {
        lines[insertion] = line;
    } else {
        lines.insert(insertion, line);
    }

    let mut updated = lines.join("\n");
    updated.push('\n');
    fs::write(path, updated)
        .map_err(|error| format!("{} no se pudo escribir: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADING: &str = "### BUG-25: Colapso de la distinción";

    fn a_ficha() -> String {
        format!(
            "{HEADING}\n\n* **Estado en `master`:** Sigue presente.\n* **Código fuente:** foo\n"
        )
    }

    #[test]
    fn inserts_the_verdict_right_under_the_heading() {
        let file = tempfile::NamedTempFile::new().unwrap();
        fs::write(file.path(), a_ficha()).unwrap();

        record_verdict(
            file.path(),
            HEADING,
            "* **Veredicto del sondeo (2026-09-17):** confirmado.",
        )
        .unwrap();

        let updated = fs::read_to_string(file.path()).unwrap();
        assert_eq!(
            updated,
            format!(
                "{HEADING}\n\n* **Veredicto del sondeo (2026-09-17):** confirmado.\n\
                 * **Estado en `master`:** Sigue presente.\n* **Código fuente:** foo\n"
            )
        );
    }

    #[test]
    fn a_second_run_replaces_the_verdict_instead_of_stacking_it() {
        let file = tempfile::NamedTempFile::new().unwrap();
        fs::write(file.path(), a_ficha()).unwrap();

        record_verdict(
            file.path(),
            HEADING,
            "* **Veredicto del sondeo (2026-09-17):** confirmado.",
        )
        .unwrap();
        record_verdict(
            file.path(),
            HEADING,
            "* **Veredicto del sondeo (2026-09-18):** refutado.",
        )
        .unwrap();

        let updated = fs::read_to_string(file.path()).unwrap();
        assert_eq!(updated.matches("Veredicto del sondeo").count(), 1);
        assert!(updated.contains("(2026-09-18):** refutado."));
        assert!(updated.contains("Estado en `master`"));
    }

    #[test]
    fn a_missing_heading_is_reported_instead_of_writing_blind() {
        let file = tempfile::NamedTempFile::new().unwrap();
        fs::write(file.path(), "### BUG-01: otra cosa\n").unwrap();

        let error =
            record_verdict(file.path(), HEADING, "* **Veredicto del sondeo:** x").unwrap_err();

        assert!(error.contains(HEADING));
    }
}
