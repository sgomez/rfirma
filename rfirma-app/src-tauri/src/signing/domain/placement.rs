//! Conversión geométrica del recuadro visual a coordenadas PAdES del puente nativo (ADR-0006).

use super::config::PadesRect;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

/// La `/Rotate` de la página, que solo puede ser uno de cuatro valores.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rotation {
    /// Sin rotar.
    None,
    /// 90°.
    Quarter,
    /// 180°.
    Half,
    /// 270°.
    ThreeQuarters,
}

impl Rotation {
    /// La `/Rotate` que trae el PDF, normalizada a múltiplos de 90°.
    pub fn from_degrees(degrees: i32) -> Option<Self> {
        match degrees.rem_euclid(360) {
            0 => Some(Self::None),
            90 => Some(Self::Quarter),
            180 => Some(Self::Half),
            270 => Some(Self::ThreeQuarters),
            _ => None,
        }
    }

    /// Los grados, ya normalizados a `[0, 360)`.
    pub fn degrees(self) -> u32 {
        match self {
            Self::None => 0,
            Self::Quarter => 90,
            Self::Half => 180,
            Self::ThreeQuarters => 270,
        }
    }
}

/// La MediaBox de la página, con las esquinas ya ordenadas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MediaBox {
    lower_x: f64,
    lower_y: f64,
    upper_x: f64,
    upper_y: f64,
}

impl MediaBox {
    /// Las cuatro coordenadas tal y como vienen en el PDF, en cualquier orden.
    pub fn new(x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        Self {
            lower_x: x0.min(x1),
            lower_y: y0.min(y1),
            upper_x: x0.max(x1),
            upper_y: y0.max(y1),
        }
    }

    /// Esquina inferior izquierda, eje X.
    pub fn lower_x(&self) -> f64 {
        self.lower_x
    }

    /// Esquina inferior izquierda, eje Y.
    pub fn lower_y(&self) -> f64 {
        self.lower_y
    }

    /// Esquina superior derecha, eje X.
    pub fn upper_x(&self) -> f64 {
        self.upper_x
    }

    /// Esquina superior derecha, eje Y.
    pub fn upper_y(&self) -> f64 {
        self.upper_y
    }
}

/// La página sobre la que se ha arrastrado el recuadro.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Page {
    /// Número de página 1-based, tal cual lo numera el visor.
    pub number: u32,
    /// La MediaBox de esa página.
    pub media_box: MediaBox,
    /// Su `/Rotate`.
    pub rotation: Rotation,
}

/// El recuadro tal y como se arrastra en píxeles del lienzo del visor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewerRect {
    /// Primera esquina del arrastre, eje X.
    pub x0: f64,
    /// Primera esquina del arrastre, eje Y.
    pub y0: f64,
    /// Segunda esquina del arrastre, eje X.
    pub x1: f64,
    /// Segunda esquina del arrastre, eje Y.
    pub y1: f64,
}

/// El recuadro de firma visible tal como lo recuerda la bandeja: rectángulo y páginas.
#[derive(Clone, Debug, PartialEq)]
pub struct VisibleBox {
    /// Coordenadas del recuadro en espacio de usuario PDF: [x0, y0, x1, y1].
    pub rect: [f64; 4],
    /// Páginas en las que estampar la firma.
    pub pages: PageSet,
}

/// El recuadro en espacio de usuario PDF, redondeado a entero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UserSpaceRect {
    /// Esquina inferior izquierda, eje X.
    pub lower_left_x: i32,
    /// Esquina inferior izquierda, eje Y.
    pub lower_left_y: i32,
    /// Esquina superior derecha, eje X.
    pub upper_right_x: i32,
    /// Esquina superior derecha, eje Y.
    pub upper_right_y: i32,
}

impl UserSpaceRect {
    /// Construye el recuadro redondeado a entero a partir de coordenadas decimales.
    pub fn rounded(x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        Self {
            lower_left_x: round(x0.min(x1)),
            lower_left_y: round(y0.min(y1)),
            upper_right_x: round(x0.max(x1)),
            upper_right_y: round(y0.max(y1)),
        }
    }
}

impl Page {
    /// Convierte el arrastre del visor a coordenadas PAdES.
    pub fn place(&self, rect: &ViewerRect, zoom: f64) -> Result<PadesRect, OutOfPage> {
        self.pades_rect(&self.to_user_space(rect, zoom))
    }

