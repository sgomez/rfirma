//! Deserialización y validación de las órdenes de la ventana.

use serde::Deserialize;

use crate::crossing::crossing;

use crate::signing::application::configuration::language_of;
use crate::signing::domain::{
    Datum, MediaBox, Page, PageSet, PhrasePart, Placement, PlacementError, Rotation, SigningChoice,
    UserSpaceRect, VisibleContent,
};

crossing! {
    /// Un dato de la frase de *Personalizada*.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub enum DatumOrder {
        Signer,
        Issuer,
        SignedAt,
    }
}

crossing! {
    /// Un trozo de la frase: `{ text }` o `{ datum }`, nunca un comodín `$$…$$` (ADR-0006).
    #[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
    #[serde(untagged)]
    pub enum PhrasePartOrder {
        Text { text: String },
        Datum { datum: DatumOrder },
    }
}

crossing! {
    /// El contenido de la firma visible, por modelo.
    #[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
    #[serde(tag = "model", rename_all = "camelCase")]
    pub enum VisibleContentOrder {
        Complete,
        RubricOnly,
        Custom { phrase: Vec<PhrasePartOrder> },
    }
}

impl From<&VisibleContentOrder> for VisibleContent {
    fn from(order: &VisibleContentOrder) -> Self {
        match order {
            VisibleContentOrder::Complete => Self::Complete,
            VisibleContentOrder::RubricOnly => Self::RubricOnly,
            VisibleContentOrder::Custom { phrase } => {
                Self::Custom(phrase.iter().map(PhrasePart::from).collect())
            }
        }
    }
}

impl From<&PhrasePartOrder> for PhrasePart {
    fn from(order: &PhrasePartOrder) -> Self {
        match order {
            PhrasePartOrder::Text { text } => Self::Text(text.clone()),
            PhrasePartOrder::Datum { datum } => Self::Datum(match datum {
                DatumOrder::Signer => Datum::Signer,
                DatumOrder::Issuer => Datum::Issuer,
                DatumOrder::SignedAt => Datum::SignedAt,
            }),
        }
    }
}

crossing! {
    /// Dónde ha caído el recuadro, tal como lo sabe el visor.
    #[derive(Clone, Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PlacementOrder {
        /// La página sobre la que se arrastró, 1-based como la numera `pdf.js`.
        pub page: u32,
        /// En qué páginas se estampa.
        pub pages: PageSet,
        /// Cuántas páginas tiene el documento, según el visor.
        pub page_count: u32,
        /// La `MediaBox` de la página del arrastre: `[x0, y0, x1, y1]`.
        pub media_box: [f64; 4],
        /// Su `/Rotate`, en grados.
        pub rotation: i32,
        /// El recuadro en espacio de usuario: `[x0, y0, x1, y1]`.
        pub rect: [f64; 4],
    }
}

impl PlacementOrder {
    /// La colocación en puntos PAdES, o la negativa si el destino no existe o el recuadro se sale de la página.
    pub fn placement(&self) -> Result<Placement, PlacementError> {
        self.pages.validate(self.page_count)?;
        PageSet::only_page(self.page).validate(self.page_count)?;

        let [x0, y0, x1, y1] = self.media_box;
        let rotation = Rotation::from_degrees(self.rotation)
            .ok_or(PlacementError::BadRotation(self.rotation))?;
        let page = Page {
            number: self.page,
            media_box: MediaBox::new(x0, y0, x1, y1),
            rotation,
        };
        let [left, bottom, right, top] = self.rect;
        let rect = page.pades_rect(&UserSpaceRect::rounded(left, bottom, right, top))?;
        Ok(Placement {
            rect,
            pages: self.pages.clone(),
        })
    }
}

crossing! {
    /// La orden de firma completa: todo lo que distingue esta firma de otra.
    #[derive(Clone, Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SigningOrder {
        /// El asa que dio el portal al abrir el documento.
        pub document: String,
        /// El asa del certificado elegido.
        pub certificate: String,
        /// Dónde cae el recuadro, en espacio de usuario PDF.
        pub placement: Option<PlacementOrder>,
        /// El contenido del recuadro, por modelo.
        pub content: VisibleContentOrder,
        /// Si la firma visible lleva la rúbrica; común a los tres modelos.
        #[serde(default)]
        pub with_rubric: bool,
        /// La fecha y hora, ya formateadas.
        pub signed_at: String,
        /// La rúbrica en JPEG y Base64, ya normalizada.
        pub rubric: Option<String>,
        /// El idioma en el que se componen las etiquetas del recuadro.
        pub language: String,
        /// Si la persona ha consentido cofirmar un PDF con firmas no reconocidas.
        #[serde(default)]
        pub allow_unregistered_signatures: bool,
    }
}

impl SigningOrder {
    /// Lo decidido en la orden, con el recuadro ya validado; las asas se resuelven aparte.
    pub fn choice(&self) -> Result<SigningChoice, PlacementError> {
        let content = VisibleContent::from(&self.content);
        Ok(SigningChoice {
            placement: self
                .placement
                .as_ref()
                .map(|placement| placement.placement())
                .transpose()?,
            rubric: self
                .rubric
                .clone()
                .filter(|_| content.carries_the_rubric(self.with_rubric)),
            content,
            signed_at: self.signed_at.clone(),
            language: language_of(&self.language),
            allow_unregistered_signatures: self.allow_unregistered_signatures,
        })
    }
}

#[cfg(test)]
mod tests;
