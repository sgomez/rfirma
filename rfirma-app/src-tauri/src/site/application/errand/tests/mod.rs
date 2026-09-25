//! Pruebas del trámite de sede en grada A, repartidas por comportamiento.

mod support;
mod support_requests;
mod support_window;

mod batch_local;
mod batch_remote;
mod certificate_selection;
mod chosen_document;
mod countersignature_and_gzip;
mod dialog_folder;
mod document_and_save;
mod headless_and_checked;
mod pdf_awaiting_the_person;
mod pdf_password;
mod relay_window;
mod service;
mod shown_refusals;
mod signature_basics;
mod signature_consent;
mod signature_formats;
mod sticky_selection;
mod token_and_launch;
mod triphase_server;
#[cfg(feature = "conformance-autoconsent")]
mod unattended;
mod visible_area;
mod websocket;