    /// Convierte del lienzo al espacio de usuario PDF invirtiendo la matriz del visor.
    pub fn to_user_space(&self, rect: &ViewerRect, zoom: f64) -> UserSpaceRect {
        let transform = self.viewport_transform(zoom);
        let (ax, ay) = invert(transform, rect.x0, rect.y0);
        let (bx, by) = invert(transform, rect.x1, rect.y1);
        UserSpaceRect {
            lower_left_x: round(ax.min(bx)),
            lower_left_y: round(ay.min(by)),
            upper_right_x: round(ax.max(bx)),
            upper_right_y: round(ay.max(by)),
        }
    }

    /// Convierte coordenadas de espacio de usuario a puntos PAdES.
    pub fn pades_rect(&self, rect: &UserSpaceRect) -> Result<PadesRect, OutOfPage> {
        self.check_fits(rect)?;
        let (ax, ay) = self.inverse_itext(rect.lower_left_x, rect.lower_left_y);
        let (bx, by) = self.inverse_itext(rect.upper_right_x, rect.upper_right_y);
        Ok(PadesRect {
            lower_left_x: ax.min(bx),
            lower_left_y: ay.min(by),
            upper_right_x: ax.max(bx),
            upper_right_y: ay.max(by),
        })
    }

    /// Aplica la transformación inversa a un punto según la rotación de la página.
    fn inverse_itext(&self, x: i32, y: i32) -> (i32, i32) {
        let upper_x = round(self.media_box.upper_x());
        let upper_y = round(self.media_box.upper_y());
        match self.rotation {
            Rotation::None => (x, y),
            Rotation::Quarter => (y, upper_x - x),
            Rotation::Half => (upper_x - x, upper_y - y),
            Rotation::ThreeQuarters => (upper_y - y, x),
        }
    }

    fn check_fits(&self, rect: &UserSpaceRect) -> Result<(), OutOfPage> {
        let media_box = [
            round(self.media_box.lower_x()),
            round(self.media_box.lower_y()),
            round(self.media_box.upper_x()),
            round(self.media_box.upper_y()),
        ];
        let fits = rect.lower_left_x >= media_box[0]
            && rect.lower_left_y >= media_box[1]
            && rect.upper_right_x <= media_box[2]
            && rect.upper_right_y <= media_box[3];
        if fits {
            Ok(())
        } else {
            Err(OutOfPage {
                page: self.number,
                rect: [
                    rect.lower_left_x,
                    rect.lower_left_y,
                    rect.upper_right_x,
                    rect.upper_right_y,
                ],
                media_box,
            })
        }
    }

    /// Matriz del visor de PDF según escala y rotación.
    fn viewport_transform(&self, zoom: f64) -> [f64; 6] {
        let media = &self.media_box;
        let center_x = (media.lower_x() + media.upper_x()) / 2.0;
        let center_y = (media.lower_y() + media.upper_y()) / 2.0;
        let (a, b, c, d) = match self.rotation {
            Rotation::None => (1.0, 0.0, 0.0, -1.0),
            Rotation::Quarter => (0.0, 1.0, 1.0, 0.0),
            Rotation::Half => (-1.0, 0.0, 0.0, 1.0),
            Rotation::ThreeQuarters => (0.0, -1.0, -1.0, 0.0),
        };
        let (offset_x, offset_y) = if a == 0.0 {
            (
                (center_y - media.lower_y()).abs() * zoom,
                (center_x - media.lower_x()).abs() * zoom,
            )
        } else {
            (
                (center_x - media.lower_x()).abs() * zoom,
                (center_y - media.lower_y()).abs() * zoom,
            )
        };
        [
            a * zoom,
            b * zoom,
            c * zoom,
            d * zoom,
            offset_x - a * zoom * center_x - c * zoom * center_y,
            offset_y - b * zoom * center_x - d * zoom * center_y,
        ]
    }
}

/// Aplica la transformación inversa de matriz al punto.
fn invert(m: [f64; 6], x: f64, y: f64) -> (f64, f64) {
    let determinant = m[0] * m[3] - m[1] * m[2];
    let dx = x - m[4];
    let dy = y - m[5];
    (
        (m[3] * dx - m[2] * dy) / determinant,
        (m[0] * dy - m[1] * dx) / determinant,
    )
}

/// Redondeo al entero más cercano con medio punto hacia arriba.
fn round(value: f64) -> i32 {
    (value + 0.5).floor() as i32
}

