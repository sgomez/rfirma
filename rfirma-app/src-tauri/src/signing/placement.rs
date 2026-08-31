//! Del recuadro que se arrastra sobre el visor al `/Rect` del PDF (ID-21,
//! ID-22).
//!
//! Son **dos pasos**, y el segundo no se ve venir:
//!
//! 1. **Lienzo → espacio de usuario PDF.** Es lo que hace `convertToPdfPoint`
//!    de `pdf.js`: invertir la matriz del viewport, que deshace de golpe la
//!    escala, el volteo del eje Y, la `/Rotate` de la página y el origen de la
//!    MediaBox. El resultado es **exactamente el `/Rect` que acabará teniendo
//!    el widget de firma**.
//! 2. **Espacio de usuario → `extraParams`.** `T⁻¹`. AutoFirma entrega el
//!    rectángulo tal cual a `setVisibleSignature(Rectangle, page, null)`, pero
//!    iText lo transforma según la `/Rotate` **al cerrar el documento**, antes
//!    de escribir el `/Rect`. Como el widget acaba en `T(entrada)` y queremos
//!    que acabe en el rectángulo del paso 1, hay que entregar `T⁻¹` de ese
//!    rectángulo.
//!
//! `T` usa los **límites superiores** de la MediaBox, no la anchura ni la
//! altura. Con la MediaBox en el origen las dos cosas coinciden y el error se
//! esconde: sin rotación y con la MediaBox en `(0,0)` los dos caminos dan lo
//! mismo, así que el fallo no aparece en el PDF de prueba de nadie y coloca la
//! firma en el sitio equivocado en el del usuario, sin lanzar excepción.
//!
//! Las coordenadas se emiten como enteros porque AutoFirma las lee como `int`
//! (`PdfUtil.getPositionOnPage`), y el recuadro se guarda en espacio de usuario
//! y no en píxeles, o el zoom lo desplaza solo.
//!
//! La tabla de `T`, las dieciséis mediciones que la respaldan y el banco de
//! pruebas están en `docs/research/coordenadas-recuadro-pades.md`. Si sube la
//! versión de `afirma-lib-itext`, vuelve a medirla: es un hecho sobre esa
//! librería, no sobre el formato PAdES.

use super::config::SignatureBox;
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
    /// La `/Rotate` que trae el PDF, normalizada.
    ///
    /// Acepta múltiplos de 90 de cualquier signo y magnitud (`-90`, `450`),
    /// porque eso es lo que permite el formato. Cualquier otra cosa devuelve
    /// `None`: no se redondea al cuadrante más cercano, porque una página con
    /// una `/Rotate` que el formato no admite es un documento roto y firmarlo
    /// a ojo colocaría la firma en cualquier sitio.
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
///
/// El PDF permite darlas en cualquier orden, así que [`MediaBox::new`]
/// normaliza. Lo que importa de ella aquí son las **esquinas superiores**
/// ([`MediaBox::upper_x`] y [`MediaBox::upper_y`]): son las que entran en `T`,
/// y confundirlas con la anchura y la altura es el fallo que este módulo
/// existe para no cometer.
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

    /// Esquina superior derecha, eje X. `mx1` en la tabla de `T`.
    pub fn upper_x(&self) -> f64 {
        self.upper_x
    }

    /// Esquina superior derecha, eje Y. `my1` en la tabla de `T`.
    pub fn upper_y(&self) -> f64 {
        self.upper_y
    }
}

/// La página sobre la que se ha arrastrado el recuadro.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Page {
    /// Número de página **1-based**, tal cual lo numera `pdf.js`: sin
    /// corrección, porque `signaturePage` usa el mismo criterio.
    pub number: u32,
    /// La MediaBox de esa página.
    pub media_box: MediaBox,
    /// Su `/Rotate`.
    pub rotation: Rotation,
}

/// El recuadro tal y como se arrastra: píxeles del lienzo del visor.
///
/// Es un dato **de paso**, no de almacenamiento: en cuanto se convierte a
/// [`UserSpaceRect`] se tira. Guardar píxeles es la trampa que hace que el
/// recuadro se mueva solo al cambiar el zoom.
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

