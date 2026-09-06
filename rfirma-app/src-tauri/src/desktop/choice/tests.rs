use super::*;

#[test]
fn the_list_that_gets_written_is_the_one_in_the_home() {
    let list = mimeapps_list(&|name| match name {
        "XDG_CONFIG_HOME" => Some(OsString::from("/home/quien/.config")),
        _ => None,
    })
    .expect("deberia resolverse");

    assert_eq!(list, PathBuf::from("/home/quien/.config/mimeapps.list"));
}

#[test]
fn choosing_a_handler_writes_an_explicit_default() {
    let updated = with_explicit_default("", "x-scheme-handler/afirma", "rfirma.desktop");

    assert_eq!(
        updated,
        "[Default Applications]\nx-scheme-handler/afirma=rfirma.desktop;\n"
    );
}

#[test]
fn the_default_never_lands_in_another_group() {
    let updated = with_explicit_default(
        "[Added Associations]\nx-scheme-handler/afirma=autofirma.desktop;\n",
        "x-scheme-handler/afirma",
        "rfirma.desktop",
    );

    assert_eq!(
        updated,
        "[Added Associations]\nx-scheme-handler/afirma=autofirma.desktop;\n\
         \n[Default Applications]\nx-scheme-handler/afirma=rfirma.desktop;\n"
    );
}

#[test]
fn an_existing_group_takes_the_line_inside_it() {
    let updated = with_explicit_default(
        "[Default Applications]\napplication/pdf=evince.desktop\n",
        "x-scheme-handler/afirma",
        "rfirma.desktop",
    );

    assert_eq!(
        updated,
        "[Default Applications]\napplication/pdf=evince.desktop\n\
         x-scheme-handler/afirma=rfirma.desktop;\n"
    );
}

#[test]
fn an_existing_default_for_the_scheme_is_replaced_not_duplicated() {
    let updated = with_explicit_default(
        "[Default Applications]\n\
         x-scheme-handler/afirma=autofirma.desktop\n\
         application/pdf=evince.desktop\n",
        "x-scheme-handler/afirma",
        "rfirma.desktop",
    );

    assert_eq!(
        updated,
        "[Default Applications]\n\
         x-scheme-handler/afirma=rfirma.desktop;\n\
         application/pdf=evince.desktop\n"
    );
    assert_eq!(updated.matches("x-scheme-handler/afirma").count(), 1);
}

#[test]
fn everything_else_in_the_list_survives_untouched() {
    let before = "# lo escribio otra cosa\n\
                  [Added Associations]\n\
                  application/pdf=evince.desktop;okular.desktop;\n\
                  \n\
                  [Default Applications]\n\
                  application/pdf=evince.desktop\n\
                  \n\
                  [Removed Associations]\n\
                  text/plain=gedit.desktop;\n";

    let updated = with_explicit_default(before, "x-scheme-handler/afirma", "rfirma.desktop");

    assert!(updated.contains("# lo escribio otra cosa\n[Added Associations]\n"));
    assert!(updated.contains("[Removed Associations]\ntext/plain=gedit.desktop;\n"));
    assert!(updated.contains(
        "[Default Applications]\n\
         application/pdf=evince.desktop\n\
         x-scheme-handler/afirma=rfirma.desktop;\n\n[Removed Associations]"
    ));
}

#[test]
fn a_commented_out_line_is_not_the_entry() {
    let updated = with_explicit_default(
        "[Default Applications]\n#x-scheme-handler/afirma=autofirma.desktop\n",
        "x-scheme-handler/afirma",
        "rfirma.desktop",
    );

    assert!(updated.contains("#x-scheme-handler/afirma=autofirma.desktop\n"));
    assert!(updated.contains("\nx-scheme-handler/afirma=rfirma.desktop;\n"));
}

#[test]
fn the_choice_lands_on_disk_even_when_there_was_no_list() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let list = directory.path().join("nuevo").join("mimeapps.list");

    let written = choose_handler_for_scheme(Channel::Native, &list, "afirma", "rfirma.desktop")
        .expect("deberia escribirse");

    assert_eq!(written.list(), list);
    assert_eq!(
        fs::read_to_string(&list).expect("deberia leerse"),
        "[Default Applications]\nx-scheme-handler/afirma=rfirma.desktop;\n"
    );
}

#[test]
fn writing_the_choice_leaves_no_leftovers_behind() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let list = directory.path().join("mimeapps.list");

    choose_handler_for_scheme(Channel::Native, &list, "afirma", "rfirma.desktop")
        .expect("deberia escribirse");

    let left: Vec<_> = fs::read_dir(directory.path())
        .expect("deberia leerse el directorio")
        .map(|entry| entry.expect("deberia haber entrada").file_name())
        .collect();
    assert_eq!(left, vec![OsString::from("mimeapps.list")]);
}

#[test]
fn inside_the_sandbox_no_default_is_written() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let list = directory.path().join("mimeapps.list");

    let refused = choose_handler_for_scheme(Channel::Flatpak, &list, "afirma", "rfirma.desktop")
        .expect_err("no deberia escribirse dentro del sandbox");

    assert_eq!(refused.situation(), Situation::NotAvailableInsideTheSandbox);
    assert!(!list.exists());
}

#[test]
fn the_written_choice_carries_what_firefox_can_override() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let list = directory.path().join("mimeapps.list");

    let written = choose_handler_for_scheme(Channel::Native, &list, "afirma", "rfirma.desktop")
        .expect("deberia escribirse");

    assert_eq!(
        written.overridden_by(),
        [ChoiceOverride::FirefoxKeepsItsOwn]
    );
}

#[test]
fn the_written_default_is_what_gets_read_back() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let list = directory.path().join("mimeapps.list");
    choose_handler_for_scheme(Channel::Native, &list, "afirma", "rfirma.desktop")
        .expect("deberia escribirse");

    let current = current_default_for_scheme(Channel::Native, &list, "afirma");

    assert_eq!(current.as_deref(), Some("rfirma.desktop"));
}

#[test]
fn no_list_means_nobody_has_been_chosen() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");

    let current = current_default_for_scheme(
        Channel::Native,
        &directory.path().join("mimeapps.list"),
        "afirma",
    );

    assert_eq!(current, None);
}

#[test]
fn an_added_association_is_not_the_default() {
    let content = "[Added Associations]\nx-scheme-handler/afirma=autofirma.desktop;\n";

    assert_eq!(default_in(content, "x-scheme-handler/afirma"), None);
}

#[test]
fn the_first_entry_of_the_list_is_the_one_that_answers() {
    let content =
        "[Default Applications]\nx-scheme-handler/afirma=rfirma.desktop;autofirma.desktop;\n";

    assert_eq!(
        default_in(content, "x-scheme-handler/afirma"),
        Some("rfirma.desktop".to_owned())
    );
}

#[test]
fn inside_the_sandbox_nothing_is_read_either() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let list = directory.path().join("mimeapps.list");
    std::fs::write(
        &list,
        "[Default Applications]\nx-scheme-handler/afirma=rfirma.desktop;\n",
    )
    .expect("deberia escribirse");

    assert_eq!(
        current_default_for_scheme(Channel::Flatpak, &list, "afirma"),
        None
    );
}
