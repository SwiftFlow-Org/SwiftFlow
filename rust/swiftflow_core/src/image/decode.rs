pub struct DecodedImage {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub fn decode(bytes: &[u8]) -> Option<DecodedImage> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        decode_png(bytes)
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        decode_jpeg(bytes)
    } else {
        None
    }
}

fn decode_png(bytes: &[u8]) -> Option<DecodedImage> {
    let mut decoder = png::Decoder::new(bytes);

    decoder.set_transformations(png::Transformations::normalize_to_color8());

    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;

    let pixels = (info.width as usize).checked_mul(info.height as usize)?;
    let rgba = match info.color_type {
        png::ColorType::Rgba => {
            buf.truncate(pixels * 4);
            buf
        }
        png::ColorType::Rgb => expand(&buf, pixels, 3, |px, out| {
            out.extend_from_slice(&[px[0], px[1], px[2], 255]);
        })?,
        png::ColorType::GrayscaleAlpha => expand(&buf, pixels, 2, |px, out| {
            out.extend_from_slice(&[px[0], px[0], px[0], px[1]]);
        })?,
        png::ColorType::Grayscale => expand(&buf, pixels, 1, |px, out| {
            out.extend_from_slice(&[px[0], px[0], px[0], 255]);
        })?,

        png::ColorType::Indexed => return None,
    };

    Some(DecodedImage {
        rgba,
        width: info.width,
        height: info.height,
    })
}

fn decode_jpeg(bytes: &[u8]) -> Option<DecodedImage> {
    let mut decoder = jpeg_decoder::Decoder::new(bytes);
    let pixels_in = decoder.decode().ok()?;
    let info = decoder.info()?;

    let pixels = (info.width as usize).checked_mul(info.height as usize)?;

    let rgba = match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => expand(&pixels_in, pixels, 3, |px, out| {
            out.extend_from_slice(&[px[0], px[1], px[2], 255]);
        })?,
        jpeg_decoder::PixelFormat::L8 => expand(&pixels_in, pixels, 1, |px, out| {
            out.extend_from_slice(&[px[0], px[0], px[0], 255]);
        })?,
        jpeg_decoder::PixelFormat::L16 => expand(&pixels_in, pixels, 2, |px, out| {

            out.extend_from_slice(&[px[1], px[1], px[1], 255]);
        })?,
        jpeg_decoder::PixelFormat::CMYK32 => return None,
    };

    Some(DecodedImage {
        rgba,
        width: info.width as u32,
        height: info.height as u32,
    })
}

fn expand(
    src: &[u8],
    pixels: usize,
    stride: usize,
    f: impl Fn(&[u8], &mut Vec<u8>),
) -> Option<Vec<u8>> {
    if src.len() < pixels * stride {
        return None;
    }
    let mut out = Vec::with_capacity(pixels * 4);
    for i in 0..pixels {
        f(&src[i * stride..i * stride + stride], &mut out);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn red_blue_png() -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, 2, 1);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header().unwrap();
            w.write_image_data(&[255, 0, 0, 255, 0, 0, 255, 128]).unwrap();
        }
        out
    }

    #[test]
    fn decodes_png_to_straight_rgba() {
        let img = decode(&red_blue_png()).expect("png should decode");
        assert_eq!((img.width, img.height), (2, 1));
        assert_eq!(img.rgba.len(), 2 * 4);

        assert_eq!(&img.rgba[0..4], &[255, 0, 0, 255]);

        assert_eq!(&img.rgba[4..8], &[0, 0, 255, 128]);
    }

    #[test]
    fn png_without_alpha_becomes_opaque() {
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, 1, 1);
            enc.set_color(png::ColorType::Rgb);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header().unwrap();
            w.write_image_data(&[10, 20, 30]).unwrap();
        }
        let img = decode(&out).expect("rgb png should decode");
        assert_eq!(&img.rgba[..], &[10, 20, 30, 255]);
    }

    #[test]
    fn rejects_unknown_and_truncated_data() {
        assert!(decode(b"not an image at all").is_none());
        let png = red_blue_png();

        assert!(decode(&png[..png.len() / 2]).is_none());
    }
}
