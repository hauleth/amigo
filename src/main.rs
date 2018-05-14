extern crate flame;
extern crate histogram;
extern crate image;
extern crate palette;
#[macro_use]
extern crate clap;

use std::str;
use std::fmt;
use std::fs::File;

use image::GenericImage;
use image::Pixel;
use palette::rgb::standards::Srgb;
use palette::IntoColor;

#[derive(Copy, Clone, Debug, PartialEq)]
struct Color {
    hue: f64,
    stddev: f64,
}

impl Color {
    fn from_histogram(hist: histogram::Histogram, percentile: f64) -> Self {
        Color {
            hue: hist.percentile(percentile).unwrap() as f64,
            stddev: hist.stddev().unwrap() as f64,
        }
    }

    fn contains(self, color: f64) -> bool {
        (self.hue - color).abs() <= self.stddev
    }
}

fn dominant_color<Is: Iterator<Item = (u32, u32, impl Pixel<Subpixel = u8>)>>(
    input: Is,
    percentile: f64,
) -> Color {
    flame::span_of("dominant_color", || {
        let mut histogram = histogram::Histogram::configure()
            .max_value(360)
            .build()
            .unwrap();

        flame::start("histogram");
        input
            .map(|(_, _, pixel)| palette::Srgb::<f64>::from_pixel(&pixel.to_rgb().data))
            .map(|rgb| rgb.into_hsv::<Srgb>())
            .map(|hsv| hsv.hue.to_positive_degrees())
            .map(|bucket| bucket.round() as u64)
            .for_each(|value| histogram.increment(value).unwrap());
        flame::end("histogram");

        // print percentiles from the histogram
        println!(
            "Percentiles: p50: {} ns p90: {} ns p99: {} ns p999: {}",
            histogram.percentile(50.0).unwrap(),
            histogram.percentile(90.0).unwrap(),
            histogram.percentile(99.0).unwrap(),
            histogram.percentile(99.9).unwrap(),
        );

        // print additional statistics
        println!(
            "Hue (degrees): Min: {} Avg: {} Max: {} StdDev: {}",
            histogram.minimum().unwrap(),
            histogram.mean().unwrap(),
            histogram.maximum().unwrap(),
            histogram.stddev().unwrap(),
        );

        flame::start_guard("max");
        Color::from_histogram(histogram, percentile)
    })
}

fn merge<Is, Ib, Ps, Pb>(input: Is, color: Color, background: Ib, output: &mut image::RgbaImage)
where
    Is: Iterator<Item = (u32, u32, Ps)>,
    Ib: Iterator<Item = (u32, u32, Pb)>,
    Ps: Pixel<Subpixel = u8> + Send,
    Pb: Pixel<Subpixel = u8> + Send,
{
    flame::span_of("merge", || {
        input
            .zip(background)
            .map(|((x, y, source), (_, _, background))| (x, y, source, background))
            .map(|(x, y, source, background)| {
                let source = palette::Srgba::<f64>::from_pixel(&source.to_rgba().data);
                let hsv = source.color.into_hsv::<Srgb>();

                let result: [u8; 4] =
                    match (hsv.hue.to_positive_degrees(), hsv.saturation, hsv.value) {
                        (hue, saturation, value)
                            if color.contains(hue) && saturation >= 0.4 && value >= 0.1 =>
                        {
                            background.to_rgba().data
                        }
                        _ => source.into_pixel(),
                    };

                (x, y, result)
            })
            .for_each(|(x, y, data)| output.put_pixel(x, y, image::Rgba { data }));
    })
}

fn main() -> Result<(), Box<fmt::Error>> {
    flame::start("arg_parse");
    let matches = clap_app! { amigo =>
        (version: "0.1.0")
        (author: "Łukasz Niemier <lukasz@niemier.pl>")
        (@arg INPUT: -i --input * +takes_value "Input image")
        (@arg OUTPUT: -o --output * +takes_value "Output image")
        (@arg BG: -b --background * +takes_value "Background image to display")
        (@arg flame_html: --("flame-html") [FLAMEGRAPH] "Print flamegraph to HTML")
        (@arg profile: --profile "Print profiling information to stdout")
        (@arg percentile: --percentile -p [percentile] default_value("30") "Set histogram percentile as a hue value")
    }.get_matches();
    flame::end("arg_parse");

    let input = flame::span_of("load_input", || {
        image::open(matches.value_of("INPUT").unwrap()).expect("Cannot open file")
    });
    let mut bg = flame::span_of("load_bg", || {
        image::open(matches.value_of("BG").unwrap()).expect("Cannot open background file")
    });
    let mut output = image::RgbaImage::new(input.width(), input.height());

    flame::start("resize_bg");
    if bg.dimensions() != input.dimensions() {
        bg = bg.resize_to_fill(input.width(), input.height(), image::FilterType::Lanczos3);
    }
    flame::end("resize_bg");

    let percentile = matches.value_of("percentile").and_then(|v| str::parse(v).ok()).unwrap_or(30.0);
    let color = dominant_color(input.pixels(), percentile);
    println!("Using hue: {} (dev {})", color.hue, color.stddev);

    merge(input.pixels(), color, bg.pixels(), &mut output);

    flame::span_of("save", || {
        output.save(matches.value_of("OUTPUT").unwrap()).unwrap()
    });

    if let Some(file) = matches.value_of("flame_html") {
        flame::dump_html(&mut File::create(file).unwrap()).unwrap();
    }

    if matches.is_present("profile") {
        flame::dump_stdout();
    }

    Ok(())
}
