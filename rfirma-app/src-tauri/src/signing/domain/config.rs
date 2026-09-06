//! Configuración de firma PAdES para el puente nativo.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::placement::PageSet;

/// Subfiltro de la firma.
pub const SUB_FILTER: &str = "ETSI.CAdES.detached";

const SUB_FILTER_KEY: &str = "signatureSubFilter";
const PAGES_KEY: &str = "signaturePages";
const LOWER_LEFT_X_KEY: &str = "signaturePositionOnPageLowerLeftX";
const LOWER_LEFT_Y_KEY: &str = "signaturePositionOnPageLowerLeftY";
const UPPER_RIGHT_X_KEY: &str = "signaturePositionOnPageUpperRightX";
const UPPER_RIGHT_Y_KEY: &str = "signaturePositionOnPageUpperRightY";
const LAYER2_TEXT_KEY: &str = "layer2Text";
const RUBRIC_IMAGE_KEY: &str = "signatureRubricImage";
const SIGN_REASON_KEY: &str = "signReason";
const LAYER2_FONT_SIZE_KEY: &str = "layer2FontSize";
/// Clave para autorizar la cofirma de firmas no registradas en el puente.
pub const ALLOW_UNREGISTERED_KEY: &str = "allowCosigningUnregisteredSignatures";

/// Tamaño de letra cero para cálculo proporcional por alto de línea.
const LAYER2_FONT_SIZE: &str = "0";

/// Ajustes cerrados de la configuración de firma.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Setting {
    /// El subfiltro, siempre [`SUB_FILTER`].
    SubFilter,
    /// Geometría del recuadro: páginas y cuatro esquinas.
    Geometry,
    /// El texto del recuadro, ya compuesto por rFirma.
    Layer2Text,
    /// La rúbrica, si la hay.
    RubricImage,
    /// El motivo de la firma, si lo hay.
    SignReason,
    /// El tamaño de letra del recuadro, siempre [`LAYER2_FONT_SIZE`].
    Layer2FontSize,
    /// Consentimiento para cofirmar firmas no registradas.
    AllowUnregisteredSignatures,
}

impl Setting {
    /// Los siete.
    pub const ALL: [Self; 7] = [
        Self::SubFilter,
        Self::Geometry,
        Self::Layer2Text,
        Self::RubricImage,
        Self::SignReason,
        Self::Layer2FontSize,
        Self::AllowUnregisteredSignatures,
    ];

    /// Las claves de `extraParams` que emite este ajuste.
    pub fn keys(self) -> &'static [&'static str] {
        match self {
            Self::SubFilter => &[SUB_FILTER_KEY],
            Self::Geometry => &[
                PAGES_KEY,
                LOWER_LEFT_X_KEY,
                LOWER_LEFT_Y_KEY,
                UPPER_RIGHT_X_KEY,
                UPPER_RIGHT_Y_KEY,
            ],
            Self::Layer2Text => &[LAYER2_TEXT_KEY],
            Self::RubricImage => &[RUBRIC_IMAGE_KEY],
            Self::SignReason => &[SIGN_REASON_KEY],
            Self::Layer2FontSize => &[LAYER2_FONT_SIZE_KEY],
            Self::AllowUnregisteredSignatures => &[ALLOW_UNREGISTERED_KEY],
        }
    }
}

/// Rectángulo de la firma visible en puntos PAdES.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PadesRect {
    /// Esquina inferior izquierda, eje X.
    pub lower_left_x: i32,
    /// Esquina inferior izquierda, eje Y.
    pub lower_left_y: i32,
    /// Esquina superior derecha, eje X.
    pub upper_right_x: i32,
    /// Esquina superior derecha, eje Y.
    pub upper_right_y: i32,
}

/// Colocación del recuadro y páginas de destino.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Placement {
    /// El rectángulo, el mismo en todas las páginas del conjunto.
    pub rect: PadesRect,
    /// Las páginas en las que se estampa.
    pub pages: PageSet,
}

impl Placement {
    fn extra_params(&self) -> Vec<(String, String)> {
        let Self { rect, pages } = self;
        let PadesRect {
            lower_left_x,
            lower_left_y,
            upper_right_x,
            upper_right_y,
        } = rect;
        vec![
            (PAGES_KEY.to_owned(), pages.literal()),
            (LOWER_LEFT_X_KEY.to_owned(), lower_left_x.to_string()),
            (LOWER_LEFT_Y_KEY.to_owned(), lower_left_y.to_string()),
            (UPPER_RIGHT_X_KEY.to_owned(), upper_right_x.to_string()),
            (UPPER_RIGHT_Y_KEY.to_owned(), upper_right_y.to_string()),
        ]
    }
}

/// Lo que distingue una firma de otra a igualdad de documento y certificado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureConfig {
    /// Dónde cae el recuadro y en qué páginas cuando lo coloca rFirma.
    pub placement: Option<Placement>,
    /// El texto del recuadro, compuesto por [`super::layer2_text::compose_layer2_text`].
    pub layer2_text: String,
    /// La rúbrica en JPEG opaco y sin perfil ICC, en base64. `None` si no la hay.
    pub rubric_image: Option<String>,
    /// El motivo de la firma. `None` si no lo hay.
    pub sign_reason: Option<String>,
    /// Consentimiento para cofirmar firmas no registradas.
    pub allow_unregistered_signatures: bool,
}

impl SignatureConfig {
    /// Los `extraParams` que rFirma envía al puente.
    pub fn extra_params(&self) -> BTreeMap<String, String> {
        let Self {
            placement,
            layer2_text,
            rubric_image,
            sign_reason,
            allow_unregistered_signatures,
        } = self;

        let mut params = BTreeMap::new();
        params.insert(SUB_FILTER_KEY.to_owned(), SUB_FILTER.to_owned());
        if let Some(placement) = placement {
            params.extend(placement.extra_params());
        }
        params.insert(LAYER2_TEXT_KEY.to_owned(), layer2_text.clone());
        params.insert(LAYER2_FONT_SIZE_KEY.to_owned(), LAYER2_FONT_SIZE.to_owned());
        if let Some(image) = rubric_image {
            params.insert(RUBRIC_IMAGE_KEY.to_owned(), image.clone());
        }
        if let Some(reason) = sign_reason {
            params.insert(SIGN_REASON_KEY.to_owned(), reason.clone());
        }
        if *allow_unregistered_signatures {
            params.insert(ALLOW_UNREGISTERED_KEY.to_owned(), "true".to_owned());
        }
        params
    }
}

#[cfg(test)]
mod tests;
