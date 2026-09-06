use super::{bmp_string, module_spec};
use std::path::Path;

#[test]
fn the_password_travels_as_a_big_endian_bmp_string_with_its_terminator() {
    assert_eq!(
        bmp_string("1234"),
        vec![0, b'1', 0, b'2', 0, b'3', 0, b'4', 0, 0]
    );
}

#[test]
fn a_password_outside_ascii_keeps_the_big_endian_order() {
    assert_eq!(bmp_string("ñ"), vec![0x00, 0xf1, 0, 0]);
}

#[test]
fn an_empty_password_is_just_the_terminator() {
    assert_eq!(bmp_string(""), vec![0, 0]);
}

#[test]
fn the_store_is_created_in_sql_format_and_writable() {
    let spec = module_spec(Path::new("/casa/datos/rfirma/certificates/abc"));

    assert!(spec.contains("configDir='sql:/casa/datos/rfirma/certificates/abc'"));
    assert!(spec.contains("flags=readWrite"));
}
