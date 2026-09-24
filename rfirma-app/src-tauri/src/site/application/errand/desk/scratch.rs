//! El documento de paso de la mesa: dónde se escribe y con qué extensión; no decide cuándo se borra.

use std::path::PathBuf;

use crate::documents::domain::handles;
use crate::signing::domain::bridge::Format;
use crate::site::application::session::SiteRefusal;
use crate::site::ports::{FilterEngine, PolicyEngine};

use super::super::state::LiveErrand;
use super::{ErrandDesk, Neighbours};

pub(in crate::site::application) fn keep_the_document<
    E: FilterEngine,
    P: PolicyEngine,
    N: Neighbours,
>(
    desk: &ErrandDesk<'_, E, P, N>,
    live: &LiveErrand,
    format: Format,
    bytes: &[u8],
) -> Result<String, SiteRefusal> {
    let path = write_the_document(desk, format, bytes)?;
    live.keep_the_scratch(path.clone(), desk.scratch.clone());
    Ok(desk.neighbours.open_unrecorded(path))
}

/// Deja el documento en el directorio de paso sin apuntarlo en el trámite: quien lo llame decide
/// cuándo borrarlo (el lote local lo hace elemento a elemento, no al final del trámite).
pub(in crate::site::application) fn write_the_document<
    E: FilterEngine,
    P: PolicyEngine,
    N: Neighbours,
>(
    desk: &ErrandDesk<'_, E, P, N>,
    format: Format,
    bytes: &[u8],
) -> Result<PathBuf, SiteRefusal> {
    desk.scratch
        .make_the_folder(&desk.scratch_dir)
        .map_err(SiteRefusal::ScratchFolderMissing)?;
    let path = desk
        .scratch_dir
        .join(format!("{}.{}", handles::mint(), what_arrives_in(format)));
    desk.scratch
        .write(&path, bytes)
        .map_err(SiteRefusal::ScratchUnwritable)?;
    Ok(path)
}

/// La extensión del documento que se firma en ese formato, no la de la firma que sale.
fn what_arrives_in(format: Format) -> &'static str {
    match format {
        Format::Pades => "pdf",
        Format::Xades(_) | Format::FacturaE => "xml",
        Format::Cades | Format::CadesAsicS | Format::Cms | Format::Pkcs1 => "bin",
    }
}
