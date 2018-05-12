extern crate image;
extern crate palette;
#[macro_use]
extern crate clap;

use std::fmt;

use image::GenericImage;
use image::Pixel;
use palette::rgb::standards::Srgb;
use palette::IntoColor;

const BUCKETS: usize = 360;
const HUE_STEP: f64 = 360.0 / BUCKETS as f64;

fn dominant_color<Is: Iterator<Item = (u32, u32, impl Pixel<Subpixel = u8>)>>(input: Is) -> f64 {
    let mut buckets = [0; BUCKETS + 1];
    let hues = input
        .map(|(_, _, pixel)| palette::Srgb::<f64>::from_pixel(&pixel.to_rgb().data))
        .map(|rgb| rgb.into_hsv::<Srgb>())
        .map(|hsv| hsv.hue.to_positive_degrees());

    for hue in hues {
        buckets[(hue / HUE_STEP).round() as usize] += 1;
    }

    buckets
        .iter()
        .enumerate()
        .max_by_key(|&(_, count)| count)
        .map_or(0.0, |(idx, _)| idx as f64 * HUE_STEP)
}

fn merge<Is, Ib, Ps, Pb>(input: Is, color: f64, bg: Ib, output: &mut image::RgbaImage)
where
    Is: Iterator<Item = (u32, u32, Ps)>,
    Ib: Iterator<Item = (u32, u32, Pb)>,
    Ps: Pixel<Subpixel = u8>,
    Pb: Pixel<Subpixel = u8>,
{
    for ((x, y, source), (_, _, background)) in input.zip(bg) {
        let mut source = palette::Srgba::<f64>::from_pixel(&source.to_rgba().data);
        let mut background = palette::Srgba::<f64>::from_pixel(&background.to_rgba().data);

        let hsv = source.color.into_hsv::<Srgb>();

        let result = match (hsv.hue.to_positive_degrees(), hsv.saturation, hsv.value) {
            (hue, saturation, value)
                if (hue - color).abs() <= 20.0 && saturation >= 0.4 && value >= 0.1 =>
            {
                background
            }
            _ => source,
        };

        output.put_pixel(
            x,
            y,
            image::Rgba {
                data: result.into_pixel(),
            },
        );
    }
}

fn main() -> Result<(), Box<fmt::Error>> {
    let matches = clap_app! { amigo =>
        (version: "0.1.0")
        (author: "Łukasz Niemier <lukasz@niemier.pl>")
        (@arg INPUT: -i --input +required +takes_value "Input image")
        (@arg OUTPUT: -o --output +required +takes_value "Output image")
        (@arg BG: -b --background +required +takes_value "Background image to display")
    }.get_matches();

    let input = image::open(matches.value_of("INPUT").unwrap()).expect("Cannot open file");
    let bg = image::open(matches.value_of("BG").unwrap())
        .expect("Cannot open background file")
        .resize_to_fill(input.width(), input.height(), image::FilterType::Lanczos3);
    let mut output = image::RgbaImage::new(input.width(), input.height());

    let color = dominant_color(input.pixels());
    println!("Using hue: {}", color);

    merge(input.pixels(), color, bg.pixels(), &mut output);

    output.save(matches.value_of("OUTPUT").unwrap()).unwrap();

    Ok(())
}
