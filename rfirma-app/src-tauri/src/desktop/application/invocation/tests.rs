use std::path::Path;

use super::*;
use crate::crossing::Failure;
use crate::desktop::domain::platform::Platform;
use crate::documents::adapters::files::RealFiles;
use crate::documents::application::documents::OpenedDocuments;
use crate::documents::application::documents::{
    dropped_document, handed_over_to_see_its_signatures,
};

fn told(
    invocation: &Invocation,
    opened: &OpenedDocuments,
) -> Option<crate::documents::domain::told::DroppedDocument> {
    dropped_document(&RealFiles, &invoked_documents(invocation)?, opened)
}

/// **Grada A**: una línea de órdenes y un fichero temporal. Ni token, ni
/// puente, ni ventana.
fn a_temporary_pdf(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("rfirma-invocation-{name}"));
    std::fs::write(&path, b"%PDF-1.4\n").expect("se puede escribir en el temporal");
    path
}

fn invoked_with(path: &Path) -> Invocation {
    Invocation {
        command_line: vec!["rfirma".to_owned(), path.display().to_string()],
        folder: PathBuf::from("/"),
    }
}

#[test]
fn a_pdf_named_in_the_command_line_opens_like_a_dropped_one() {
    let pdf = a_temporary_pdf("contrato.pdf");
    let opened = OpenedDocuments::new();

    let view = told(&invoked_with(&pdf), &opened).expect("algo trae");

    assert!(view.refused.is_none(), "un PDF legible se abre y no avisa");
    assert_eq!(view.discarded, 0);
    let document = view.document.expect("y el documento cruza ya apuntado");
    assert_eq!(document.name, "rfirma-invocation-contrato.pdf");
}

#[test]
fn an_argument_that_is_not_a_pdf_opens_the_normal_window_and_says_so() {
    let other = a_temporary_pdf("hoja.ods");

    let view = told(&invoked_with(&other), &OpenedDocuments::new()).expect("algo trae");

    assert!(view.document.is_none(), "no se abre ningun documento");
    assert_eq!(
        Failure::from(view.refused.expect("y se dice por que")).situation,
        "notAPdf"
    );
}

#[test]
fn invoking_without_a_document_is_just_opening_the_application() {
    let invocation = Invocation {
        command_line: vec!["rfirma".to_owned()],
        folder: PathBuf::from("/"),
    };

    assert_eq!(told(&invocation, &OpenedDocuments::new()), None);
}

#[test]
fn a_second_invocation_with_a_document_replaces_the_one_that_was_there() {
    let pdf = a_temporary_pdf("segundo.pdf");
    let opened = OpenedDocuments::new();

    let second = second_invocation(&invoked_with(&pdf), false);

    let SecondInvocation::ReplacesWhatWasThere(paths, intent) = second else {
        panic!("sustituye: {second:?}");
    };
    let view = dropped_document(&RealFiles, &paths, &opened).expect("algo trae");
    assert!(
        view.document.is_some(),
        "el documento nuevo es el que queda"
    );
    assert_eq!(intent, WindowIntent::OpenTheDocument);
}

fn handed_over_by_the_terminal(file: &Path, intent: WindowIntent) -> Invocation {
    Invocation {
        command_line: std::iter::once(OsString::from("rfirma"))
            .chain(arguments_for_the_desktop(file, intent))
            .map(|argument| argument.into_string().expect("UTF-8"))
            .collect(),
        folder: PathBuf::from("/"),
    }
}

#[test]
fn a_file_handed_over_to_see_its_signatures_reaches_the_desktop_with_that_intent() {
    let pdf = a_temporary_pdf("ver-firmas.pdf");

    let invocation = handed_over_by_the_terminal(&pdf, WindowIntent::SeeItsSignatures);

    assert_eq!(
        role_of(invocation.clone()),
        Role::Desktop(invocation.clone())
    );
    assert_eq!(invoked_documents(&invocation), Some(vec![pdf]));
    assert_eq!(invoked_intent(&invocation), WindowIntent::SeeItsSignatures);
}

