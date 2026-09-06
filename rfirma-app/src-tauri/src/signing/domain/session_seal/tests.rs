use super::{SealMismatch, SessionSeal};

/// La forma que compone el puente. Vive **solo en las pruebas**: el código
/// de producción no sabe qué hay aquí dentro, y ese es justo el punto.
fn seal_of(extra_params: &str, instant: &str, time_zone: &str) -> SessionSeal {
    SessionSeal::from_bridge(format!(
        r#"{{"extraParams":{extra_params},"time":"{instant}","timeZone":"{time_zone}"}}"#
    ))
}

fn a_seal() -> SessionSeal {
    seal_of(
        r#"{"signatureSubFilter":"ETSI.CAdES.detached"}"#,
        "2026-08-31T12:00:00",
        "Europe/Madrid",
    )
}

#[test]
fn accepts_the_very_seal_the_presign_returned() {
    assert_eq!(a_seal().verify_unchanged(&a_seal()), Ok(()));
}

#[test]
fn rejects_a_seal_whose_effective_extra_params_changed() {
    let changed = seal_of(
        r#"{"signatureSubFilter":"adbe.pkcs7.detached"}"#,
        "2026-08-31T12:00:00",
        "Europe/Madrid",
    );
    assert_eq!(a_seal().verify_unchanged(&changed), Err(SealMismatch));
}

#[test]
fn rejects_a_seal_whose_instant_changed() {
    let changed = seal_of(
        r#"{"signatureSubFilter":"ETSI.CAdES.detached"}"#,
        "2026-08-31T12:00:01",
        "Europe/Madrid",
    );
    assert_eq!(a_seal().verify_unchanged(&changed), Err(SealMismatch));
}

#[test]
fn rejects_a_seal_whose_time_zone_changed() {
    let changed = seal_of(
        r#"{"signatureSubFilter":"ETSI.CAdES.detached"}"#,
        "2026-08-31T12:00:00",
        "Atlantic/Canary",
    );
    assert_eq!(a_seal().verify_unchanged(&changed), Err(SealMismatch));
}

#[test]
fn carries_an_opaque_seal_through_untouched() {
    let payload = "\u{1}\u{0}no es JSON, ni falta que hace\u{7f}ñ€\n";
    let seal = SessionSeal::from_bridge(payload);
    assert_eq!(seal.as_bridge_payload(), payload);
    assert_eq!(seal.verify_unchanged(&seal.clone()), Ok(()));
}

#[test]
fn does_not_show_the_inside_of_the_seal() {
    let debug = format!("{:?}", a_seal());
    assert!(!debug.contains("Europe/Madrid"), "{debug}");
    assert!(!debug.contains("signatureSubFilter"), "{debug}");
}

#[test]
fn explains_the_mismatch() {
    assert_eq!(
        SealMismatch.to_string(),
        "el sello de sesión de la postfirma no es el de la prefirma"
    );
}
