//! La lista en caliente de la ventana de sede: el vigilante del proceso de sede vuelve a cribar con el filtro de la sede y se lo anuncia a esa ventana (ADR-0048).

use tauri::Manager as _;

use crate::identity::adapters::readers::follow_the_readers_for;
use crate::identity::domain::certificate::ListedCertificate;
use crate::identity::domain::readers::{ready_card_among, ReadyCard};
use crate::identity::ports::Relisting;
use crate::identity::IdentityRoot;
use crate::site::application::errand::{after_the_readers, AfterTheReaders};

use super::window::{with_the_desk, SITE_WINDOW};

/// Quien vuelve a listar para la ventana de sede, con las asas que la sede ya concedió.
struct ForTheSiteWindow {
    app: tauri::AppHandle,
}

impl Relisting for ForTheSiteWindow {
    fn relisted(&self) -> (Option<Vec<ListedCertificate>>, Option<ReadyCard>) {
        let identity = self.app.state::<IdentityRoot>();
        let found = identity.certificates().unwrap_or_default();
        let ready = ready_card_among(&identity.only_on_a_card(&found));
        let after = with_the_desk(&self.app, |desk, live| after_the_readers(desk, found, live));
        let rows = match after {
            AfterTheReaders::Accepted(accepted) => Some(identity.rows_keeping_handles(accepted)),
            AfterTheReaders::Unchanged => None,
        };
        (rows, ready)
    }
}

/// Arranca en su propio hilo la lista en caliente de la ventana de sede.
pub fn follow_the_readers_for_the_site_window(app: &tauri::AppHandle) {
    let lending = app.clone();
    follow_the_readers_for(app, SITE_WINDOW, move |use_it| {
        use_it(&ForTheSiteWindow {
            app: lending.clone(),
        });
    });
}
