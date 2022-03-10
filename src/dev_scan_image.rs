use image::open;
use std::path::PathBuf;
use std::time::Instant;

fn make_html<'a>(
    width: u32,
    height: u32,
    matches: &[histogram_text_matcher::Match2D<'a>],
    image_path: &PathBuf,
    out_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::Write;
    let mut c: String = String::new();
    let mut rects: String = String::new();

    for (i, m) in matches.iter().enumerate() {
        rects.push_str(&format!(
            r#"<rect id="roi_{i}" class="area-of-interest"
                    width="{w}"
                    height="{h}"
                    x="{l}"
                    y="{t}"
                    onmousemove="mouse_move(event, {i});"
                    onmouseout="mouse_out(event, {i});"
                    onclick="mouse_click(event, {i});"
                />
                "#,
            l = m.location.left(),
            t = m.location.bottom(),
            w = m.location.width(),
            h = m.location.height(),
        ));
    }

    c.push_str(
        &r##"<!DOCTYPE html>
            <html>
            <head>
                <style>
                svg .area-of-interest {
                    stroke-width: 2px;
                    stroke: green;
                    fill: transparent;
                }
                svg .area-of-interest:hover {
                    stroke: red;
                    fill: rgba(255,0,0,0.25);
                }
                #tooltip-combined {
                    color: black;
                    font: 18px serif;
                    display: inline-block;
                    background-color: #EEE;
                    padding: 5px;
                    border-radius: 10px;
                }
                #message {
                    min-height: 50px;
                }
                </style>
            </head>
            <body>
                <script>
            let d = (a) => document.getElementById(a);
            let combined = (match) => match.tokens.map((a) => a.glyph.glyph).join("");
            function mouse_move(e, index){
                let match = matches[index];
                let match_str = combined(match);

                let svg_el = d("svg_el");
                let tooltip = d("tooltip");
                var point = svg_el.createSVGPoint();
                point.x = e.clientX;
                point.y = e.clientY;
                var ctm = svg_el.getScreenCTM();
                var inverse = ctm.inverse();
                var p = point.matrixTransform(inverse);
                // position the tooltip.
                d("tooltip").transform.baseVal.getItem(0).setTranslate(p.x,p.y);
                // Set tooltip to visible.
                d("tooltip").setAttributeNS(null, "visibility", "visible");
                // Set the fancy embedded html text.
                d("tooltip-combined").innerHTML = match_str + "<br>" +  JSON.stringify(match.location);
            }

            function mouse_click(e, index){
                let match = matches[index];
                let match_str = combined(match);
                d("message").innerHTML = match_str + "<br>" +  JSON.stringify(match.location);
            }
            function mouse_out(e, index){
                d("tooltip").setAttributeNS(null, "visibility", "hidden");
            }
            "##,
    );

    c.push_str(&format!(
        r#"const matches = {};
        </script>
        <p id="message">Click an area to provide the information here.</p>"#,
        &serde_json::to_string(&matches).expect("cannot fail")
    ));

    c.push_str(&format!(
        r#"<svg id="svg_el" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"
        width="{width}" height="{height}" viewBox="0 0 {width} {height}" version="1.1">
        <image xlink:href="{file}" width="{width}" height="{height}"
        preserveAspectRatio="none" x="0" y="0" />"#,
        file = image_path.to_string_lossy()
    ));

    c.push_str(&rects);

    // Draw the tooltip after the rectangles such that it goes over them.
    c.push_str(&format!(
        r#"<g id="tooltip" x="0" y="0" visibility="hidden" transform="translate(0,0)" >
            <foreignObject x="10" y="10" width="1000" height="1000">
                <div id="tooltip-combined"  xmlns="http://www.w3.org/1999/xhtml">
                </div>
            </foreignObject>
        </g>
        "#,
    ));

    c.push_str(&"</svg></body></html>");

    let mut file = File::create(out_path)?;
    file.write(&c.as_bytes())?;
    Ok(())
}

fn main() {
    if std::env::args().len() <= 1 {
        println!("expected: ./binary glyph_set_file input_image_file");
        println!("glyph_set_file: File to load the glyph set from.");
        println!("input_image_file: File to search in.");
        std::process::exit(1);
    }

    let glyph_set_file = std::env::args()
        .nth(1)
        .expect("no glyph set file specified");

    let input_image_file = std::env::args().nth(2).expect("no image file specified");

    let glyph_path = PathBuf::from(&glyph_set_file);
    let glyph_set = histogram_text_matcher::glyphs::load_glyph_set(&glyph_path)
        .expect(&format!("could not load image at {:?}", glyph_set_file));

    let image_path = PathBuf::from(&input_image_file);
    let orig_image = open(&image_path)
        .expect(&format!("could not load image at {:?}", input_image_file))
        .to_rgb8();

    let image = histogram_text_matcher::image_support::rgb_image_to_view(&orig_image);
    use histogram_text_matcher::RGB;
    let labels = vec![
        (RGB::white(), 0),
        (RGB::rgb(238, 238, 238), 1),
        (RGB::rgb(100, 100, 255), 1),
        (RGB::rgb(194, 192, 107), 1),
        (RGB::rgb(194, 172, 109), 4),
    ];

    let matcher = histogram_text_matcher::matcher::LongestGlyphMatcher::new(&glyph_set.entries);

    let now = Instant::now();
    let matches = histogram_text_matcher::moving_windowed_histogram(&image, &glyph_set, &matcher, &labels);
    for m in matches.iter() {
        let location = &m.location;
        print!("{location:?} -> ");
        for t in m.tokens.iter() {
            let l = t.label;
            let g = t.glyph.glyph();
            print!(" {g:?}#{l}");
        }
        println!();
    }
    println!("Took {}", now.elapsed().as_secs_f64());

    use std::fs;
    make_html(
        orig_image.width(),
        orig_image.height(),
        &matches,
        &fs::canonicalize(image_path).expect("can be made absolute"),
        &PathBuf::from("/tmp/zzz.html"),
    )
    .expect("should succeed");
}
