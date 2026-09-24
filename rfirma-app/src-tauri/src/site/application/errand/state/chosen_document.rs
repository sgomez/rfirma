//! El nombre del fichero que la persona eligió para firmar, que la respuesta devuelve a la sede.

use super::LiveErrand;

impl LiveErrand {
    /// Apunta el nombre del fichero que la persona acaba de elegir para firmar.
    pub(in crate::site::application::errand) fn name_the_chosen_document(&self, name: String) {
        *crate::lock(&self.chosen_document) = Some(name);
    }

    /// El nombre del fichero elegido para firmar en esta operación, si lo eligió la persona.
    pub(in crate::site::application::errand) fn the_chosen_document(&self) -> Option<String> {
        crate::lock(&self.chosen_document).clone()
    }
}
