//! Lo que ve el informe de depuración del DNIe en el lector: ATR, versión y juego de claves, sin resetear la tarjeta.

use super::{take_the_card_turn, CardOpener};
use crate::identity::domain::seen_dnie::{DnieInTheReader, SeenDnie};

const DNIE_4_CHIP: [u8; 2] = [0x4C, 0x34];
const CHIP_TYPE: std::ops::Range<usize> = 13..15;

/// Lo que hay en el lector de `opener`: el DNIe que se ve, ninguno, o el error de rdnie al abrirlo.
pub fn seen_dnie<O: CardOpener>(opener: &O) -> DnieInTheReader {
    match described(opener) {
        Ok(seen) => DnieInTheReader::Seen(seen),
        Err(rdnie::Error::NoReader | rdnie::Error::NoCard | rdnie::Error::NotDnie) => {
            DnieInTheReader::Absent
        }
        Err(error) => DnieInTheReader::Unopened(error.to_string()),
    }
}

fn described<O: CardOpener>(opener: &O) -> Result<SeenDnie, rdnie::Error> {
    let _turn = take_the_card_turn();
    let session = opener.open()?;
    let key_set = match session.key_generation() {
        rdnie::KeyGeneration::First => "primero",
        _ => "segundo",
    };
    let atr = session.atr().as_bytes();
    let version = if atr.get(CHIP_TYPE) == Some(&DNIE_4_CHIP[..]) {
        "4.0"
    } else {
        "3.0"
    };
    Ok(SeenDnie {
        atr: atr.iter().map(|byte| format!("{byte:02X}")).collect(),
        version: version.to_owned(),
        key_set: key_set.to_owned(),
    })
}