#[test]
fn a_file_handed_over_just_to_open_it_reaches_the_desktop_as_today() {
    let pdf = a_temporary_pdf("solo-abrir.pdf");

    let invocation = handed_over_by_the_terminal(&pdf, WindowIntent::OpenTheDocument);

    assert_eq!(invocation, invoked_with(&pdf));
    assert_eq!(invoked_intent(&invocation), WindowIntent::OpenTheDocument);
}

#[test]
fn a_file_that_is_not_a_pdf_handed_over_to_see_its_signatures_opens_in_the_window() {
    let signature = a_temporary_pdf("ver-firmas.csig");
    let invocation = handed_over_by_the_terminal(&signature, WindowIntent::SeeItsSignatures);

    let view = handed_over_to_see_its_signatures(
        &RealFiles,
        &invoked_documents(&invocation).expect("trae el fichero"),
        &OpenedDocuments::new(),
    )
    .expect("algo trae");

    assert!(view.refused.is_none());
    assert_eq!(
        view.document.expect("se abre").name,
        "rfirma-invocation-ver-firmas.csig"
    );
}

#[test]
fn a_second_invocation_to_see_the_signatures_replaces_the_document_with_that_intent() {
    let pdf = a_temporary_pdf("segundo-ver-firmas.pdf");

    let second = second_invocation(
        &handed_over_by_the_terminal(&pdf, WindowIntent::SeeItsSignatures),
        false,
    );

    assert_eq!(
        second,
        SecondInvocation::ReplacesWhatWasThere(vec![pdf], WindowIntent::SeeItsSignatures)
    );
}

#[test]
fn a_second_invocation_replaces_nothing_while_a_signing_session_is_live() {
    let pdf = a_temporary_pdf("mientras-firmo.pdf");

    assert_eq!(
        second_invocation(&invoked_with(&pdf), true),
        SecondInvocation::NothingHappens
    );
}

#[test]
fn not_even_a_notice_reaches_the_window_while_a_signing_session_is_live() {
    let other = a_temporary_pdf("hoja-mientras-firmo.ods");

    assert_eq!(
        second_invocation(&invoked_with(&other), true),
        SecondInvocation::NothingHappens
    );
}

#[test]
fn no_arguments_give_the_desktop_role() {
    let invocation = Invocation {
        command_line: vec!["rfirma".to_owned()],
        folder: PathBuf::from("/"),
    };

    assert_eq!(role_of(invocation.clone()), Role::Desktop(invocation));
}

#[test]
fn a_pdf_gives_the_desktop_role_with_that_document() {
    let pdf = a_temporary_pdf("rol-escritorio.pdf");
    let invocation = invoked_with(&pdf);

    assert_eq!(
        role_of(invocation.clone()),
        Role::Desktop(invocation.clone())
    );
    let opened = OpenedDocuments::new();
    assert!(
        told(&invocation, &opened)
            .expect("algo trae")
            .document
            .is_some(),
        "el documento sigue llegando a la ventana del escritorio"
    );
}

#[test]
fn a_url_gives_the_site_role_with_the_whole_url() {
    let invocation = invoked_with_the_url(A_LAUNCH);

    assert_eq!(role_of(invocation), Role::Site(A_LAUNCH.to_owned()));
}

#[test]
fn a_pdf_and_a_url_give_the_site_role_and_narrate_the_discarded_document() {
    let pdf = a_temporary_pdf("descartado.pdf");
    let invocation = Invocation {
        command_line: vec![
            "rfirma".to_owned(),
            pdf.display().to_string(),
            A_LAUNCH.to_owned(),
        ],
        folder: PathBuf::from("/"),
    };

    assert_eq!(role_of(invocation.clone()), Role::Site(A_LAUNCH.to_owned()));
    assert!(
        !Role::said(&invocation).is_empty(),
        "el documento descartado se narra"
    );
}

