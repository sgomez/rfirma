//! De lo que el usuario aporta a lo único que el puente acepta: un **JPEG
//! opaco y sin perfil ICC** (ID-23, ADR-0012).
//!
//! Aquí no se toca el disco: entran bytes y salen bytes. Leer el fichero y
//! copiarlo al almacén es cosa de [`super::store`].

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{ExtendedColorType, ImageEncoder, ImageFormat};
use std::io::Cursor;

use super::error::{RubricError, Situation};

/// Calidad del JPEG de salida (ADR-0012).
pub const JPEG_QUALITY: u8 = 90;

/// Lado mayor máximo, en píxeles (ADR-0012). Por encima se reescala, y el
/// reescalado **es silencioso**: es la operación que el usuario habría pedido.
pub const MAX_SIDE_PX: u32 = 1000;

/// Tope del fichero de entrada, 10 MB (ADR-0012). Se comprueba antes de
/// decodificar: el tope está para no darle a un decodificador un fichero
/// arbitrariamente grande, así que comprobarlo después no serviría de nada.
pub const MAX_INPUT_BYTES: usize = 10 * 1024 * 1024;

/// Un formato de entrada admitido.
///
/// Son dos y nada más (ADR-0012): ni un TIFF ni un WebP aparecen en la vida
/// real de una firma escaneada, y cada decodificador de más es superficie
/// sobre un fichero que elige el usuario.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcceptedFormat {
    /// PNG, el caso normal de una rúbrica recortada.
    Png,
    /// JPEG, el caso normal de una rúbrica escaneada.
    Jpeg,
}

impl AcceptedFormat {
    /// El tipo MIME. Es lo que filtra el selector del portal, **no** la
    /// extensión: la extensión miente (ADR-0012).
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

/// Los formatos que rFirma admite **ahora mismo**, preguntándoselo al
/// decodificador que hay montado.
///
/// Que esto sea una función y no una constante del puente es justo lo que se
/// ganó al normalizar en Rust: mientras la normalización vivía en Java, los
/// formatos había que declararlos en el comando de `native-image` —la traza de
/// una rúbrica PNG no cubre una JPEG— y quedaban congelados en tiempo de
/// construcción (ADR-0012). Un formato queda fuera de la lista si el binario
/// no trae su decodificador, y eso se sabe al arrancar, no al compilar el
/// puente.
pub fn accepted_formats() -> Vec<AcceptedFormat> {
    [AcceptedFormat::Png, AcceptedFormat::Jpeg]
        .into_iter()
        .filter(|format| {
            image::ImageFormat::from_mime_type(format.mime())
                .is_some_and(|format| format.reading_enabled())
        })
        .collect()
}

/// «image/png, image/jpeg»: la lista que lleva el mensaje de rechazo, tomada
/// de la capacidad real y no de un literal que se quedaría desfasado.
fn accepted_formats_label() -> String {
    accepted_formats()
        .iter()
        .map(|format| format.mime())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Una rúbrica ya normalizada: JPEG opaco, sin perfil ICC, lista para el
/// puente.
///
/// Solo se construye en [`normalize`], así que tener una en la mano ya
/// significa que la imagen pasó por aquí.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedRubric {
    jpeg: Vec<u8>,
}

impl NormalizedRubric {
    /// Los bytes del JPEG, que es lo que se guarda en el almacén.
    pub fn bytes(&self) -> &[u8] {
        &self.jpeg
    }

    /// Los mismos bytes en Base64, que es exactamente lo que viaja al puente
    /// en `signatureRubricImage`
    /// ([`crate::signing::SignatureConfig::rubric_image`]).
    pub fn to_base64(&self) -> String {
        BASE64.encode(&self.jpeg)
    }
}

/// Convierte un PNG o un JPEG cualquiera en el JPEG que el puente exige.
///
/// Un fichero que no sea PNG ni JPEG se rechaza aquí, con el diálogo del
/// usuario todavía abierto, y no al firmar (ADR-0010).
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

    let image = image::load_from_memory_with_format(bytes, format.as_image_format())
        .map_err(|error| RubricError::new(Situation::DamagedImage, error.to_string()))?;

    let image = downscale(image);
    let opaque = flatten_onto_white(&image);

