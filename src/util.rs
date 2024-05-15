use crate::Match2D;
use image::{GenericImageView, Pixel};
use std::path::Path;
/// Function to render an html page for inspecting matches.
pub fn write_match_html<'a>(
    width: u32,
    height: u32,
    matches: &[Match2D<'a>],
    labels: &[crate::ColorLabel],
    image_path: &Path,
    out_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::Write;
    let mut c: String = String::new();

    c.push_str(
        &r##"<!DOCTYPE html>
            <html>
            <head>
                <style>
                svg .area-of-interest {
                    stroke-width: 2px;
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
            var update_id = null;
            let d = (a) => document.getElementById(a);
            let combined = (match) => match.tokens.map((a) => a.glyph.glyph).join("");
            let labels = (match) => match.tokens.map((a) => a.label).filter((v, i, a) => a.indexOf(v) === i).join(", ");
            function mouse_move(e, match){
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
                d("tooltip-combined").innerHTML = match_str + "<br>" +  JSON.stringify(match.location) + "<br>label: " + labels(match);
            }

            function mouse_click(e, match){
                let match_str = combined(match);
                d("message").innerHTML = match_str + "<br>" +  JSON.stringify(match.location) + "<br>label: " + labels(match);
            }
            function mouse_out(e, match){
                d("tooltip").setAttributeNS(null, "visibility", "hidden");
            }

            function add_rectangles(matches) {
                let rects = d("rectangles");
                rects.innerHTML = "";
                for (let match of matches) {
                    var rect = document.createElementNS("http://www.w3.org/2000/svg", "rect");
                    rect.setAttributeNS(null, 'width', match.location.w);
                    rect.setAttributeNS(null, 'height', match.location.h);
                    rect.setAttributeNS(null, 'x', match.location.x);
                    rect.setAttributeNS(null, 'y', match.location.y);
                    rect.onmousemove = (event) => mouse_move(event, match);
                    rect.onmouseout = (event) => mouse_out(event, match);
                    rect.onclick = (event) => mouse_click(event, match);
                    if (update_id != null) {
                      // If we are doing live updates, we probably want a text.
                      var text = document.createElementNS("http://www.w3.org/2000/svg", "text");
                      text.setAttributeNS(null, 'x', match.location.x);
                      text.setAttributeNS(null, 'y', match.location.y + match.location.h - 3);
                      text.innerHTML = combined(match);
                      rects.appendChild(text);
                    }
                    rect.classList.add('area-of-interest');
                    rect.classList.add('label_' + match.tokens[0].label);
                    rects.appendChild(rect);
                }
              }

            var update_id = null;
            function update_poll() {
              let endpoint = d("remote_json_endpoint").value;
              let interval = d("remote_json_poll_interval").value + 0;
              clearInterval(update_id);
              let f = () => {
                fetch(endpoint)
                  .then(response => response.json())
                  .then(data => add_rectangles(data));
              };

              update_id = setInterval(f, interval);
            }
            "##,
    );

    c.push_str(&format!(
        r#"const matches = {};
        </script>
        <p id="message">Click an area to provide the information here.</p>
        <input id="remote_json_endpoint" type="text" onchange="update_poll()" />
        <input id="remote_json_poll_interval" type="number" onchange="update_poll()" value="50" min="10" max="10000" />
        "#,
        &serde_json::to_string(&matches).expect("cannot fail")
    ));

    c.push_str(&format!(
        r#"<svg id="svg_el" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"
        width="{width}" height="{height}" viewBox="0 0 {width} {height}" version="1.1">
        <image xlink:href="{file}" width="{width}" height="{height}"
        preserveAspectRatio="none" x="0" y="0" style="image-rendering: pixelated" />"#,
        file = image_path.to_string_lossy()
    ));

    // Draw the group for the rectangles.
    c.push_str(&format!(
        r#"
        <g id="rectangles" >
        </g>
        "#,
    ));

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

    c.push_str(&"</svg>");

    let mut color_label_css: String = String::new();
    for (c, label) in labels.iter() {
        color_label_css.push_str(&format!(
            "svg .label_{l} {{
                stroke: #{r:0>2x}{g:0>2x}{b:0>2x};
            }}
            ",
            l = label,
            r = c.channels()[0],
            g = c.channels()[1],
            b = c.channels()[2]
        ));
    }

    c.push_str(&format!(
        r#"<style>
            {}
        </style>
        "#,
        color_label_css
    ));

    c.push_str(
        &"
      <script>
          add_rectangles(matches);
      </script>
    </body></html>",
    );

    let mut file = File::create(out_path)?;
    file.write(&c.as_bytes())?;
    Ok(())
}

pub fn image_as_svg<I: image::GenericImageView>(image: &I, width: u32, height: u32) -> String
where
    <<I as GenericImageView>::Pixel as Pixel>::Subpixel: std::fmt::Display,
{
    let mut c: String = String::new();
    let image_width = image.width();
    let image_height = image.height();
    c.push_str(&format!(
        r#"<svg id="svg_el" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"
        width="{width}" height="{height}" viewBox="0 0 {image_width} {image_height}" version="1.1">
        "#,
    ));
    for y in 0..image.height() {
        for x in 0..image.width() {
            let color = image.get_pixel(x, y);
            c.push_str(&format!(
                r#"<rect style="fill:rgb({r},{g},{b});" shape-rendering="crispEdges"
                    width="1"
                    height="1"
                    x="{x}"
                    y="{y}"
                />
                "#,
                r = color.channels()[0],
                g = color.channels()[1],
                b = color.channels()[2]
            ));
        }
    }
    c.push_str("\n</svg>");
    c
}

pub fn parse_json_labels(data: &str) -> serde_json::Result<Vec<(image::Rgb<u8>, u32)>> {
    let mut res: Vec<(image::Rgb<u8>, u32)> = vec![];
    let v: Vec<(u8, u8, u8, u32)> = serde_json::from_str(data)?;
    for r in v {
        res.push((image::Rgb::<u8>([r.0, r.1, r.2]), r.3));
    }
    Ok(res)
}