#[test]
fn a_url_alone_says_nothing() {
    assert_eq!(
        Role::said(&invoked_with_the_url(A_LAUNCH)),
        Vec::<String>::new()
    );
}

#[test]
fn a_desktop_invocation_says_nothing() {
    let pdf = a_temporary_pdf("sin-narrar.pdf");
    assert_eq!(Role::said(&invoked_with(&pdf)), Vec::<String>::new());
}

#[test]
fn the_help_is_asked_for_with_any_of_its_three_flags_and_from_any_position() {
    for flag in ["--help", "-help", "-h"] {
        assert!(help_was_asked_for(["rfirma", flag]), "con {flag} sola");
        assert!(
            help_was_asked_for(["rfirma", "documento.pdf", flag]),
            "con {flag} detrás de un documento"
        );
    }
}

#[test]
fn nothing_else_asks_for_the_help() {
    assert!(!help_was_asked_for(["rfirma"]));
    assert!(!help_was_asked_for(["rfirma", "documento.pdf"]));
    assert!(!help_was_asked_for([
        "rfirma",
        "afirma://websocket?ports=51000"
    ]));
    assert!(!help_was_asked_for(["rfirma", "--helpful"]));
    assert!(!help_was_asked_for(["-h"]), "el ejecutable no cuenta");
}

#[test]
fn the_help_names_every_autofirma_command_and_parameter() {
    for command in [
        "sign",
        "cosign",
        "countersign",
        "listaliases",
        "verify",
        "batchsign",
    ] {
        assert!(
            help(Platform::Linux).contains(command),
            "falta la orden {command}"
        );
    }
    for parameter in [
        "-i",
        "-o",
        "-alias",
        "-filter",
        "-store",
        "-format",
        "-password",
        "-algorithm",
        "-config",
        "-operation",
        "-gui",
        "-certgui",
        "-preurl",
        "-posturl",
        "-hformat",
        "-halgorithm",
        "-r",
        "-xml",
    ] {
        assert!(
            help(Platform::Linux).contains(parameter),
            "falta el parámetro {parameter}"
        );
    }
}

#[test]
fn the_help_names_the_three_ways_of_asking_for_it() {
    for flag in HELP_FLAGS {
        assert!(help(Platform::Linux).contains(flag), "falta {flag}");
    }
}

#[test]
fn the_pending_invocation_is_handed_over_once_and_only_once() {
    let pdf = a_temporary_pdf("pendiente.pdf");
    let pending = PendingInvocation::of(invoked_with(&pdf));

    assert_eq!(pending.take(), Some(invoked_with(&pdf)));
    assert_eq!(pending.take(), None);
}

#[test]
fn a_window_opened_with_nothing_pending_has_nothing_to_pick_up() {
    assert_eq!(PendingInvocation::default().take(), None);
}

const A_LAUNCH: &str = "afirma://websocket?ports=51000,51001&v=4&idsession=8jAkPZfRw2mQxN4TbYuL";

fn invoked_with_the_url(url: &str) -> Invocation {
    Invocation {
        command_line: vec!["rfirma".to_owned(), url.to_owned()],
        folder: PathBuf::from("/"),
    }
}

#[test]
fn a_site_url_is_not_treated_as_a_file_path() {
    let invocation = invoked_with_the_url(A_LAUNCH);

    assert_eq!(invocation.site_launch(), Some(A_LAUNCH));
    assert_eq!(told(&invocation, &OpenedDocuments::new()), None);
}

#[test]
fn an_afirma_url_by_the_bus_never_replaces_anything() {
    let invocation = Invocation {
        command_line: vec!["rfirma".to_owned(), A_LAUNCH.to_owned()],
        folder: PathBuf::from("/otra/carpeta"),
    };

    assert_eq!(invocation.site_launch(), Some(A_LAUNCH));
    assert_eq!(
        second_invocation(&invocation, false),
        SecondInvocation::NothingHappens,
        "un afirma:// por el bus lo manda un binario viejo; no sustituye ningún documento"
    );
}

