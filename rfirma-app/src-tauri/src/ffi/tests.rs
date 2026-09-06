use super::*;
use std::alloc::{alloc, dealloc, Layout};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};

/// **Grada A**: ni librería nativa, ni token, ni entorno del proceso.
fn environment(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
    let map: HashMap<String, OsString> = pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), OsString::from(*value)))
        .collect();
    move |name: &str| map.get(name).cloned()
}

#[test]
fn the_library_is_looked_for_next_to_the_executable() {
    let looked_at = candidates(&environment(&[]), Path::new("/app/bin"));

    assert_eq!(looked_at.len(), 1);
    assert_eq!(looked_at[0].origin(), Origin::RelativeToExecutable);
    assert!(looked_at[0]
        .library_path()
        .ends_with("lib/rfirma/librfirma_crypto.so"));
}

#[test]
fn the_environment_variable_is_looked_at_first() {
    let looked_at = candidates(
        &environment(&[(LIBRARY_DIRECTORY_VARIABLE, "/otro/sitio")]),
        Path::new("/app/bin"),
    );

    assert_eq!(looked_at.len(), 2);
    assert_eq!(looked_at[0].origin(), Origin::Override);
    assert_eq!(
        looked_at[0].library_path(),
        PathBuf::from("/otro/sitio/librfirma_crypto.so")
    );
    assert_eq!(looked_at[1].origin(), Origin::RelativeToExecutable);
}

#[test]
fn an_empty_variable_is_ignored_instead_of_pointing_at_the_working_directory() {
    let looked_at = candidates(
        &environment(&[(LIBRARY_DIRECTORY_VARIABLE, "")]),
        Path::new("/app/bin"),
    );

    assert_eq!(looked_at.len(), 1);
    assert_eq!(looked_at[0].origin(), Origin::RelativeToExecutable);
}

#[test]
fn the_override_wins_when_both_directories_have_the_library() {
    let directory = tempfile::tempdir().expect("debería haber directorio temporal");
    let overridden = directory.path().join("override");
    let next_to_executable = directory.path().join("app/lib/rfirma");
    for place in [&overridden, &next_to_executable] {
        std::fs::create_dir_all(place).expect("debería crearse");
        std::fs::write(place.join(LIBRARY_FILE), b"no es una libreria de verdad")
            .expect("debería escribirse");
    }

    let found = locate(
        &environment(&[(LIBRARY_DIRECTORY_VARIABLE, &overridden.to_string_lossy())]),
        &directory.path().join("app/bin"),
    )
    .expect("debería encontrarla");

    assert_eq!(found, overridden.join(LIBRARY_FILE));
}

#[test]
fn starting_without_the_library_names_the_two_paths_it_looked_at() {
    let directory = tempfile::tempdir().expect("debería haber directorio temporal");
    let overridden = directory.path().join("vacio");

    let error = locate(
        &environment(&[(LIBRARY_DIRECTORY_VARIABLE, &overridden.to_string_lossy())]),
        &directory.path().join("app/bin"),
    )
    .expect_err("sin librería no debería resolverse");

    let message = error.to_string();
    assert_eq!(error.looked_at().len(), 2, "{message}");
    assert!(
        message.contains(&overridden.display().to_string()),
        "{message}"
    );
    assert!(message.contains("lib/rfirma"), "{message}");
    assert!(message.contains(LIBRARY_DIRECTORY_VARIABLE), "{message}");
    assert!(message.contains("relativa al ejecutable"), "{message}");
}

#[test]
fn a_directory_without_the_file_is_not_the_library() {
    let directory = tempfile::tempdir().expect("debería haber directorio temporal");
    std::fs::create_dir_all(directory.path().join("lib/rfirma")).expect("debería crearse");

    let error = locate(&environment(&[]), &directory.path().join("bin"))
        .expect_err("un directorio no es la librería");

    assert_eq!(error.looked_at().len(), 1);
}

#[derive(Default)]
struct Counter {
    live: RefCell<HashSet<usize>>,
    freed: Cell<usize>,
}

