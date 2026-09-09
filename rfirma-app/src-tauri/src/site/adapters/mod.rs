//! Adaptadores de `site`: todo lo que toca el mundo, incluidas las órdenes y las vistas de Tauri.

pub mod batch_services;
pub mod channel;
pub mod codec;
pub mod codec_relay;
pub mod codec_v1;
pub mod codec_v3;
pub mod data_download;
pub mod desk;
pub mod frontier;
pub mod nss;
pub mod relay;
pub mod scratch;
pub mod service;
pub mod servlets;
pub mod tauri;
pub mod tls;
pub mod trace;
pub mod transport;
pub mod views;
pub mod window;