#[test]
fn the_scheme_is_recognised_whatever_the_case_and_never_in_the_executable() {
    assert_eq!(
        invoked_with_the_url("AFIRMA://selectcert?ports=51000").site_launch(),
        Some("AFIRMA://selectcert?ports=51000")
    );
    assert_eq!(
        Invocation {
            command_line: vec!["afirma://rfirma".to_owned()],
            folder: PathBuf::from("/"),
        }
        .site_launch(),
        None
    );
}

#[cfg(unix)]
#[test]
fn an_argument_that_is_not_utf8_is_made_readable_before_the_plugin_reads_it() {
    use std::os::unix::ffi::OsStringExt as _;

    let unreadable = OsString::from_vec(vec![b'/', 0xff, b'.', b'p', b'd', b'f']);

    assert_eq!(
        arguments_before_the_single_instance(vec![OsString::from("rfirma"), unreadable]),
        Arguments::RerunWith(vec!["rfirma".to_owned(), "/\u{fffd}.pdf".to_owned()])
    );
}

#[test]
fn a_readable_command_line_reruns_nothing() {
    assert_eq!(
        arguments_before_the_single_instance(vec![
            OsString::from("rfirma"),
            OsString::from(A_LAUNCH),
        ]),
        Arguments::Readable
    );
}

#[test]
fn a_url_of_a_foreign_scheme_gives_no_role_and_opens_nothing() {
    let url = "other://websocket?ports=54391,54392,54393&v=4";

    assert_eq!(
        role_of(invoked_with_the_url(url)),
        Role::Foreign(url.to_owned())
    );
}

#[test]
fn a_path_without_a_scheme_still_gives_the_desktop_role() {
    let invocation = invoked_with_the_url("carpeta/documento.pdf");

    assert_eq!(role_of(invocation.clone()), Role::Desktop(invocation));
}

#[test]
fn a_file_url_gives_the_desktop_role_with_that_document() {
    let pdf = a_temporary_pdf("por-url.pdf");
    let invocation = invoked_with_the_url(&format!("file://{}", pdf.display()));

    assert_eq!(
        role_of(invocation.clone()),
        Role::Desktop(invocation.clone())
    );
    assert_eq!(invoked_documents(&invocation), Some(vec![pdf]));
}

#[test]
fn a_url_delivered_at_launch_becomes_a_site_process_and_ends_the_one_that_received_it() {
    assert_eq!(
        delivered_urls([A_LAUNCH.to_owned()], false),
        DeliveredUrls {
            site_launches: vec![A_LAUNCH.to_owned()],
            this_process_goes_on: false,
        }
    );
}

#[test]
fn a_url_delivered_to_a_process_already_serving_becomes_a_site_process_and_it_goes_on() {
    assert_eq!(
        delivered_urls([A_LAUNCH.to_owned()], true),
        DeliveredUrls {
            site_launches: vec![A_LAUNCH.to_owned()],
            this_process_goes_on: true,
        }
    );
}

#[test]
fn a_delivered_url_of_a_foreign_scheme_launches_nothing_and_ends_nothing() {
    assert_eq!(
        delivered_urls(["https://ejemplo.es".to_owned()], false),
        DeliveredUrls {
            site_launches: Vec::new(),
            this_process_goes_on: true,
        }
    );
}

fn invoked_with_the_words(words: &[&str]) -> Invocation {
    Invocation {
        command_line: std::iter::once("rfirma")
            .chain(words.iter().copied())
            .map(str::to_owned)
            .collect(),
        folder: PathBuf::from("/"),
    }
}

#[test]
fn a_command_as_the_first_argument_gives_the_terminal_role_whatever_the_case() {
    for command in ["sign", "COSIGN", "ListAliases", "verify"] {
        let words = [command, "-i", "a.pdf", "-o", "b.pdf"];

        assert_eq!(
            role_of(invoked_with_the_words(&words)),
            Role::Terminal(words.iter().map(|word| (*word).to_owned()).collect())
        );
    }
}

