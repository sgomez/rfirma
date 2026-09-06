//! Normalización de imágenes de rúbrica a JPEG opaco sin perfil ICC (ADR-0012).

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{ExtendedColorType, ImageDecoder as _, ImageEncoder, ImageFormat, ImageReader};
use std::io::Cursor;

pub use crate::documents::domain::rubric::{NormalizedRubric, RubricError, Situation};

/// Calidad de compresión del JPEG de salida (ADR-0012).
pub const JPEG_QUALITY: u8 = 90;

/// Lado mayor máximo en píxeles antes de reescalar (ADR-0012).
pub const MAX_SIDE_PX: u32 = 1000;

/// Tamaño máximo permitido para el fichero de entrada en bytes (ADR-0012).
pub const MAX_INPUT_BYTES: usize = 10 * 1024 * 1024;

/// Límite máximo de memoria para la imagen descomprimida en bytes.
pub const MAX_DECODED_BYTES: u64 = 512 * 1024 * 1024;

/// Formatos de imagen de entrada admitidos para rúbricas (ADR-0012).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcceptedFormat {
    /// Formato PNG.
    Png,
    /// Formato JPEG.
    Jpeg,
}

impl AcceptedFormat {
    /// Tipo MIME correspondiente al formato.
    pub fn mime(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
        }
    }

    fn of(format: ImageFormat) -> Option<Self> {
        match format {
            ImageFormat::Png => Some(Self::Png),
            ImageFormat::Jpeg => Some(Self::Jpeg),
            _ => None,
        }
    }

    fn as_image_format(self) -> ImageFormat {
        match self {
            Self::Png => ImageFormat::Png,
            Self::Jpeg => ImageFormat::Jpeg,
        }
    }
}

/// Formatos admitidos por el decodificador disponible en tiempo de ejecución.
pub fn accepted_formats() -> Vec<AcceptedFormat> {
    [AcceptedFormat::Png, AcceptedFormat::Jpeg]
        .into_iter()
        .filter(|format| {
            image::ImageFormat::from_mime_type(format.mime())
                .is_some_and(|format| format.reading_enabled())
        })
        .collect()
}

fn accepted_formats_label() -> String {
    accepted_formats()
        .iter()
        .map(|format| format.mime())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Normaliza los bytes de una imagen a JPEG opaco sin perfil ICC (ADR-0012).
pub fn normalize(bytes: &[u8]) -> Result<NormalizedRubric, RubricError> {
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(RubricError::new(
            Situation::ImageTooLarge,
            format!("{} bytes, el tope son {MAX_INPUT_BYTES}", bytes.len()),
        ));
    }

    let format = image::guess_format(bytes)
        .ok()
        .and_then(AcceptedFormat::of)
        .filter(|format| accepted_formats().contains(format))
        .ok_or_else(|| {
            RubricError::new(
                Situation::NotAnAcceptedImage,
                format!(
                    "no es PNG ni JPEG; formatos admitidos: {}",
                    accepted_formats_label()
                ),
            )
        })?;

    let image = decode_upright(bytes, format)?;
    let opaque = downscale(flatten_onto_white(&image));

    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut Cursor::new(&mut jpeg), JPEG_QUALITY)
        .write_image(
            opaque.as_raw(),
            opaque.width(),
            opaque.height(),
            ExtendedColorType::Rgb8,
        )
        .map_err(|error| RubricError::new(Situation::DamagedImage, error.to_string()))?;

    Ok(NormalizedRubric { jpeg })
}

fn decode_upright(
    bytes: &[u8],
    format: AcceptedFormat,
) -> Result<image::DynamicImage, RubricError> {
    let damaged =
        |error: image::ImageError| RubricError::new(Situation::DamagedImage, error.to_string());

    let mut decoder = ImageReader::with_format(Cursor::new(bytes), format.as_image_format())
        .into_decoder()
        .map_err(damaged)?;
    let orientation = decoder.orientation().map_err(damaged)?;

    let decoded_bytes = decoder.total_bytes();
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(MAX_DECODED_BYTES);
    limits.reserve(decoded_bytes).map_err(|_| {
        RubricError::new(
            Situation::ImageTooLarge,
            format!("{decoded_bytes} bytes ya descomprimida, el tope son {MAX_DECODED_BYTES}"),
        )
    })?;

    decoder.set_limits(limits).map_err(damaged)?;

    let mut image = image::DynamicImage::from_decoder(decoder).map_err(damaged)?;
    image.apply_orientation(orientation);
    Ok(image)
}

fn downscale(image: image::RgbImage) -> image::RgbImage {
    let longest = image.width().max(image.height());
    if longest <= MAX_SIDE_PX {
        return image;
    }
    let ratio = f64::from(MAX_SIDE_PX) / f64::from(longest);
    let scale = |side: u32| ((f64::from(side) * ratio).round() as u32).max(1);
    image::imageops::resize(
        &image,
        scale(image.width()),
        scale(image.height()),
        FilterType::Lanczos3,
    )
}

fn flatten_onto_white(image: &image::DynamicImage) -> image::RgbImage {
    let source = image.to_rgba8();
    let mut opaque = image::RgbImage::new(source.width(), source.height());
    for (x, y, pixel) in source.enumerate_pixels() {
        let [red, green, blue, alpha] = pixel.0;
        opaque.put_pixel(x, y, image::Rgb(over_white([red, green, blue], alpha)));
    }
    opaque
}

fn over_white(channels: [u8; 3], alpha: u8) -> [u8; 3] {
    let alpha = u32::from(alpha);
    channels.map(|channel| {
        let blended = u32::from(channel) * alpha + 255 * (255 - alpha);
        ((blended + 127) / 255) as u8
    })
}

#[cfg(test)]
mod tests;
