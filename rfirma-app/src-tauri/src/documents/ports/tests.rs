use std::path::PathBuf;

use super::{DialogClues, PortalDialogs};

struct TestDialogs {
    file: Option<PathBuf>,
    files: Vec<PathBuf>,
    save: Option<PathBuf>,
}

impl PortalDialogs for TestDialogs {
    fn pick_file(&self, _clues: &DialogClues) -> Result<Option<PathBuf>, String> {
        Ok(self.file.clone())
    }

    fn pick_files(&self, _clues: &DialogClues) -> Result<Vec<PathBuf>, String> {
        Ok(self.files.clone())
    }

    fn save_file(&self, _clues: &DialogClues) -> Result<Option<PathBuf>, String> {
        Ok(self.save.clone())
    }
}

#[test]
fn default_clues_are_empty() {
    let clues = DialogClues::new();
    assert_eq!(clues.title, None);
    assert_eq!(clues.filename, None);
    assert!(clues.extensions.is_empty());
    assert_eq!(clues.description, None);
    assert_eq!(clues.starting_folder, None);
}

#[test]
fn builder_populates_all_clues() {
    let clues = DialogClues::new()
        .with_title("Seleccionar documento")
        .with_filename("propuesta.pdf")
        .with_filter("Archivos PDF", &["pdf"])
        .with_starting_folder(PathBuf::from("/tmp"));

    assert_eq!(clues.title.as_deref(), Some("Seleccionar documento"));
    assert_eq!(clues.filename.as_deref(), Some("propuesta.pdf"));
    assert_eq!(clues.extensions, vec!["pdf".to_string()]);
    assert_eq!(clues.description.as_deref(), Some("Archivos PDF"));
    assert_eq!(clues.starting_folder, Some(PathBuf::from("/tmp")));
}

#[test]
fn portal_dialogs_trait_is_usable() {
    let portal: Box<dyn PortalDialogs> = Box::new(TestDialogs {
        file: Some(PathBuf::from("/home/user/doc.pdf")),
        files: vec![
            PathBuf::from("/home/user/1.pdf"),
            PathBuf::from("/home/user/2.pdf"),
        ],
        save: Some(PathBuf::from("/home/user/guardado.pdf")),
    });

    let clues = DialogClues::new();
    assert_eq!(
        portal.pick_file(&clues).unwrap(),
        Some(PathBuf::from("/home/user/doc.pdf"))
    );
    assert_eq!(portal.pick_files(&clues).unwrap().len(), 2);
    assert_eq!(
        portal.save_file(&clues).unwrap(),
        Some(PathBuf::from("/home/user/guardado.pdf"))
    );
}
