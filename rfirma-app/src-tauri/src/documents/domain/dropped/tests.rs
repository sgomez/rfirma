use super::*;

fn a_pdf(name: &str) -> PathBuf {
    PathBuf::from(format!("/home/quien/Contratos/{name}"))
}

#[test]
fn a_single_pdf_is_the_one_that_opens() {
    let pdf = a_pdf("solo.pdf");

    assert_eq!(
        resolved(first_pdf(std::slice::from_ref(&pdf)), Ok(())),
        Dropped::Opened {
            path: pdf,
            also_entering: Vec::new(),
            discarded: 0,
        }
    );
}

#[test]
fn something_that_is_not_a_pdf_is_told_and_nothing_opens() {
    assert_eq!(
        resolved(first_pdf(&[a_pdf("hoja.ods")]), Ok(())),
        Dropped::NotAPdf { discarded: 1 }
    );
}

#[test]
fn the_first_pdf_of_several_files_opens_and_the_rest_are_counted() {
    let pdf = a_pdf("factura.pdf");
    let another = a_pdf("contrato.pdf");

    assert_eq!(
        resolved(
            first_pdf(&[a_pdf("hoja.ods"), pdf.clone(), another.clone()]),
            Ok(())
        ),
        Dropped::Opened {
            path: pdf,
            also_entering: vec![another],
            discarded: 1,
        }
    );
}

#[test]
fn every_pdf_dropped_together_also_enters_and_none_is_silenced() {
    let first = a_pdf("primero.pdf");
    let second = a_pdf("segundo.pdf");
    let third = a_pdf("tercero.pdf");

    let dropped = resolved(
        first_pdf(&[first.clone(), second.clone(), third.clone()]),
        Ok(()),
    );

    assert_eq!(
        dropped,
        Dropped::Opened {
            path: first,
            also_entering: vec![second, third],
            discarded: 0,
        }
    );
}

#[test]
fn a_pdf_the_sandbox_cannot_read_is_a_failure_with_its_raw_detail() {
    let choice = first_pdf(&[a_pdf("contrato.pdf")]);

    let Dropped::Unreadable { detail, discarded } =
        resolved(choice, Err("permiso denegado".to_owned()))
    else {
        panic!("un PDF que no se puede abrir tiene que contarse como tal");
    };

    assert_eq!(discarded, 0);
    assert_eq!(detail, "permiso denegado", "el detalle crudo no se pierde");
}

#[test]
fn an_unreadable_first_pdf_does_not_fall_through_to_the_next_one() {
    let choice = first_pdf(&[a_pdf("primero.pdf"), a_pdf("segundo.pdf")]);

    let dropped = resolved(choice, Err("permiso denegado".to_owned()));

    assert!(matches!(dropped, Dropped::Unreadable { discarded: 1, .. }));
}

#[test]
fn the_extension_is_read_without_minding_the_case() {
    let choice = first_pdf(&[a_pdf("CONTRATO.PDF")]);

    assert!(matches!(resolved(choice, Ok(())), Dropped::Opened { .. }));
}

#[test]
fn dropping_nothing_is_not_a_failure() {
    assert_eq!(resolved(first_pdf(&[]), Ok(())), Dropped::Nothing);
}

#[test]
fn a_bare_positional_argument_is_the_document_that_opens() {
    let pdf = a_pdf("invocado.pdf");

    let paths = invoked_paths(
        &["rfirma".to_owned(), pdf.display().to_string()],
        Path::new("/"),
    );

    assert_eq!(paths, vec![pdf]);
}

#[test]
fn a_relative_argument_is_resolved_against_the_folder_it_was_invoked_from() {
    let paths = invoked_paths(
        &["rfirma".to_owned(), "relativo.pdf".to_owned()],
        Path::new("/home/quien/Contratos"),
    );

    assert_eq!(paths, vec![a_pdf("relativo.pdf")]);
}

#[test]
fn invoking_with_no_arguments_brings_no_document() {
    assert!(invoked_paths(&["rfirma".to_owned()], Path::new("/")).is_empty());
}

#[test]
fn a_flag_is_not_a_path_and_does_not_count() {
    let pdf = a_pdf("con-bandera.pdf");

    let paths = invoked_paths(
        &[
            "rfirma".to_owned(),
            "--algo".to_owned(),
            pdf.display().to_string(),
        ],
        Path::new("/"),
    );

    assert_eq!(paths, vec![pdf]);
}

#[test]
fn a_path_exported_by_the_portal_is_a_path_like_any_other() {
    let exported = PathBuf::from("/run/user/1000/doc/1e20dd88/contrato.pdf");

    assert!(is_pdf(&exported));
    assert_eq!(
        exported.file_name().and_then(|name| name.to_str()),
        Some("contrato.pdf")
    );
}
