use image::open;
use std::path::PathBuf;

fn naive_escape(z: &str) -> String {
    z.replace("'", "\'").replace('"', "\\\"")
}
use std::time::Instant;

// https://stackoverflow.com/a/8344059
// Because... html area maps are really unusable.
fn make_html<'a>(
    width: u32,
    height: u32,
    matches: &[histogram_text_matcher::Match2D<'a>],
    image_path: &PathBuf,
    out_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::Write;

    let mut c = String::new();
    let mut css = String::new();
    let mut code = String::new();
    let mut areas = String::new();

    for (i, m) in matches.iter().enumerate() {
        let name = format!("area_{}", i);

        // #area2 { left:320px; right:40px; bottom: 20px; top: 50px;
        let l = m.location.left();
        let t = m.location.top();
        let w = m.location.width();
        let h = m.location.height();
        css.push_str(&format!(
            "#{} {{ left: {}px; top: {}px; width: {}px; height: {}px; }}\n",
            &name, l, t, w, h
        ));

        // Constant to display on mouse over.
        code.push_str(&format!(
            "const {} = \"{}\";\n",
            &name,
            &naive_escape(&format!("{:?} - {:?}", m.location, m.tokens))
        ));

        //  <a id="area1" class="area" href="#"></a>
        areas.push_str(&format!("<a  id=\"{}\" class=\"area\" ", &name));
        areas.push_str(&format!(" onmouseover=\"z({})\" ", name));
        let joined = m
            .tokens
            .iter()
            .map(|x| x.glyph.glyph())
            .collect::<Vec<&str>>()
            .join("");
        areas.push_str(&format!(" href=\"#\">{}</a>\n", &joined));
    }

    c.push_str(&format!(
        "<html><body><style>
        .area {{
            display:block;
            position:absolute;
            border: 1px solid red;
        }}

        .area:hover {{ 
            border: 1px solid green;
        }}

        {}

        .theimage {{
            display: block; 
        }}
    </style>",
        &css
    ));
    c.push_str(&"<p id=\"n\"></p>");
    c.push_str(
        &"<script>
        function z(m){
            document.getElementById(\"n\").innerHTML = m;
            console.log(m);
        };\n",
    );

    c.push_str(&code);

    c.push_str(
        &"</script>
        <div class=\"base\">",
    );
    c.push_str(&areas);

    c.push_str(&format!(
        "<img class=\"theimage\" src =\"{}\" width=\"{}\" height=\"{}\"/>",
        image_path.to_string_lossy(),
        width,
        height
    ));
    c.push_str(&"</div></body></html>");

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
    let labels = vec![(histogram_text_matcher::RGB::white(), 0)];

    let now = Instant::now();
    let matches = histogram_text_matcher::moving_windowed_histogram(&image, &glyph_set, &labels);
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
