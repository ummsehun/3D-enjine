use std::io::{self, Write};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use image::{ColorType, ImageEncoder, Rgb, RgbImage, codecs::png::PngEncoder};

use crate::{renderer::FrameBuffers, scene::GraphicsProtocol};

const CELL_PIXELS_W: u32 = 2;
const CELL_PIXELS_H: u32 = 4;
const BACKGROUND_RGB: [u8; 3] = [26, 32, 44];
const KITTY_CHUNK_LEN: usize = 4096;

pub fn detect_supported_protocol(requested: GraphicsProtocol) -> Option<GraphicsProtocol> {
    match requested {
        GraphicsProtocol::None => None,
        GraphicsProtocol::Kitty => supports_kitty().then_some(GraphicsProtocol::Kitty),
        GraphicsProtocol::Iterm2 => supports_iterm2().then_some(GraphicsProtocol::Iterm2),
        GraphicsProtocol::Auto => {
            if supports_kitty() {
                Some(GraphicsProtocol::Kitty)
            } else if supports_iterm2() {
                Some(GraphicsProtocol::Iterm2)
            } else {
                None
            }
        }
    }
}

pub fn write_graphics_frame(
    writer: &mut impl Write,
    frame: &FrameBuffers,
    protocol: GraphicsProtocol,
) -> io::Result<()> {
    let png = encode_png_frame(frame)?;
    match protocol {
        GraphicsProtocol::Kitty => write_kitty_frame(writer, &png, frame.width, frame.height),
        GraphicsProtocol::Iterm2 => write_iterm2_frame(writer, &png),
        _ => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "graphics protocol is not available",
        )),
    }
}

fn supports_kitty() -> bool {
    let term = std::env::var("TERM")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let term_program = std::env::var("TERM_PROGRAM")
        .unwrap_or_default()
        .to_ascii_lowercase();
    term.contains("kitty") || term_program.contains("ghostty")
}

fn supports_iterm2() -> bool {
    let term_program = std::env::var("TERM_PROGRAM")
        .unwrap_or_default()
        .to_ascii_lowercase();
    term_program.contains("iterm")
}

fn encode_png_frame(frame: &FrameBuffers) -> io::Result<Vec<u8>> {
    let width = u32::from(frame.width).max(1) * CELL_PIXELS_W;
    let height = u32::from(frame.height).max(1) * CELL_PIXELS_H;
    let mut image = RgbImage::from_pixel(width, height, Rgb(BACKGROUND_RGB));

    for y in 0..u32::from(frame.height) {
        for x in 0..u32::from(frame.width) {
            let idx = (y as usize)
                .saturating_mul(frame.width as usize)
                .saturating_add(x as usize);
            let glyph = frame.glyphs.get(idx).copied().unwrap_or(' ');
            let fg = frame.fg_rgb.get(idx).copied().unwrap_or([255, 255, 255]);
            let coverage = glyph_coverage(glyph);
            fill_cell(
                &mut image,
                x * CELL_PIXELS_W,
                y * CELL_PIXELS_H,
                fg,
                BACKGROUND_RGB,
                coverage,
            );
        }
    }

    let mut bytes = Vec::new();
    let encoder = PngEncoder::new(&mut bytes);
    encoder
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ColorType::Rgb8.into(),
        )
        .map_err(io::Error::other)?;
    Ok(bytes)
}

fn fill_cell(
    image: &mut RgbImage,
    start_x: u32,
    start_y: u32,
    fg: [u8; 3],
    bg: [u8; 3],
    coverage: f32,
) {
    let total = CELL_PIXELS_W * CELL_PIXELS_H;
    let lit = (coverage.clamp(0.0, 1.0) * total as f32).round() as u32;
    let mut index = 0_u32;
    for py in 0..CELL_PIXELS_H {
        for px in 0..CELL_PIXELS_W {
            let color = if index < lit { fg } else { bg };
            image.put_pixel(start_x + px, start_y + py, Rgb(color));
            index += 1;
        }
    }
}

fn glyph_coverage(glyph: char) -> f32 {
    if glyph == ' ' {
        return 0.0;
    }
    let code = glyph as u32;
    if (0x2800..=0x28ff).contains(&code) {
        let mask = (code - 0x2800) as u8;
        return (mask.count_ones() as f32 / 8.0).clamp(0.20, 1.0);
    }
    match glyph {
        '.' | '\'' | '`' => 0.35,
        ':' | ';' => 0.45,
        '-' | '_' => 0.55,
        '=' | '+' => 0.70,
        '*' | 'x' | 'X' => 0.80,
        '#' => 0.90,
        '%' => 0.95,
        '@' => 1.0,
        _ => 0.82,
    }
}

fn write_kitty_frame(
    writer: &mut impl Write,
    png: &[u8],
    cells_w: u16,
    cells_h: u16,
) -> io::Result<()> {
    let data = STANDARD.encode(png);
    let px_w = u32::from(cells_w).saturating_mul(CELL_PIXELS_W);
    let px_h = u32::from(cells_h).saturating_mul(CELL_PIXELS_H);

    write!(writer, "\x1b[H")?;

    if data.len() <= KITTY_CHUNK_LEN {
        write!(writer, "\x1b_Ga=T,f=100,t=d,s={px_w},v={px_h};{data}\x1b\\")?;
        writer.flush()?;
        return Ok(());
    }

    let mut offset = 0usize;
    let mut first = true;
    while offset < data.len() {
        let end = (offset + KITTY_CHUNK_LEN).min(data.len());
        let chunk = &data[offset..end];
        let more = if end < data.len() { 1 } else { 0 };
        if first {
            write!(
                writer,
                "\x1b_Ga=T,f=100,t=d,s={px_w},v={px_h},m={more};{chunk}\x1b\\"
            )?;
            first = false;
        } else {
            write!(writer, "\x1b_Gm={more};{chunk}\x1b\\")?;
        }
        offset = end;
    }
    writer.flush()
}

fn write_iterm2_frame(writer: &mut impl Write, png: &[u8]) -> io::Result<()> {
    let data = STANDARD.encode(png);
    write!(writer, "\x1b[H")?;
    write!(
        writer,
        "\x1b]1337;File=inline=1;width=100%;height=100%;preserveAspectRatio=0:{data}\x07"
    )?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::FrameBuffers;

    #[test]
    fn coverage_for_braille_is_nonzero() {
        assert!(glyph_coverage('⣿') > 0.9);
        assert!(glyph_coverage(' ') < 0.01);
    }

    #[test]
    fn encode_png_has_data() {
        let mut frame = FrameBuffers::new(2, 1);
        frame.glyphs[0] = '@';
        frame.glyphs[1] = '.';
        frame.fg_rgb[0] = [255, 64, 64];
        frame.fg_rgb[1] = [64, 255, 255];
        let png = encode_png_frame(&frame).expect("png");
        assert!(png.len() > 32);
    }
}
