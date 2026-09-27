use super::*;

const SCHEME: &str = "afirma";

struct ScratchHives {
    root: String,
    classes: Classes,
}

impl ScratchHives {
    fn new(tag: &str, ours: &str) -> Self {
        let root = format!(r"Software\rfirma-test-{tag}-{}", std::process::id());
        let leak = |branch: &str| -> &'static str {
            Box::leak(format!(r"{root}\{branch}").into_boxed_str())
        };
        let classes = Classes::new(
            Hive::new(HKEY_CURRENT_USER, leak("user")),
            Hive::new(HKEY_CURRENT_USER, leak("machine")),
            PathBuf::from(ours),
        );
        Self { root, classes }
    }

    fn machine_has(&self, program: &str) {
        set_string(
            HKEY_CURRENT_USER,
            &format!(
                r"{}\{SCHEME}\shell\open\command",
                self.classes.machine.classes
            ),
            None,
            &format!("\"{program}\" \"%1\""),
        )
        .expect("se escribe la rama de la máquina");
    }

    fn user_command(&self) -> Option<String> {
        get_string(
            HKEY_CURRENT_USER,
            &format!(r"{}\{SCHEME}\shell\open\command", self.classes.user.classes),
        )
    }
}

impl Drop for ScratchHives {
    fn drop(&mut self) {
        let root = wide(&self.root);
        unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, root.as_ptr()) };
    }
}

const OURS: &str = r"C:\Programas\rFirma\rfirma.exe";
const AUTOFIRMA: &str = r"C:\Program Files\AutoFirma\AutoFirma\AutoFirma.exe";

#[test]
fn the_program_of_a_command_is_quoted_or_up_to_the_first_space() {
    assert_eq!(
        program_of(r#""C:\Program Files\AutoFirma\AutoFirma.exe" "%1""#).as_deref(),
        Some(r"C:\Program Files\AutoFirma\AutoFirma.exe")
    );
    assert_eq!(
        program_of(r"C:\rfirma.exe %1").as_deref(),
        Some(r"C:\rfirma.exe")
    );
    assert_eq!(program_of("  "), None);
}

#[test]
fn with_nothing_written_only_rfirma_is_offered_and_nobody_is_chosen() {
    let hives = ScratchHives::new("nothing", OURS);
    assert_eq!(
        hives.classes.handlers_for(SCHEME),
        vec![UrlHandler {
            id: OUR_DESKTOP_FILE.to_owned(),
            name: "rFirma".to_owned(),
        }]
    );
    assert_eq!(hives.classes.current_handler_for(SCHEME), None);
}

#[test]
fn autofirma_on_the_machine_is_offered_by_its_name_and_is_the_current_one() {
    let hives = ScratchHives::new("machine", OURS);
    hives.machine_has(AUTOFIRMA);

    let names: Vec<String> = hives
        .classes
        .handlers_for(SCHEME)
        .into_iter()
        .map(|handler| handler.name)
        .collect();
    assert_eq!(names, vec!["rFirma", "AutoFirma"]);
    assert_eq!(
        hives.classes.current_handler_for(SCHEME).as_deref(),
        Some(AUTOFIRMA)
    );
}

#[test]
fn choosing_rfirma_writes_the_user_branch_that_wins_over_the_machine() {
    let hives = ScratchHives::new("choose", OURS);
    hives.machine_has(AUTOFIRMA);

    hives
        .classes
        .choose_handler_for(SCHEME, OUR_DESKTOP_FILE)
        .expect("se registra");

    assert_eq!(
        hives.user_command().as_deref(),
        Some(format!("\"{OURS}\" \"%1\"").as_str())
    );
    assert_eq!(
        hives.classes.current_handler_for(SCHEME).as_deref(),
        Some(OUR_DESKTOP_FILE)
    );
}

#[test]
fn choosing_the_machine_handler_removes_the_user_branch() {
    let hives = ScratchHives::new("back", OURS);
    hives.machine_has(AUTOFIRMA);
    hives
        .classes
        .choose_handler_for(SCHEME, OUR_DESKTOP_FILE)
        .expect("se registra");

    hives
        .classes
        .choose_handler_for(SCHEME, AUTOFIRMA)
        .expect("vuelve AutoFirma");

    assert_eq!(hives.user_command(), None);
    assert_eq!(
        hives.classes.current_handler_for(SCHEME).as_deref(),
        Some(AUTOFIRMA)
    );
}

#[test]
fn an_unknown_handler_cannot_be_chosen() {
    let hives = ScratchHives::new("unknown", OURS);
    assert!(hives
        .classes
        .choose_handler_for(SCHEME, "otro.exe")
        .is_err());
}

#[test]
fn removing_takes_away_only_a_user_branch_that_is_ours() {
    let ours = ScratchHives::new("remove-ours", OURS);
    ours.classes
        .choose_handler_for(SCHEME, OUR_DESKTOP_FILE)
        .expect("se registra");
    ours.classes.remove_ours_for(SCHEME).expect("se retira");
    assert_eq!(ours.user_command(), None);

    let theirs = ScratchHives::new("remove-theirs", OURS);
    set_string(
        HKEY_CURRENT_USER,
        &format!(
            r"{}\{SCHEME}\shell\open\command",
            theirs.classes.user.classes
        ),
        None,
        &format!("\"{AUTOFIRMA}\" \"%1\""),
    )
    .expect("AutoFirma por usuario");
    theirs.classes.remove_ours_for(SCHEME).expect("no falla");
    assert!(theirs.user_command().is_some());
}

#[test]
fn a_stale_rfirma_elsewhere_is_not_ours_and_asks_for_repair() {
    let hives = ScratchHives::new("stale", OURS);
    set_string(
        HKEY_CURRENT_USER,
        &format!(
            r"{}\{SCHEME}\shell\open\command",
            hives.classes.user.classes
        ),
        None,
        r#""C:\viejo\rfirma.exe" "%1""#,
    )
    .expect("un rFirma de otra ruta");

    assert_ne!(
        hives.classes.current_handler_for(SCHEME).as_deref(),
        Some(OUR_DESKTOP_FILE)
    );
    hives
        .classes
        .choose_handler_for(SCHEME, OUR_DESKTOP_FILE)
        .expect("se repara");
    assert_eq!(
        hives.classes.current_handler_for(SCHEME).as_deref(),
        Some(OUR_DESKTOP_FILE)
    );
}