    // El JPEG se emite pelado: `JpegEncoder` no incrusta perfil ICC si no se le
    // da uno, y no se le da. No es cosmética —`com.aowagie.text.Jpeg` parsea el
    // APP2 y construye un `java.awt.color.ICC_Profile`, o sea que un perfil
    // sRGB incrustado devolvería a AWT por la puerta de atrás, en una librería
    // nativa que ya no lleva ni `libawt.so` ni `liblcms.so` (ADR-0012).
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

/// Reescala solo si hace falta, conservando la proporción.
fn downscale(image: image::DynamicImage) -> image::DynamicImage {
    if image.width().max(image.height()) <= MAX_SIDE_PX {
        return image;
    }
    image.resize(MAX_SIDE_PX, MAX_SIDE_PX, FilterType::Lanczos3)
}

/// Aplana el canal alfa sobre **blanco**.
///
/// Esto parece un bug y no lo es: la rúbrica del PDF es siempre un JPEG opaco
/// (`new Jpeg`, no `Image.getInstance`), y el JPEG no tiene alfa, así que **la
/// transparencia no es ofrecible con ningún formato de entrada** (ID-24,
/// ADR-0012). Un PNG recortado sale con fondo blanco, que es además lo que
/// producía el original: en `removeAlphaChannel` la línea `g.setColor(...)`
/// está comentada y `Graphics2D` arranca en blanco. No se avisa con un cartel;
/// la miniatura del panel de firma enseña el resultado real, sobre blanco, y el
/// usuario ve lo que va a salir antes de firmar.
///
/// Se compone a mano en vez de con `to_rgb8` porque `to_rgb8` **descarta** el
/// alfa sin componer: un píxel transparente saldría con el color que llevara
/// debajo, casi siempre negro.
fn flatten_onto_white(image: &image::DynamicImage) -> image::RgbImage {
    let source = image.to_rgba8();
    let mut opaque = image::RgbImage::new(source.width(), source.height());
    for (x, y, pixel) in source.enumerate_pixels() {
        let [red, green, blue, alpha] = pixel.0;
        opaque.put_pixel(x, y, image::Rgb(over_white([red, green, blue], alpha)));
    }
    opaque
}

/// `canal * alfa + 255 * (1 - alfa)`, en enteros y con redondeo.
fn over_white(channels: [u8; 3], alpha: u8) -> [u8; 3] {
    let alpha = u32::from(alpha);
    channels.map(|channel| {
        let blended = u32::from(channel) * alpha + 255 * (255 - alpha);
        ((blended + 127) / 255) as u8
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    /// **Grada A**: son bytes en memoria, no hace falta ni token ni puente.
    fn png(width: u32, height: u32, pixel: Rgba<u8>) -> Vec<u8> {
        let mut image = RgbaImage::new(width, height);
        for target in image.pixels_mut() {
            *target = pixel;
        }
        let mut bytes = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .expect("el PNG de prueba deberia codificarse");
        bytes
    }

    fn jpeg_with_icc_profile() -> Vec<u8> {
        let plain = normalize(&png(8, 8, Rgba([10, 20, 30, 255])))
            .expect("un PNG opaco deberia normalizarse")
            .jpeg;

        // Un APP2 «ICC_PROFILE» metido justo detrás del SOI: es exactamente el
        // segmento que `com.aowagie.text.Jpeg` busca.
        let profile = b"ICC_PROFILE\0\x01\x01payload".to_vec();
        let length = u16::try_from(profile.len() + 2).expect("el perfil de prueba es pequeno");
        let mut with_profile = vec![0xFF, 0xD8, 0xFF, 0xE2];
        with_profile.extend_from_slice(&length.to_be_bytes());
        with_profile.extend_from_slice(&profile);
        with_profile.extend_from_slice(&plain[2..]);
        with_profile
    }

    #[test]
    fn accepts_png_and_jpeg_as_a_runtime_capability() {
        let formats = accepted_formats();

        assert_eq!(formats, vec![AcceptedFormat::Png, AcceptedFormat::Jpeg]);
        for format in formats {
            let sample = match format {
                AcceptedFormat::Png => png(4, 4, Rgba([1, 2, 3, 255])),
                AcceptedFormat::Jpeg => {
                    normalize(&png(4, 4, Rgba([1, 2, 3, 255])))
                        .expect("el JPEG de muestra deberia normalizarse")
                        .jpeg
                }
            };
            // La lista solo vale si el decodificador esta de verdad ahi.
            assert!(
                normalize(&sample).is_ok(),
                "{} figura como admitido pero no se decodifica",
                format.mime()
            );
        }
    }

    #[test]
    fn the_output_is_a_jpeg_which_is_what_the_bridge_accepts() {
        let normalized = normalize(&png(20, 10, Rgba([200, 100, 50, 255])))
            .expect("un PNG opaco deberia normalizarse");

        assert_eq!(
            image::guess_format(normalized.bytes()).ok(),
            Some(ImageFormat::Jpeg)
        );
        // Y en Base64 es literalmente el valor de `signatureRubricImage`.
        assert_eq!(
            BASE64
                .decode(normalized.to_base64())
                .expect("el Base64 deberia decodificarse"),
            normalized.bytes()
        );
    }

    #[test]
    fn the_output_carries_no_icc_profile_even_when_the_input_did() {
        let input = jpeg_with_icc_profile();
        assert!(
            input.windows(11).any(|window| window == b"ICC_PROFILE"),
            "la entrada de la prueba deberia llevar perfil ICC"
        );

        let normalized = normalize(&input).expect("un JPEG con perfil deberia normalizarse");

        assert!(
            !normalized
                .bytes()
                .windows(11)
                .any(|window| window == b"ICC_PROFILE"),
            "el perfil ICC ha sobrevivido a la normalizacion"
        );
    }

    #[test]
    fn a_transparent_png_comes_out_opaque_on_white() {
        let transparent = png(6, 6, Rgba([0, 0, 0, 0]));

        let normalized = normalize(&transparent).expect("un PNG transparente deberia normalizarse");
        let decoded = image::load_from_memory(normalized.bytes())
            .expect("la salida deberia decodificarse")
            .to_rgb8();

        for pixel in decoded.pixels() {
            for channel in pixel.0 {
                assert!(
                    channel > 245,
                    "el fondo deberia ser blanco, y hay un canal a {channel}"
                );
            }
        }
    }

    #[test]
    fn a_half_transparent_pixel_lands_between_its_colour_and_white() {
        assert_eq!(over_white([0, 0, 0], 0), [255, 255, 255]);
        assert_eq!(over_white([0, 0, 0], 255), [0, 0, 0]);
        // 127/255 de blanco: alfa 128 sobre negro da 127, no 128, porque el
        // blanco pesa 255 - 128 = 127 partes de 255.
        assert_eq!(over_white([0, 0, 0], 128), [127, 127, 127]);
        assert_eq!(over_white([64, 128, 192], 51), [217, 230, 242]);
    }

    #[test]
    fn a_file_that_is_neither_png_nor_jpeg_says_which_formats_are_accepted() {
        let error =
            normalize(b"GIF89a and then some bytes").expect_err("un GIF deberia rechazarse");

        assert_eq!(error.situation(), Situation::NotAnAcceptedImage);
        assert!(error.detail().contains("image/png"));
        assert!(error.detail().contains("image/jpeg"));
    }

    #[test]
    fn a_png_that_is_broken_is_damaged_and_not_an_unknown_format() {
        let mut broken = png(4, 4, Rgba([1, 2, 3, 255]));
        broken.truncate(30);

        let error = normalize(&broken).expect_err("un PNG truncado deberia rechazarse");

        assert_eq!(error.situation(), Situation::DamagedImage);
        assert!(!error.detail().is_empty());
    }

    #[test]
    fn a_file_over_the_input_cap_is_rejected_before_decoding() {
        let error =
            normalize(&vec![0_u8; MAX_INPUT_BYTES + 1]).expect_err("deberia pasar del tope");

        assert_eq!(error.situation(), Situation::ImageTooLarge);
    }

    #[test]
    fn an_oversized_image_is_scaled_down_in_silence_keeping_its_proportions() {
        let wide = png(MAX_SIDE_PX * 2, MAX_SIDE_PX / 2, Rgba([9, 9, 9, 255]));

        let normalized = normalize(&wide).expect("una imagen grande deberia normalizarse");
        let decoded =
            image::load_from_memory(normalized.bytes()).expect("la salida deberia decodificarse");

        assert_eq!(decoded.width(), MAX_SIDE_PX);
        assert_eq!(decoded.height(), MAX_SIDE_PX / 4);
    }

    #[test]
    fn an_image_within_the_limit_keeps_its_size() {
        let small = png(120, 40, Rgba([9, 9, 9, 255]));

        let decoded = image::load_from_memory(
            normalize(&small)
                .expect("una imagen pequena deberia normalizarse")
                .bytes(),
        )
        .expect("la salida deberia decodificarse");

        assert_eq!((decoded.width(), decoded.height()), (120, 40));
    }
}