/// El recuadro en **espacio de usuario PDF**, que es como se guarda.
///
/// Es el `/Rect` que acabará teniendo el widget de firma, ya redondeado a
/// entero. No depende del zoom: los píxeles se derivan de aquí en cada pintada,
/// nunca al revés.
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

impl Page {
    /// Los dos pasos de una vez: del arrastre del visor a los puntos PAdES.
    ///
    /// `zoom` es la escala del viewport de `pdf.js` con la que se dibujó el
    /// arrastre, y tiene que ser mayor que cero.
    pub fn place(&self, rect: &ViewerRect, zoom: f64) -> Result<SignatureBox, OutOfPage> {
        self.signature_box(&self.to_user_space(rect, zoom))
    }

    /// Paso 1: lienzo → espacio de usuario PDF.
    ///
    /// Réplica de `convertToPdfPoint` de `pdf.js`: invierte la matriz del
    /// viewport que construye `PageViewport`. Por eso el zoom desaparece aquí y
    /// no vuelve a aparecer.
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

    /// Paso 2: espacio de usuario → puntos PAdES, con la guardia del ID-22.
    ///
    /// Un recuadro que se saliera de la página **iText lo recortaría en
    /// silencio** y la firma saldría válida igual, con la rúbrica de 13 pt de
    /// ancho en vez de los 200 que se dibujaron. Aquí se rechaza antes de
    /// firmar.
    pub fn signature_box(&self, rect: &UserSpaceRect) -> Result<SignatureBox, OutOfPage> {
        self.check_fits(rect)?;
        let (ax, ay) = self.inverse_itext(rect.lower_left_x, rect.lower_left_y);
        let (bx, by) = self.inverse_itext(rect.upper_right_x, rect.upper_right_y);
        Ok(SignatureBox {
            page: self.number,
            lower_left_x: ax.min(bx),
            lower_left_y: ay.min(by),
            upper_right_x: ax.max(bx),
            upper_right_y: ay.max(by),
        })
    }