#[test]
fn the_commands_left_out_also_give_the_terminal_role_so_they_can_be_refused() {
    for command in ["countersign", "batchsign"] {
        assert!(matches!(
            role_of(invoked_with_the_words(&[command, "-i", "a.pdf"])),
            Role::Terminal(_)
        ));
    }
}

#[test]
fn an_afirma_url_wins_over_a_command() {
    assert_eq!(
        role_of(invoked_with_the_words(&["sign", A_LAUNCH])),
        Role::Site(A_LAUNCH.to_owned())
    );
}

#[test]
fn a_command_that_is_not_the_first_argument_gives_no_terminal_role() {
    let invocation = invoked_with_the_words(&["contrato.pdf", "sign"]);

    assert_eq!(role_of(invocation.clone()), Role::Desktop(invocation));
}

#[test]
fn the_help_no_longer_promises_that_there_is_no_unattended_mode() {
    assert!(!help(Platform::Linux).contains("desatendido"));
    for line in [
        "rfirma <orden> [parámetros…]",
        "rfirma <orden> --help",
        "--password-fd <N>",
        "--certtui",
        "secret-tool lookup",
        "--file-forwarding",
        "Desviaciones",
    ] {
        assert!(help(Platform::Linux).contains(line), "falta {line}");
    }
}

#[test]
fn the_version_is_asked_for_in_either_form() {
    for flag in ["--version", "-version"] {
        assert!(version_was_asked_for(["rfirma", flag]), "con {flag}");
    }
    assert!(!version_was_asked_for(["rfirma"]));
    assert!(
        !version_was_asked_for(["--version"]),
        "el ejecutable no cuenta"
    );
}

#[test]
fn the_debug_info_is_asked_for_only_by_its_flag() {
    assert!(debug_info_was_asked_for(["rfirma", "--debug-info"]));
    assert!(!debug_info_was_asked_for(["rfirma"]));
    assert!(!debug_info_was_asked_for([
        "rfirma",
        "--version",
        "debug-info"
    ]));
    assert!(
        !debug_info_was_asked_for(["--debug-info"]),
        "el ejecutable no cuenta"
    );
}

#[test]
fn the_help_names_the_debug_info_flag() {
    assert!(help(Platform::Linux).contains("--debug-info"));
    assert!(help(Platform::Windows).contains("--debug-info"));
}

#[test]
fn the_version_text_is_one_line_with_the_build_and_the_compatible_autofirma() {
    let build = RunningBuild {
        channel: "flatpak",
        system: "linux x86_64",
        library: "/app/lib/librfirma_crypto.so",
    };

    assert_eq!(
        version_text("1.2.3", build),
        "rfirma 1.2.3 (flatpak, linux x86_64) · compatible con AutoFirma 1.9.2 · lib: /app/lib/librfirma_crypto.so"
    );
}

#[test]
fn the_windows_help_offers_neither_password_fd_nor_certtui_nor_the_keyring_example() {
    let help = help(Platform::Windows);

    for absent in ["--password-fd", "--certtui", "secret-tool"] {
        assert!(!help.contains(absent), "sobra {absent}");
    }
    for present in [
        "--certgui",
        "--store",
        "--password ",
        "PIN del almacén de Windows",
    ] {
        assert!(help.contains(present), "falta {present}");
    }
}

#[test]
fn the_windows_help_is_what_the_process_prints_on_windows() {
    let arguments = ["rfirma".to_owned(), "--help".to_owned()];
    let build = RunningBuild {
        channel: "windows",
        system: "windows x86_64",
        library: "rfirma_crypto.dll",
    };

    assert_eq!(
        informative_text(&arguments, "1.0.0", build, Platform::Windows),
        Some(help(Platform::Windows))
    );
}
