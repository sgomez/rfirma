//! Normaliza una imagen de rubrica igual que hacia es.gob.afirma.ui.utils.ImageUtils:
//! aplana el canal alfa sobre blanco y reencoda a JPEG. Anade el reescalado y
//! el tope de tamano que fija el ADR-0012.
//!
//! uso: rubrica <entrada.png|jpg> <salida.jpg>

use image::codecs::jpeg::JpegEncoder;
use image::{ImageReader, Rgb, RgbImage};

/// Constantes del ADR-0012.
const CALIDAD: u8 = 90;
const LADO_MAXIMO: u32 = 1000;
const TOPE_ENTRADA: u64 = 10 * 1024 * 1024;
const FONDO: Rgb<u8> = Rgb([255, 255, 255]);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let (entrada, salida) = (&args[1], &args[2]);

    if std::fs::metadata(entrada)?.len() > TOPE_ENTRADA {
        return Err("la imagen es demasiado grande".into());
    }

    // Solo PNG y JPEG: el formato se decide por contenido, no por extension.
    let lector = ImageReader::open(entrada)?.with_guessed_format()?;
    match lector.format() {
        Some(image::ImageFormat::Png) | Some(image::ImageFormat::Jpeg) => {}
        _ => return Err("no es una imagen PNG o JPEG".into()),
    }
    let mut img = lector.decode()?;

    let mayor = img.width().max(img.height());
    if mayor > LADO_MAXIMO {
        let f = LADO_MAXIMO as f32 / mayor as f32;
        img = img.resize(
            (img.width() as f32 * f) as u32,
            (img.height() as f32 * f) as u32,
            image::imageops::FilterType::Lanczos3,
        );
    }

    // Aplanado del alfa sobre blanco, que es lo que produce de hecho el
    // removeAlphaChannel del original (su g.setColor esta comentado y
    // Graphics2D arranca en blanco).
    let rgba = img.to_rgba8();
    let mut plano = RgbImage::from_pixel(rgba.width(), rgba.height(), FONDO);
    for (x, y, p) in rgba.enumerate_pixels() {
        let a = p[3] as f32 / 255.0;
        plano.put_pixel(
            x,
            y,
            Rgb([
                (p[0] as f32 * a + 255.0 * (1.0 - a)) as u8,
                (p[1] as f32 * a + 255.0 * (1.0 - a)) as u8,
                (p[2] as f32 * a + 255.0 * (1.0 - a)) as u8,
            ]),
        );
    }

    let mut fichero = std::fs::File::create(salida)?;
    JpegEncoder::new_with_quality(&mut fichero, CALIDAD).encode_image(&plano)?;
    println!("{} -> {} ({}x{})", entrada, salida, plano.width(), plano.height());
    Ok(())
}