impl Counter {
    fn allocate(&self, contents: &str) -> *mut c_char {
        let bytes = contents.as_bytes();
        let layout = Layout::array::<u8>(bytes.len() + 1).expect("cabe");
        let pointer = unsafe { alloc(layout) };
        assert!(!pointer.is_null(), "sin memoria");
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer, bytes.len());
            pointer.add(bytes.len()).write(0);
        }
        assert!(
            self.live.borrow_mut().insert(pointer as usize),
            "el asignador ha devuelto una dirección que ya estaba viva"
        );
        pointer.cast()
    }

    fn freed(&self) -> usize {
        self.freed.get()
    }

    fn live(&self) -> usize {
        self.live.borrow().len()
    }
}

impl FreeBridgeString for &Counter {
    unsafe fn free(&self, pointer: *mut c_char) {
        assert!(!pointer.is_null(), "nunca se libera un nulo");
        assert!(
            self.live.borrow_mut().remove(&(pointer as usize)),
            "doble free: {pointer:?} ya se había liberado"
        );
        self.freed.set(self.freed.get() + 1);
        unsafe {
            let length = CStr::from_ptr(pointer).to_bytes().len();
            let layout = Layout::array::<u8>(length + 1).expect("cabe");
            dealloc(pointer.cast(), layout);
        }
    }
}

