use criterion::{black_box, criterion_group, criterion_main, Criterion};
use image::open;
use std::path::PathBuf;

use histogram_text_matcher::RGB;

fn criterion_benchmark(c: &mut Criterion) {
    let image_path = std::env::var("BENCH_SCAN_IMAGE").expect("BENCH_SCAN_IMAGE should be a path to an image");
    let image = open(image_path).expect("Failed to load file").to_rgb8();
    let glyph_set_file = std::env::var("BENCH_SCAN_GLYPH_SET").expect("BENCH_SCAN_GLYPH_SET should be a path to a glyph set");
    let color_labels = std::env::var("BENCH_SCAN_COLOR_LABELS").expect("BENCH_SCAN_COLOR_LABELS should be a path to a glyph set");

    let glyph_path = PathBuf::from(&glyph_set_file);
    let glyph_set = histogram_text_matcher::glyphs::load_glyph_set(&glyph_path)
        .expect(&format!("could not load glyph set at {:?}", glyph_set_file));

    let image = histogram_text_matcher::image_support::rgb_image_to_view(&image);

    let matcher = histogram_text_matcher::matcher::LongestGlyphMatcher::new(&glyph_set.entries);

    let labels = histogram_text_matcher::util::parse_json_labels(&color_labels).expect("invalid json");

    let mut group = c.benchmark_group("moving_windowed_histogram");
    {
        group.significance_level(0.1).sample_size(500);
        group.bench_function("iterator", |b| b.iter(|| {
            let matches = histogram_text_matcher::moving_windowed_histogram(
                &image,
                glyph_set.line_height,
                &matcher,
                &labels,
            );
        }));
        group.bench_function("direct", |b| b.iter(|| {
            let matches = histogram_text_matcher::moving_windowed_histogram2(
                &image,
                glyph_set.line_height,
                &matcher,
                &labels,
            );
        }));
    }
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);