use super::*;

struct CardAndWindows {
    opened: Cell<bool>,
}

impl CardAndWindows {
    fn new() -> Self {
        Self {
            opened: Cell::new(false),
        }
    }
}

impl CertificateStores for CardAndWindows {
    fn certificates(&self) -> Result<Vec<TokenCertificate>, TokenError> {
        self.opened.set(true);
        Ok([("cng:CurrentUser", "WIN"), ("/modulo.so", "TARJETA")]
            .into_iter()
            .map(|(module, label)| {
                TokenCertificate::new(
                    CertificateRef::new(Store::module(module), "token", label, None),
                    Vec::new(),
                )
            })
            .collect())
    }

    fn discovered_module(&self, library: &str) -> Option<PathBuf> {
        (library == "/modulo.so").then(|| PathBuf::from(library))
    }
}

fn listed_on(platform: Platform, words: &[&str], stores: &dyn CertificateStores) -> Outcome {
    attended_on(
        platform,
        words,
        stores,
        &RecordingDesktop::default(),
        &FilesInMemory::default(),
        &RecordingSigner::default(),
    )
}

#[test]
fn on_windows_store_windows_and_auto_narrow_to_the_windows_store() {
    for store in ["windows", "auto", "Windows"] {
        let outcome = listed_on(
            Platform::Windows,
            &["listaliases", "-store", store],
            &CardAndWindows::new(),
        );

        assert_eq!(outcome.exit_code, SUCCEEDED, "{store}");
        assert_eq!(outcome.stdout, b"WIN\n", "{store}");
    }
}

#[test]
fn on_windows_the_stores_it_does_not_open_are_refused_without_opening_any_store() {
    for store in [
        "mozilla",
        "pkcs12:/a.p12",
        "dni",
        "dnie",
        "mac",
        "inventado",
    ] {
        let stores = CardAndWindows::new();

        let outcome = listed_on(
            Platform::Windows,
            &["listaliases", "-store", store],
            &stores,
        );

        assert_eq!(outcome.exit_code, REFUSED, "{store}");
        assert!(outcome.stdout.is_empty());
        assert!(said(&outcome).contains("almacén"), "{}", said(&outcome));
        assert!(!stores.opened.get(), "{store}");
    }
}

#[test]
fn on_windows_pkcs11_with_a_discovered_module_behaves_as_on_linux() {
    let outcome = listed_on(
        Platform::Windows,
        &["listaliases", "-store", "pkcs11:/modulo.so"],
        &CardAndWindows::new(),
    );

    assert_eq!(outcome.exit_code, SUCCEEDED);
    assert_eq!(outcome.stdout, b"TARJETA\n");
}

#[test]
fn on_linux_store_windows_is_still_refused_without_opening_any_store() {
    let stores = CardAndWindows::new();

    let outcome = listed_on(
        Platform::Linux,
        &["listaliases", "-store", "windows"],
        &stores,
    );

    assert_eq!(outcome.exit_code, REFUSED);
    assert!(!stores.opened.get());
}

#[test]
fn on_windows_the_command_help_offers_neither_password_fd_nor_certtui() {
    for command in ["sign", "cosign"] {
        let outcome = listed_on(
            Platform::Windows,
            &[command, "--help"],
            &CardAndWindows::new(),
        );

        let help = String::from_utf8(outcome.stdout).expect("utf-8");
        assert_eq!(outcome.exit_code, SUCCEEDED);
        assert!(
            !help.contains("password-fd") && !help.contains("certtui"),
            "{help}"
        );
    }
}

#[test]
fn on_windows_the_password_refusal_explains_the_windows_pin_and_names_no_linux_tool() {
    let outcome = listed_on(
        Platform::Windows,
        &["sign", "-password", "1234", "-i", "a.pdf"],
        &CardAndWindows::new(),
    );

    assert_eq!(outcome.exit_code, REFUSED);
    let said = said(&outcome);
    assert!(said.contains("Windows"), "{said}");
    assert!(said.contains("--certgui"), "{said}");
    assert!(
        !said.contains("password-fd") && !said.contains("secret-tool"),
        "{said}"
    );
    assert!(!said.contains("1234"));
}

#[test]
fn on_linux_the_password_refusal_still_names_password_fd() {
    let outcome = listed_on(
        Platform::Linux,
        &["sign", "-password", "1234", "-i", "a.pdf"],
        &CardAndWindows::new(),
    );

    assert!(
        said(&outcome).contains("-password-fd"),
        "{}",
        said(&outcome)
    );
}

#[test]
fn the_password_is_refused_in_any_position_with_a_message_naming_password_fd() {
    for words in [
        &["sign", "-password", "1234", "-i", "a.pdf"][..],
        &["listaliases", "-store", "pkcs11", "-password", "1234"][..],
        &["sign", "-help", "-password", "1234"][..],
    ] {
        let outcome = attended(words);

        assert_eq!(outcome.exit_code, REFUSED);
        assert!(outcome.stdout.is_empty());
        assert!(
            said(&outcome).contains("-password-fd"),
            "{}",
            said(&outcome)
        );
        assert!(!said(&outcome).contains("1234"), "nunca repite el secreto");
    }
}
