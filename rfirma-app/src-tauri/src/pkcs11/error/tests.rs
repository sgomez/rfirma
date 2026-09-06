use super::*;

fn from_rv(rv: RvError) -> TokenError {
    TokenError::from(Error::Pkcs11(rv, Function::Login))
}

#[test]
fn the_named_codes_map_to_distinct_situations() {
    let situations = [
        from_rv(RvError::PinIncorrect).situation(),
        from_rv(RvError::PinLocked).situation(),
        from_rv(RvError::TokenNotPresent).situation(),
        from_rv(RvError::SessionHandleInvalid).situation(),
    ];

    assert_eq!(
        situations,
        [
            Situation::IncorrectPin,
            Situation::PinLocked,
            Situation::TokenAbsent,
            Situation::ExpiredSession,
        ]
    );
}

#[test]
fn every_mapped_code_keeps_its_raw_ckr_apart_and_untranslated() {
    for (rv, expected) in [
        (RvError::PinIncorrect, "CKR_PIN_INCORRECT"),
        (RvError::PinLocked, "CKR_PIN_LOCKED"),
        (RvError::TokenNotPresent, "CKR_TOKEN_NOT_PRESENT"),
        (RvError::DeviceRemoved, "CKR_DEVICE_REMOVED"),
        (RvError::SessionHandleInvalid, "CKR_SESSION_HANDLE_INVALID"),
    ] {
        let error = from_rv(rv);
        assert_eq!(error.ckr(), Some(expected));
        assert!(
            error.detail().starts_with(expected),
            "el detalle deberia empezar por el codigo crudo: {}",
            error.detail()
        );
    }
}

#[test]
fn an_unknown_code_falls_back_to_the_generic_situation_and_still_shows_itself() {
    let error = from_rv(RvError::UnknownErrorCode(0x0ded));

    assert_eq!(error.situation(), Situation::Unknown);
    assert_eq!(error.ckr(), Some("CKR_UNKNOWN(0xded)"));
    assert!(error.detail().contains("0xded"));
}

#[test]
fn a_vendor_code_also_shows_itself_instead_of_disappearing() {
    let error = from_rv(RvError::VendorDefined(0x8000_0042));

    assert_eq!(error.situation(), Situation::Unknown);
    assert_eq!(error.ckr(), Some("CKR_VENDOR_DEFINED+0x80000042"));
}

#[test]
fn the_detail_names_the_pkcs11_function_that_failed() {
    assert_eq!(
        from_rv(RvError::PinIncorrect).detail(),
        "CKR_PIN_INCORRECT (C_Login)"
    );
}

#[test]
fn a_failure_of_ours_carries_no_ckr_but_still_carries_a_detail() {
    let error = TokenError::new(Situation::CertificateNotFound, "no hay ninguna etiqueta X");

    assert_eq!(error.ckr(), None);
    assert!(!error.detail().is_empty());
    assert!(error.to_string().contains("CertificateNotFound"));
}
