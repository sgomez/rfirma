//! Las órdenes de Tauri: lo único que la ventana puede pedir. No deciden nada.

pub mod desktop;
pub mod documents;
pub mod failure;
pub mod identity;
pub mod orders;
pub mod rubric;
pub mod signing;
pub mod site;
pub mod site_window;
pub mod views;
pub mod views_site;

#[cfg(test)]
mod guards;

pub use crate::app::documents::dropped_document;
pub use crate::app::invocation::second_invocation;
pub use crate::app::invocation::PendingInvocation;
pub use crate::app::signing::SigningSession;
pub use failure::Failure;
pub use orders::{PlacementOrder, SigningOrder};
pub use rubric::{RubricChoiceView, RubricView};
pub use site_window::{
    attend_site_operation, open_the_site_window, publish_the_moment, SITE_ERRAND, SITE_WINDOW,
};
pub use views::{
    CertificateView, ConfigurationView, DestinationView, DroppedDocumentView, NewVersionView,
    OpenedDocumentView, PlacementView, RecentDocumentView, SecretView, SignedDocumentView,
    UrlHandlerView, UrlHandlersView,
};
pub use views_site::{
    NoCertificateView, NoChannelView, RefusalSituationView, SignatureRoundView, SiteErrandView,
    SiteOutcomeView, SiteStageView,
};

pub use desktop::{
    __cmd__check_for_new_version, __cmd__choose_url_handler, __cmd__read_invocation,
    __cmd__url_handlers, __tauri_command_name_check_for_new_version,
    __tauri_command_name_choose_url_handler, __tauri_command_name_read_invocation,
    __tauri_command_name_url_handlers, check_for_new_version, choose_url_handler, read_invocation,
    url_handlers,
};
pub use documents::{
    __cmd__choose_destination, __cmd__choose_rubric, __cmd__forget_recent, __cmd__list_recents,
    __cmd__open_document, __cmd__open_signed_document, __cmd__open_signed_folder,
    __cmd__preview_destination, __cmd__read_document, __cmd__read_rubric, __cmd__record_recent,
    __tauri_command_name_choose_destination, __tauri_command_name_choose_rubric,
    __tauri_command_name_forget_recent, __tauri_command_name_list_recents,
    __tauri_command_name_open_document, __tauri_command_name_open_signed_document,
    __tauri_command_name_open_signed_folder, __tauri_command_name_preview_destination,
    __tauri_command_name_read_document, __tauri_command_name_read_rubric,
    __tauri_command_name_record_recent, choose_destination, choose_rubric, forget_recent,
    list_recents, open_document, open_signed_document, open_signed_folder, preview_destination,
    read_document, read_rubric, record_recent,
};
pub use identity::{
    __cmd__install_certificate, __cmd__list_certificates, __cmd__remove_certificate,
    __tauri_command_name_install_certificate, __tauri_command_name_list_certificates,
    __tauri_command_name_remove_certificate, install_certificate, list_certificates,
    remove_certificate,
};
pub use signing::{
    __cmd__begin_signing, __cmd__cancel_signing, __cmd__finish_signing, __cmd__forget_activity,
    __cmd__pades_lower_left, __cmd__preview_signature, __cmd__read_configuration,
    __cmd__sign_with_pin, __cmd__unregistered_signatures, __cmd__write_configuration,
    __tauri_command_name_begin_signing, __tauri_command_name_cancel_signing,
    __tauri_command_name_finish_signing, __tauri_command_name_forget_activity,
    __tauri_command_name_pades_lower_left, __tauri_command_name_preview_signature,
    __tauri_command_name_read_configuration, __tauri_command_name_sign_with_pin,
    __tauri_command_name_unregistered_signatures, __tauri_command_name_write_configuration,
    begin_signing, cancel_signing, finish_signing, forget_activity, pades_lower_left,
    preview_signature, read_configuration, sign_with_pin, unregistered_signatures,
    write_configuration,
};
pub use site::{
    __cmd__close_site_window, __cmd__install_local_ca, __cmd__read_site_errand,
    __cmd__site_begin_signing, __cmd__site_decline, __cmd__site_finish_signing,
    __cmd__site_identify, __cmd__site_install_certificate, __cmd__site_look_again,
    __tauri_command_name_close_site_window, __tauri_command_name_install_local_ca,
    __tauri_command_name_read_site_errand, __tauri_command_name_site_begin_signing,
    __tauri_command_name_site_decline, __tauri_command_name_site_finish_signing,
    __tauri_command_name_site_identify, __tauri_command_name_site_install_certificate,
    __tauri_command_name_site_look_again, close_site_window, install_local_ca, read_site_errand,
    site_begin_signing, site_decline, site_finish_signing, site_identify, site_install_certificate,
    site_look_again,
};

/// Nombre del evento emitido cuando se suelta un documento en la ventana.
pub const DOCUMENT_DROPPED: &str = "document-dropped";