    /// `T⁻¹` sobre un punto, según la `/Rotate`. La tabla vive en
    /// `docs/research/coordenadas-recuadro-pades.md`.
    fn inverse_itext(&self, x: i32, y: i32) -> (i32, i32) {
        // Los límites superiores de la MediaBox, NO la anchura ni la altura.
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

    /// La matriz del viewport de `pdf.js`, tal y como la construye
    /// `PageViewport`: `[a, b, c, d, e, f]`.
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
        // En los cuartos de vuelta el lienzo intercambia los ejes, así que los
        // desplazamientos también se intercambian.
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

/// `applyInverseTransform` de `pdf.js`.
fn invert(m: [f64; 6], x: f64, y: f64) -> (f64, f64) {
    let determinant = m[0] * m[3] - m[1] * m[2];
    let dx = x - m[4];
    let dy = y - m[5];
    (
        (m[3] * dx - m[2] * dy) / determinant,
        (m[0] * dy - m[1] * dx) / determinant,
    )
}

/// Redondeo al entero más cercano, con el medio punto hacia arriba: el mismo
/// criterio que `Math.round` de JavaScript, que es quien redondeaba en la
/// medición del banco de pruebas.
fn round(value: f64) -> i32 {
    (value + 0.5).floor() as i32
}

/// El recuadro no cabe en la página (ID-22).
///
/// Lleva el límite dentro a propósito: «no cabe» sin decir dónde acaba la
/// página obliga a quien lo lee a ir a buscarlo.
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

#[cfg(test)]
mod tests {
    use super::{MediaBox, OutOfPage, Page, Rotation, UserSpaceRect, ViewerRect};

    /// El arrastre de pantalla con el que se midieron los dieciséis casos.
    const DRAG: ViewerRect = ViewerRect {
        x0: 60.0,
        y0: 80.0,
        x1: 260.0,
        y1: 160.0,
    };

    const A4: [f64; 4] = [0.0, 0.0, 595.0, 842.0];
    const A5: [f64; 4] = [0.0, 0.0, 420.0, 595.0];
    const LETTER: [f64; 4] = [0.0, 0.0, 612.0, 792.0];
    /// La MediaBox desplazada: la que destapa el uso de la anchura en vez del
    /// límite superior.
    const OFFSET: [f64; 4] = [20.0, 30.0, 615.0, 872.0];

    /// Un caso del banco de `research/9-coordenadas-pdfjs`: lo que se arrastró
    /// y lo que salió medido del `/Rect` del PDF firmado de verdad.
    struct Case {
        name: &'static str,
        page: u32,
        media_box: [f64; 4],
        rotate: i32,
        zoom: f64,
        /// El `/Rect` del widget, o sea el paso 1.
        widget: [i32; 4],
        /// Los `signaturePositionOnPage*`, o sea el paso 2.
        params: [i32; 4],
    }

    /// Los dieciséis casos de `prototipos/9-coordenadas-pdfjs/salidas/*.properties`.
    ///
    /// Todos con el mismo arrastre, [`DRAG`]. No son aritmética sobre el papel:
    /// cada uno se firmó con el ciclo trifásico entero y se comprobó leyendo el
    /// `/Rect` del widget del PDF resultante.
    // La tabla se lee en columnas: expandida a un campo por línea son ciento
    // cuarenta y cuatro líneas y deja de verse que los casos se emparejan.
    #[rustfmt::skip]
    const BANK: [Case; 16] = [
        Case { name: "a4",                 page: 1, media_box: A4,     rotate: 0,   zoom: 1.0,  widget: [60, 682, 260, 762],  params: [60, 682, 260, 762] },
        Case { name: "a4-rot90",           page: 1, media_box: A4,     rotate: 90,  zoom: 1.0,  widget: [80, 60, 160, 260],   params: [60, 435, 260, 515] },
        Case { name: "a4-rot180",          page: 1, media_box: A4,     rotate: 180, zoom: 1.0,  widget: [335, 80, 535, 160],  params: [60, 682, 260, 762] },
        Case { name: "a4-rot270",          page: 1, media_box: A4,     rotate: 270, zoom: 1.0,  widget: [435, 582, 515, 782], params: [60, 435, 260, 515] },
        Case { name: "a5",                 page: 1, media_box: A5,     rotate: 0,   zoom: 1.0,  widget: [60, 435, 260, 515],  params: [60, 435, 260, 515] },
        Case { name: "letter",             page: 1, media_box: LETTER, rotate: 0,   zoom: 1.0,  widget: [60, 632, 260, 712],  params: [60, 632, 260, 712] },
        Case { name: "offset",             page: 1, media_box: OFFSET, rotate: 0,   zoom: 1.0,  widget: [80, 712, 280, 792],  params: [80, 712, 280, 792] },
        Case { name: "offset-rot90",       page: 1, media_box: OFFSET, rotate: 90,  zoom: 1.0,  widget: [100, 90, 180, 290],  params: [90, 435, 290, 515] },
        Case { name: "offset-rot180",      page: 1, media_box: OFFSET, rotate: 180, zoom: 1.0,  widget: [355, 110, 555, 190], params: [60, 682, 260, 762] },
        Case { name: "offset-rot270",      page: 1, media_box: OFFSET, rotate: 270, zoom: 1.0,  widget: [455, 612, 535, 812], params: [60, 455, 260, 535] },
        Case { name: "mixto-p1",           page: 1, media_box: A4,     rotate: 0,   zoom: 1.0,  widget: [60, 682, 260, 762],  params: [60, 682, 260, 762] },
        Case { name: "mixto-p2",           page: 2, media_box: A5,     rotate: 90,  zoom: 1.0,  widget: [80, 60, 160, 260],   params: [60, 260, 260, 340] },
        Case { name: "mixto-p3",           page: 3, media_box: OFFSET, rotate: 180, zoom: 1.0,  widget: [355, 110, 555, 190], params: [60, 682, 260, 762] },
        Case { name: "a4-zoom175",         page: 1, media_box: A4,     rotate: 0,   zoom: 1.75, widget: [34, 751, 149, 796],  params: [34, 751, 149, 796] },
        Case { name: "a4rot90-zoom06",     page: 1, media_box: A4,     rotate: 90,  zoom: 0.6,  widget: [133, 100, 267, 433], params: [100, 328, 433, 462] },
        Case { name: "offrot270-zoom175",  page: 1, media_box: OFFSET, rotate: 270, zoom: 1.75, widget: [524, 723, 569, 838], params: [34, 524, 149, 569] },
    ];

    fn page_of(case: &Case) -> Page {
        let [x0, y0, x1, y1] = case.media_box;
        Page {
            number: case.page,
            media_box: MediaBox::new(x0, y0, x1, y1),
            rotation: Rotation::from_degrees(case.rotate).expect("rotación del banco"),
        }
    }

    fn rect(values: [i32; 4]) -> UserSpaceRect {
        UserSpaceRect {
            lower_left_x: values[0],
            lower_left_y: values[1],
            upper_right_x: values[2],
            upper_right_y: values[3],
        }
    }

    #[test]
    fn converts_the_drag_to_user_space_like_pdfjs_does() {
        for case in &BANK {
            assert_eq!(
                page_of(case).to_user_space(&DRAG, case.zoom),
                rect(case.widget),
                "paso 1 del caso «{}»",
                case.name
            );
        }
    }

    #[test]
    fn matches_every_measured_case_of_the_bank() {
        for case in &BANK {
            let placed = page_of(case)
                .place(&DRAG, case.zoom)
                .unwrap_or_else(|error| panic!("caso «{}»: {error}", case.name));
            assert_eq!(
                [
                    placed.lower_left_x,
                    placed.lower_left_y,
                    placed.upper_right_x,
                    placed.upper_right_y
                ],
                case.params,
                "paso 2 del caso «{}»",
                case.name
            );
            assert_eq!(placed.page, case.page, "página del caso «{}»", case.name);
        }
    }

    #[test]
    fn covers_the_four_rotations_with_a_displaced_media_box() {
        // El criterio que separa la implementación correcta de la que acierta
        // por casualidad: /Rotate distinto de 0 Y MediaBox fuera del origen a
        // la vez. Si alguien adelgaza el banco, esta prueba se queja.
        for degrees in [90, 180, 270] {
            assert!(
                BANK.iter().any(|case| case.media_box == OFFSET
                    && case.rotate == degrees
                    && case.media_box[0] != 0.0),
                "el banco se ha quedado sin el caso de MediaBox desplazada a {degrees}°"
            );
        }
    }

    #[test]
    fn uses_the_upper_bounds_of_the_media_box_and_not_the_width() {
        // La misma página en tamaño y rotación, movida de sitio. Si el paso 2
        // usara la anchura (595 en los dos casos) en vez del límite superior,
        // los dos darían lo mismo.
        let at_origin = Page {
            number: 1,
            media_box: MediaBox::new(0.0, 0.0, 595.0, 842.0),
            rotation: Rotation::Quarter,
        };
        let displaced = Page {
            media_box: MediaBox::new(20.0, 30.0, 615.0, 872.0),
            ..at_origin
        };
        let same_rect = rect([100, 200, 300, 260]);

        assert_ne!(
            at_origin.signature_box(&same_rect).expect("cabe"),
            displaced.signature_box(&same_rect).expect("cabe"),
        );
    }

    #[test]
    fn emits_integer_coordinates() {
        // El arrastre cae en medio punto por todas partes; lo que sale son
        // enteros porque `SignatureBox` no sabe guardar otra cosa.
        let page = Page {
            number: 1,
            media_box: MediaBox::new(0.0, 0.0, 595.0, 842.0),
            rotation: Rotation::None,
        };
        let placed = page
            .place(
                &ViewerRect {
                    x0: 60.4,
                    y0: 80.7,
                    x1: 260.2,
                    y1: 160.9,
                },
                1.0,
            )
            .expect("cabe");
        assert_eq!(
            [
                placed.lower_left_x,
                placed.lower_left_y,
                placed.upper_right_x,
                placed.upper_right_y
            ],
            [60, 681, 260, 761]
        );
    }

    #[test]
    fn keeps_the_box_still_when_the_zoom_changes() {
        // El mismo sitio del documento, dibujado a dos escalas: en píxeles el
        // arrastre es otro, en espacio de usuario es el mismo. Esto es lo que
        // se rompe si el recuadro se guarda en píxeles.
        let page = Page {
            number: 1,
            media_box: MediaBox::new(20.0, 30.0, 615.0, 872.0),
            rotation: Rotation::ThreeQuarters,
        };
        let at_one = page.to_user_space(&DRAG, 1.0);
        for zoom in [0.5, 1.75, 3.0] {
            let scaled = ViewerRect {
                x0: DRAG.x0 * zoom,
                y0: DRAG.y0 * zoom,
                x1: DRAG.x1 * zoom,
                y1: DRAG.y1 * zoom,
            };
            assert_eq!(
                page.to_user_space(&scaled, zoom),
                at_one,
                "el zoom {zoom} ha movido el recuadro"
            );
        }
    }

    #[test]
    fn rejects_a_box_that_would_fall_off_the_page() {
        let page = Page {
            number: 4,
            media_box: MediaBox::new(20.0, 30.0, 615.0, 872.0),
            rotation: Rotation::None,
        };
        let error = page
            .signature_box(&rect([500, 700, 700, 780]))
            .expect_err("un recuadro que se sale no puede firmarse");
        assert_eq!(
            error,
            OutOfPage {
                page: 4,
                rect: [500, 700, 700, 780],
                media_box: [20, 30, 615, 872],
            }
        );
    }

    #[test]
    fn says_which_limit_the_box_crossed() {
        let page = Page {
            number: 1,
            media_box: MediaBox::new(20.0, 30.0, 615.0, 872.0),
            rotation: Rotation::None,
        };
        let message = page
            .signature_box(&rect([500, 700, 700, 780]))
            .expect_err("se sale")
            .to_string();
        assert!(message.contains("615"), "no dice el límite: {message}");
        assert!(message.contains("872"), "no dice el límite: {message}");
    }

    #[test]
    fn rejects_a_box_that_falls_short_of_a_displaced_origin() {
        // El otro lado de la guardia: con la MediaBox desplazada, la página no
        // empieza en (0,0) y un recuadro «dentro del papel» puede quedarse
        // fuera por abajo.
        let page = Page {
            number: 1,
            media_box: MediaBox::new(20.0, 30.0, 615.0, 872.0),
            rotation: Rotation::None,
        };
        assert!(page.signature_box(&rect([5, 10, 200, 100])).is_err());
    }

    #[test]
    fn accepts_a_box_that_touches_the_edge() {
        let page = Page {
            number: 1,
            media_box: MediaBox::new(20.0, 30.0, 615.0, 872.0),
            rotation: Rotation::None,
        };
        assert!(page.signature_box(&rect([20, 30, 615, 872])).is_ok());
    }

    #[test]
    fn normalises_the_rotation_the_pdf_declares() {
        assert_eq!(Rotation::from_degrees(0), Some(Rotation::None));
        assert_eq!(Rotation::from_degrees(360), Some(Rotation::None));
        assert_eq!(Rotation::from_degrees(-90), Some(Rotation::ThreeQuarters));
        assert_eq!(Rotation::from_degrees(450), Some(Rotation::Quarter));
        assert_eq!(Rotation::from_degrees(45), None);
        assert_eq!(Rotation::ThreeQuarters.degrees(), 270);
    }

    #[test]
    fn orders_the_corners_of_the_media_box() {
        let media_box = MediaBox::new(615.0, 872.0, 20.0, 30.0);
        assert_eq!(media_box.lower_x(), 20.0);
        assert_eq!(media_box.lower_y(), 30.0);
        assert_eq!(media_box.upper_x(), 615.0);
        assert_eq!(media_box.upper_y(), 872.0);
    }

    #[test]
    fn normalises_a_drag_made_in_any_direction() {
        // Se arrastra tan a menudo de abajo a la derecha hacia arriba a la
        // izquierda como al revés; el recuadro es el mismo.
        let page = Page {
            number: 1,
            media_box: MediaBox::new(0.0, 0.0, 595.0, 842.0),
            rotation: Rotation::Half,
        };
        let backwards = ViewerRect {
            x0: DRAG.x1,
            y0: DRAG.y1,
            x1: DRAG.x0,
            y1: DRAG.y0,
        };
        assert_eq!(
            page.to_user_space(&backwards, 1.0),
            page.to_user_space(&DRAG, 1.0)
        );
    }
}
