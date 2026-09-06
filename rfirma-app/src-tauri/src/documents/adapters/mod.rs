//! Adaptadores de `documents`: todo lo que toca el mundo, incluidas las órdenes y las vistas de Tauri.

pub mod failures;
pub mod recents_store;
pub mod rubric;
pub mod tauri;
pub mod tauri_rubric;
pub mod views;

pub use crate::documents::domain::portal;
