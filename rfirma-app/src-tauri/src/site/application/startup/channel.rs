//! Gestión del ciclo de vida y retención de canales de comunicación abiertos con la sede.

use crate::site::domain::channel::OpenChannel;

use super::super::site::Attendance;

/// Contenedor en memoria de los canales de comunicación activos para trámites o rechazos.
#[derive(Default)]
pub struct HeldChannel {
    /// Canal asignado al trámite en curso.
    serving: std::sync::Mutex<Option<OpenChannel>>,
    /// Canal temporal para emitir un código de rechazo por socket.
    refusing: std::sync::Mutex<Option<OpenChannel>>,
}

impl HeldChannel {
    /// Almacena el canal de un trámite activo cerrando el anterior si existía.
    pub fn hold(&self, channel: OpenChannel) {
        if let Some(previous) = crate::lock(&self.serving).replace(channel) {
            previous.close();
        }
    }

    /// Comprueba si existe un canal de trámite activo en servicio.
    pub fn is_serving(&self) -> bool {
        crate::lock(&self.serving).is_some()
    }

    /// Almacena un canal temporal para comunicar un rechazo por socket.
    pub fn hold_a_refusal(&self, channel: OpenChannel) {
        if let Some(previous) = crate::lock(&self.refusing).replace(channel) {
            previous.close();
        }
    }
}

/// Registra el canal resultante de la atención de sede o emite el aviso correspondiente.
pub fn hold_the_channel(held: &HeldChannel, attendance: Attendance) -> Vec<String> {
    match attendance {
        Attendance::Serving { channel, .. } => {
            held.hold(channel);
            Vec::new()
        }
        Attendance::RefusingOverTheChannel { channel, .. } => {
            held.hold_a_refusal(channel);
            Vec::new()
        }
        Attendance::RefusingInTheWindow(refusal) => vec![format!(
            "rfirma: la invocacion de sede se rechaza con {} y no hay canal por el que decirlo: {}",
            refusal.answer().on_the_wire(),
            refusal.detail()
        )],
        Attendance::ChannelNotOpened(error) => vec![format!(
            "rfirma: la invocacion de sede era buena pero no se abrio el canal ({error})"
        )],
    }
}

#[cfg(test)]
mod tests;