/// El recuadro no cabe en la página.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutOfPage {
    /// La página en la que se intentó colocar, 1-based.
    pub page: u32,
    /// El recuadro que no cabe, `[llx, lly, urx, ury]`.
    pub rect: [i32; 4],
    /// La MediaBox de esa página, `[llx, lly, urx, ury]`.
    pub media_box: [i32; 4],
}

impl fmt::Display for OutOfPage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [rx0, ry0, rx1, ry1] = self.rect;
        let [mx0, my0, mx1, my1] = self.media_box;
        write!(
            f,
            "el recuadro ({rx0}, {ry0})-({rx1}, {ry1}) se sale de la página {}, \
             que va de ({mx0}, {my0}) a ({mx1}, {my1})",
            self.page
        )
    }
}

impl std::error::Error for OutOfPage {}

/// En qué páginas se estampa el recuadro.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PageSet {
    /// Todas las páginas del documento, sean cuantas sean.
    All,
    /// Estas y ninguna más, 1-based, ordenadas y sin repetir.
    Only(BTreeSet<u32>),
}

impl PageSet {
    /// El conjunto de una sola página.
    pub fn only_page(page: u32) -> Self {
        Self::Only(BTreeSet::from([page]))
    }

    /// Un conjunto explícito, o `None` si venía vacío.
    pub fn only(pages: impl IntoIterator<Item = u32>) -> Option<Self> {
        let pages: BTreeSet<u32> = pages.into_iter().collect();
        (!pages.is_empty()).then_some(Self::Only(pages))
    }

    /// Las páginas que este conjunto nombra en un documento de `page_count`.
    pub fn resolve(&self, page_count: u32) -> BTreeSet<u32> {
        match self {
            Self::All => (1..=page_count).collect(),
            Self::Only(pages) => pages.clone(),
        }
    }

    /// El literal de `signaturePages`.
    pub fn literal(&self) -> String {
        match self {
            Self::All => ALL_PAGES.to_owned(),
            Self::Only(pages) => pages
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(","),
        }
    }

    /// Valida que el documento contenga las páginas solicitadas.
    pub fn validate(&self, page_count: u32) -> Result<(), OutOfDocument> {
        let missing: Vec<u32> = match self {
            Self::All => Vec::new(),
            Self::Only(pages) => pages
                .iter()
                .copied()
                .filter(|page| *page < 1 || *page > page_count)
                .collect(),
        };
        if !missing.is_empty() || self.resolve(page_count).is_empty() {
            return Err(OutOfDocument {
                missing,
                page_count,
            });
        }
        Ok(())
    }
}

/// Por qué la colocación que pidió la ventana no vale.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlacementError {
    /// El destino no existe en el documento.
    OutOfDocument(OutOfDocument),
    /// La página está girada un ángulo que no es múltiplo de 90.
    BadRotation(i32),
    /// El recuadro no cabe en la página.
    OutOfPage(OutOfPage),
}

impl From<OutOfDocument> for PlacementError {
    fn from(out: OutOfDocument) -> Self {
        Self::OutOfDocument(out)
    }
}

impl From<OutOfPage> for PlacementError {
    fn from(out: OutOfPage) -> Self {
        Self::OutOfPage(out)
    }
}

impl fmt::Display for PlacementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfDocument(out) => write!(f, "{out}"),
            Self::BadRotation(degrees) => {
                write!(f, "una pagina no puede estar girada {degrees} grados")
            }
            Self::OutOfPage(out) => write!(f, "{out}"),
        }
    }
}

impl std::error::Error for PlacementError {}

/// Literal con el que el puente nombra todas las páginas.
const ALL_PAGES: &str = "all";

/// El destino no existe en el documento.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutOfDocument {
    /// Las páginas pedidas que el documento no tiene.
    pub missing: Vec<u32>,
    /// Cuántas páginas tiene el documento de verdad.
    pub page_count: u32,
}

impl fmt::Display for OutOfDocument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.missing.is_empty() {
            return write!(
                f,
                "el conjunto de paginas no nombra ninguna pagina de un documento de {}",
                self.page_count
            );
        }
        let missing: Vec<String> = self.missing.iter().map(u32::to_string).collect();
        write!(
            f,
            "el documento tiene {} paginas y el recuadro se colocaria en la {}",
            self.page_count,
            missing.join(", ")
        )
    }
}

impl std::error::Error for OutOfDocument {}

#[cfg(test)]
mod tests;
