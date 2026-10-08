//! Módulo PKCS#11 falso que se porta como el DNIe medido por OpenSC, y la tarjeta de pruebas que lo arranca.

mod calls;
mod card;
mod ffi;
mod harness;
mod info;
mod material;
mod module;
mod objects;
mod pin;
mod signing;

pub use ffi::C_GetFunctionList;
pub use harness::FakeCard;
