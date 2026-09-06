use super::*;
use image::{Rgba, RgbaImage};

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
    assert_eq!(over_white([0, 0, 0], 128), [127, 127, 127]);
    assert_eq!(over_white([64, 128, 192], 51), [217, 230, 242]);
}

#[test]
fn a_file_that_is_neither_png_nor_jpeg_says_which_formats_are_accepted() {
    let error = normalize(b"GIF89a and then some bytes").expect_err("un GIF deberia rechazarse");

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
    let error = normalize(&vec![0_u8; MAX_INPUT_BYTES + 1]).expect_err("deberia pasar del tope");

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

fn jpeg_rotated_by_exif(width: u32, height: u32) -> Vec<u8> {
    let plain = normalize(&png(width, height, Rgba([10, 20, 30, 255])))
        .expect("el JPEG de prueba deberia normalizarse")
        .jpeg;

    let mut tiff = b"MM\x00\x2A\x00\x00\x00\x08".to_vec();
    tiff.extend_from_slice(&1_u16.to_be_bytes());
    tiff.extend_from_slice(&0x0112_u16.to_be_bytes());
    tiff.extend_from_slice(&3_u16.to_be_bytes());
    tiff.extend_from_slice(&1_u32.to_be_bytes());
    tiff.extend_from_slice(&6_u16.to_be_bytes());
    tiff.extend_from_slice(&0_u16.to_be_bytes());
    tiff.extend_from_slice(&0_u32.to_be_bytes());

    let mut payload = b"Exif\0\0".to_vec();
    payload.extend_from_slice(&tiff);
    let length = u16::try_from(payload.len() + 2).expect("el EXIF de prueba es pequeno");

    let mut with_exif = vec![0xFF, 0xD8, 0xFF, 0xE1];
    with_exif.extend_from_slice(&length.to_be_bytes());
    with_exif.extend_from_slice(&payload);
    with_exif.extend_from_slice(&plain[2..]);
    with_exif
}

#[test]
fn a_photographed_rubric_comes_out_upright_and_not_sideways() {
    let sideways = jpeg_rotated_by_exif(40, 10);

    let normalized = normalize(&sideways).expect("un JPEG con EXIF deberia normalizarse");
    let decoded =
        image::load_from_memory(normalized.bytes()).expect("la salida deberia decodificarse");

    assert_eq!((decoded.width(), decoded.height()), (10, 40));
}

fn png_declaring(width: u32, height: u32) -> Vec<u8> {
    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFF_u32;
        for byte in bytes {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                let carry = crc & 1;
                crc >>= 1;
                if carry != 0 {
                    crc ^= 0xEDB8_8320;
                }
            }
        }
        !crc
    }
    fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut typed = kind.to_vec();
        typed.extend_from_slice(data);
        let mut out = (u32::try_from(data.len()).expect("cabe"))
            .to_be_bytes()
            .to_vec();
        out.extend_from_slice(&typed);
        out.extend_from_slice(&crc32(&typed).to_be_bytes());
        out
    }

    let mut header = width.to_be_bytes().to_vec();
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);

    let idat = [
        0x78, 0x01, 0x01, 0x01, 0x00, 0xFE, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x01,
    ];

    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    png.extend_from_slice(&chunk(b"IHDR", &header));
    png.extend_from_slice(&chunk(b"IDAT", &idat));
    png.extend_from_slice(&chunk(b"IEND", &[]));
    png
}

#[test]
fn a_small_file_that_decompresses_to_gigabytes_is_rejected_before_reserving_them() {
    let bomb = png_declaring(30_000, 30_000);
    assert!(
        bomb.len() < MAX_INPUT_BYTES,
        "la bomba de la prueba deberia pasar el tope de entrada"
    );

    let error = normalize(&bomb).expect_err("una bomba de descompresion deberia rechazarse");

    assert_eq!(error.situation(), Situation::ImageTooLarge);
    assert!(error.detail().contains("descomprimida"));
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
