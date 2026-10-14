//! Compare an original PNG with optimized PNG/SVG/SVGZ exports.
//! cargo run --release --example check_compression -- original.png smaller.png smaller.svg smaller.svgz key-file
use code_is_magic::*;
use std::{env, fs};
fn raster(svg: &str, width: u32, height: u32) -> Vec<u8> {
    let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default()).unwrap();
    let mut image = resvg::tiny_skia::Pixmap::new(width, height).unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(
            width as f32 / tree.size().width(),
            height as f32 / tree.size().height(),
        ),
        &mut image.as_mut(),
    );
    image.take()
}
fn png_pixels(path: &str) -> Vec<u8> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(fs::read(path).unwrap()));
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().unwrap();
    let mut data = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut data).unwrap();
    data[..frame.buffer_size()]
        .chunks_exact(frame.color_type.samples())
        .flat_map(|p| match frame.color_type {
            png::ColorType::Rgb => [p[0], p[1], p[2], 255],
            png::ColorType::Grayscale => [p[0], p[0], p[0], 255],
            png::ColorType::GrayscaleAlpha => [p[0], p[0], p[0], p[1]],
            _ => [p[0], p[1], p[2], p[3]],
        })
        .collect()
}
fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    assert!(
        args.len() == 4 || args.len() == 5,
        "Provide original.png optimized.png optimized.svg optimized.svgz [key-file]"
    );
    let key = args
        .get(4)
        .map(|p| MagicKey::from_hex(&fs::read_to_string(p).unwrap()).unwrap());
    let original = decode_image_with_key(&fs::read(&args[0]).unwrap(), key.as_ref()).unwrap();
    assert!(
        png_pixels(&args[0]) == png_pixels(&args[1]),
        "Optimized PNG has changed a pixel"
    );
    let info = png::Decoder::new(std::io::Cursor::new(fs::read(&args[0]).unwrap()))
        .read_info()
        .unwrap();
    let width = info.info().width;
    let height = info.info().height;
    let pixels = raster(&original.svg, width, height);
    let names = decode_svg_names_with_key(&original.svg, key.as_ref()).unwrap();
    for path in &args[1..4] {
        let recovered = decode_image_with_key(&fs::read(path).unwrap(), key.as_ref()).unwrap();
        assert_eq!(recovered.snapshot, original.snapshot);
        assert!(
            raster(&recovered.svg, width, height) == pixels,
            "Artwork pixels differ: {path}"
        );
        assert_eq!(
            decode_svg_names_with_key(&recovered.svg, key.as_ref()).unwrap(),
            names
        );
        println!("{path}: {} bytes; {} exact entries and {} exact names; {width}×{height} vector pixels identical",fs::metadata(path).unwrap().len(),recovered.snapshot.entries.len(),names.len());
    }
}
