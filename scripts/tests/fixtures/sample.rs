//! Fixture de prueba para el esqueleto de `outline.sh`.

/// Une dos partes en una ruta.
pub fn join_paths(a: &str, b: &str) -> String {
    format!("{a}/{b}")
}

/// Una pareja de rutas.
pub struct PathPair {
    pub left: String,
    pub right: String,
}

#[test]
fn joins_two_paths() {
    assert_eq!(join_paths("a", "b"), "a/b");
}
