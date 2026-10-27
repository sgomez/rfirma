use super::der_encoded;

#[test]
fn concatenated_r_and_s_come_back_as_a_sequence_of_two_integers() {
    assert_eq!(
        der_encoded(&[0x01, 0x02, 0x03, 0x04]).expect("r||s de cuatro bytes se reempaqueta"),
        vec![0x30, 0x08, 0x02, 0x02, 0x01, 0x02, 0x02, 0x02, 0x03, 0x04]
    );
}

#[test]
fn an_r_with_the_high_bit_set_keeps_the_zero_that_makes_it_positive() {
    assert_eq!(
        der_encoded(&[0x80, 0x01, 0x00, 0x02]).expect("r||s con el bit alto puesto"),
        vec![0x30, 0x08, 0x02, 0x03, 0x00, 0x80, 0x01, 0x02, 0x01, 0x02]
    );
}

#[test]
fn the_sixty_four_bytes_of_a_p256_token_become_a_sequence_of_two_thirty_two_byte_integers() {
    let der = der_encoded(&[0x7f; 64]).expect("r||s de una curva P-256 se reempaqueta");

    assert_eq!(der[0], 0x30);
    assert_eq!(der.len(), 2 + 2 * (2 + 32));
}

#[test]
fn a_signature_with_an_odd_number_of_bytes_is_refused() {
    let refusal = der_encoded(&[0x01, 0x02, 0x03]).expect_err("r||s impar no es r||s");

    assert!(
        refusal.detail().contains("3 bytes"),
        "el detalle dice cuántos bytes ha devuelto el token: {refusal}"
    );
}

#[test]
fn an_empty_signature_is_refused() {
    assert!(der_encoded(&[]).is_err());
}