#[test]
fn every_pointer_the_bridge_returns_is_freed_exactly_once() {
    let counter = Counter::default();

    for _ in 0..1_000 {
        let pointer = counter.allocate(r#"{"ok":true,"pdf":"AAAA"}"#);
        let owned = unsafe { BridgeString::adopt(pointer, &counter) }.expect("no es nulo");
        assert_eq!(owned.to_utf8_lossy(), r#"{"ok":true,"pdf":"AAAA"}"#);
    }

    assert_eq!(counter.freed(), 1_000, "cada cadena se libera");
    assert_eq!(counter.live(), 0, "y no queda ninguna sin liberar");
}

#[test]
fn the_pointer_is_freed_even_when_the_response_is_unusable() {
    let counter = Counter::default();

    let pointer = counter.allocate("esto no es JSON");
    let owned = unsafe { BridgeString::adopt(pointer, &counter) }.expect("no es nulo");
    let error = parse_presign(&owned.to_utf8_lossy()).expect_err("no es el JSON del contrato");
    drop(owned);

    assert!(matches!(error, BridgeError::MalformedResponse(_)));
    assert_eq!(counter.freed(), 1, "el camino de error también libera");
}

#[test]
fn a_null_answer_is_an_error_and_frees_nothing() {
    let counter = Counter::default();

    let adopted = unsafe { BridgeString::adopt(std::ptr::null_mut(), &counter) };

    assert!(matches!(adopted, Err(BridgeError::NullResponse)));
    assert_eq!(counter.freed(), 0);
}

#[test]
fn a_presign_answer_comes_back_split_into_its_three_pieces() {
    let signature =
        parse_presign(r#"{"ok":true,"session":"<xml/>","pre":"MTIz","stamp":"c2VsbG8="}"#)
            .expect("es el JSON del contrato");

    assert_eq!(signature.session(), "<xml/>");
    assert_eq!(signature.pre_sign(), b"123");
    assert_eq!(
        signature.stamp(),
        &SessionSeal::from_bridge("c2VsbG8="),
        "el sello viaja opaco, tal y como vino"
    );
}

#[test]
fn a_postsign_answer_comes_back_as_the_bytes_of_the_pdf() {
    let pdf = parse_postsign(r#"{"ok":true,"pdf":"JVBERi0="}"#).expect("es el JSON del contrato");

    assert_eq!(pdf, b"%PDF-");
}

#[test]
fn a_filter_answer_comes_back_as_the_rows_that_survived() {
    let selected = parse_filter_selection(r#"{"ok":true,"selected":[0,2]}"#).expect("es valida");

    assert_eq!(selected, vec![0, 2]);
}

#[test]
fn an_empty_selection_is_an_answer_and_not_a_failure() {
    assert_eq!(
        parse_filter_selection(r#"{"ok":true,"selected":[]}"#).expect("es valida"),
        Vec::<usize>::new()
    );
}

#[test]
fn a_selection_that_is_not_a_list_of_rows_is_a_malformed_answer() {
    assert!(parse_filter_selection(r#"{"ok":true}"#).is_err());
    assert!(parse_filter_selection(r#"{"ok":true,"selected":"0,2"}"#).is_err());
    assert!(parse_filter_selection(r#"{"ok":true,"selected":[-1]}"#).is_err());
}

#[test]
fn a_failure_of_the_filter_engine_travels_like_any_other() {
    let error =
        parse_filter_selection(r#"{"ok":false,"error":"java.lang.IllegalArgumentException: mal"}"#)
            .expect_err("el motor ha fallado");

    assert!(error.to_string().contains("IllegalArgumentException"));
}

#[test]
fn a_failure_from_the_bridge_keeps_the_java_message_untranslated() {
    let error = parse_presign(r#"{"ok":false,"error":"java.io.IOException: no es un PDF"}"#)
        .expect_err("ok:false es un fallo");

    let message = error.to_string();
    assert!(matches!(error, BridgeError::Failed(_)), "{message}");
    assert!(message.contains("java.io.IOException"), "{message}");
}

#[test]
fn a_pdf_with_unregistered_signatures_is_not_just_a_failure() {
    let error = parse_presign(
        r#"{"ok":false,"kind":"pdfHasUnregisteredSignatures","error":"PdfHasUnregisteredSignaturesException"}"#,
    )
    .expect_err("ok:false es un fallo");

    assert!(
        matches!(error, BridgeError::PdfHasUnregisteredSignatures(_)),
        "{error}"
    );
}

#[test]
fn a_failure_kind_this_binary_does_not_know_is_still_a_failure() {
    let error = parse_presign(r#"{"ok":false,"kind":"loQueSea","error":"algo"}"#)
        .expect_err("ok:false es un fallo");

    assert!(matches!(error, BridgeError::Failed(_)), "{error}");
}

#[test]
fn an_answer_without_the_ok_field_is_not_a_signature() {
    let error = parse_postsign(r#"{"pdf":"JVBERi0="}"#).expect_err("falta \"ok\"");

    assert!(matches!(error, BridgeError::MalformedResponse(_)));
}

#[test]
fn an_answer_missing_a_field_is_not_a_signature_either() {
    let error =
        parse_presign(r#"{"ok":true,"session":"<xml/>","pre":"MTIz"}"#).expect_err("falta stamp");

    assert!(error.to_string().contains("stamp"), "{error}");
}

#[test]
fn a_field_that_is_not_base64_is_a_malformed_answer_and_not_a_panic() {
    let error =
        parse_postsign(r#"{"ok":true,"pdf":"esto no es base64 %%%"}"#).expect_err("no es Base64");

    assert!(matches!(error, BridgeError::MalformedResponse(_)));
}

#[test]
fn every_failure_of_the_border_says_what_actually_went_wrong() {
    let directory = tempfile::tempdir().expect("debería haber directorio temporal");
    let not_found = locate(&environment(&[]), &directory.path().join("bin"))
        .expect_err("sin librería no debería resolverse");

    let messages = [
        (
            BridgeError::ExecutablePathUnknown("no such file".to_owned()),
            "ejecutable",
        ),
        (BridgeError::from(not_found), "lib/rfirma"),
        (
            BridgeError::Load {
                path: PathBuf::from("/app/lib/rfirma/librfirma_crypto.so"),
                detail: "no es un ELF".to_owned(),
            },
            "librfirma_crypto.so",
        ),
        (
            BridgeError::MissingSymbol {
                symbol: "autofirma_free_string".to_owned(),
                detail: "undefined symbol".to_owned(),
            },
            "autofirma_free_string",
        ),
        (BridgeError::IsolateFailed(7), "graal_create_isolate"),
        (BridgeError::InvalidArgument("el PDF"), "el PDF"),
        (BridgeError::NullResponse, "NULL"),
        (
            BridgeError::MalformedResponse("no trae \"ok\"".to_owned()),
            "respuesta ilegible",
        ),
        (
            BridgeError::Failed("java.io.IOException: no es un PDF".to_owned()),
            "java.io.IOException",
        ),
    ];

    for (error, expected) in messages {
        let message = error.to_string();
        assert!(
            message.contains(expected),
            "{error:?} debería nombrar «{expected}»: {message}"
        );
    }
}

#[test]
fn not_knowing_where_the_executable_is_does_not_blame_the_bridge() {
    let error = BridgeError::ExecutablePathUnknown("no such file".to_owned()).to_string();

    assert!(!error.contains("puente"), "{error}");
}

#[test]
fn an_argument_with_a_nul_inside_is_rejected_before_crossing() {
    let error = c_string("con\0nulo", "el PDF").expect_err("no puede ser una cadena C");

    assert!(matches!(error, BridgeError::InvalidArgument("el PDF")));
}
