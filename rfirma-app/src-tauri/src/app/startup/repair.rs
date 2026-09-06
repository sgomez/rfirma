//! **La reparación de la CA local desde la ventana de sede** (ID-329, ID-341):
//! lo que la persona pide con el botón delante cuando el canal no llega a
//! abrirse, y en qué queda la pantalla después.

use std::path::PathBuf;

use crate::tls::LocalCaStore;
use crate::trust::{Moment as TrustMoment, NssTrustStores};

use super::super::errand::{LiveErrand, Moment, NoChannel};
use super::super::trust;
use super::channel::HeldChannel;

/// **Dónde está la CA local y qué perfiles NSS hay que dejarla de confianza**
/// (ID-329), para que la ventana de sede pueda instalarla cuando el canal no
/// llega a abrirse.
///
/// Las mismas dos cosas que el arranque le pasa a [`attend_startup`],
/// sostenidas en el estado porque la reparación las necesita mucho después: el
/// arranque las resuelve una vez y esto es su copia viva.
pub struct LocalCaTrust {
    /// Las dos ranuras de la CA local: la que sirve y la del solape.
    pub store: LocalCaStore,
    /// Los perfiles NSS que se intentan recorrer.
    pub profiles: Vec<PathBuf>,
}

/// **Caso de uso.** Instala la CA local en los almacenes NSS de la persona
/// porque ella lo ha pedido desde la pantalla de reparación, y apunta en qué
/// queda esa pantalla (ID-329, ID-341).
///
/// Es la **acción principal** de la reparación: sin la CA local ningún
/// navegador llega a intentar el canal, así que el resto de la receta —el
/// permiso de red local— sobra hasta que esté. La pide la persona, con el
/// botón delante; no es un refresco automático a mitad de trámite, que es lo
/// que el ID-224 prohíbe.
///
/// Por eso el momento que se le pasa es el del arranque y no el de mitad de
/// trámite: lo que se pide es exactamente el trabajo del arranque —instalar la
/// que hay, o fabricarla si no la hay—, mientras que a mitad de trámite está
/// definido como «no hacer nada». El aviso de reiniciar el navegador se
/// descarta y no se enseña: la ventana de sede no tiene dónde ponerlo, y ésa es
/// la mitad del ID-224 que sigue en pie.
///
/// Lo que la ventana ve después son **dos preguntas y no una**: si la CA local
/// ha quedado en algún almacén, y si hay canal sirviendo. Las decide
/// [`what_the_repair_leaves`].
pub fn repair_the_local_ca(trust: &LocalCaTrust, held: &HeldChannel, live: &LiveErrand) -> Moment {
    let in_some_store = trust::refresh_local_ca_trust(
        &trust.store,
        &trust.profiles,
        &NssTrustStores,
        TrustMoment::Startup,
    )
    .is_ok_and(|outcome| !outcome.nowhere());

    let moment = what_the_repair_leaves(in_some_store, held.is_serving());
    live.note(moment.clone());
    moment
}

/// **En qué queda la pantalla de reparación después de instalar la CA local**
/// (ID-341).
///
/// Las dos preguntas son distintas y hasta el #402 se confundían: que la CA haya
/// entrado en un almacén NSS no dice que el canal esté en pie. Al botón se llega
/// desde tres sitios —la CA que falta, el canal que no se abrió y la espera
/// pasada de plazo— y sólo desde el primero es cierto que el canal sigue
/// sirviendo.
///
/// Y el canal **no se reabre desde aquí**: el transporte se abre una sola vez,
/// en el arranque, y allí se emite el certificado del servidor. Así que con la
/// CA instalada pero sin canal la respuesta correcta es la pantalla de
/// reparación definitiva —la que lleva la dirección del ajuste del navegador—,
/// no treinta segundos de «Conectando con la sede» sobre algo que el backend
/// ya sabe que no va a llegar.
fn what_the_repair_leaves(in_some_store: bool, channel_is_serving: bool) -> Moment {
    match (in_some_store, channel_is_serving) {
        // Sin CA en ningún almacén ningún navegador llega a intentar el canal:
        // la reparación sigue siendo instalarla.
        (false, _) => Moment::NoChannel(NoChannel::LocalCaMissing),
        // Con CA y con canal la petición de la sede puede llegar ya.
        (true, true) => Moment::Waiting,
        // Con CA y sin canal, lo que de verdad le pasa a la persona.
        (true, false) => Moment::NoChannel(NoChannel::ChannelNotOpened),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **La reparación no manda esperar sobre un canal que no existe**
    /// (ID-341).
    #[test]
    fn the_repair_only_waits_when_a_channel_is_serving() {
        assert_eq!(what_the_repair_leaves(true, true), Moment::Waiting);
        assert_eq!(
            what_the_repair_leaves(true, false),
            Moment::NoChannel(NoChannel::ChannelNotOpened)
        );
    }

    /// Y sin CA en ningún almacén la respuesta sigue siendo instalarla, haya
    /// canal o no: ningún navegador llega a intentar abrirlo (ID-329).
    #[test]
    fn the_repair_asks_for_the_local_ca_again_when_it_reached_no_store() {
        for serving in [true, false] {
            assert_eq!(
                what_the_repair_leaves(false, serving),
                Moment::NoChannel(NoChannel::LocalCaMissing)
            );
        }
    }
}
