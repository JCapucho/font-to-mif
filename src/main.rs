use ab_glyph::{Font, FontVec};
use clap::{App, Arg};
use std::{
    fs,
    io::{self, Write},
    ops::Range,
    path::Path,
};

const ABOUT: &str = r#"
Converts true type fonts (.ttf) and Open type fonts (.otf) to intel quartus Memory Initialization File (.mif)
"#;

fn range_parser(range: &str) -> Result<Range<usize>, String> {
    let mut chars = range.chars();

    if let Some(first) = chars.next() {
        if !first.is_digit(10) {
            return Err(String::from("First char must be an integer"));
        }

        let mut start_end = 1;
        let mut end_start = None;

        while let Some(c) = chars.next() {
            if c.is_digit(10) {
                start_end += 1
            } else if c == '.' {
                if chars.next() != Some('.') {
                    return Err(String::from("Expected '.'"));
                }

                if !chars.next().map(|c| c.is_digit(10)).unwrap_or(false) {
                    return Err(String::from("Character after '.' must be a digit"));
                }

                end_start = Some(start_end + 2);

                while let Some(c) = chars.next() {
                    if !c.is_digit(10) {
                        return Err(String::from("Invalid char"));
                    }
                }
            } else {
                return Err(String::from("Invalid char"));
            }
        }

        let start = range[..start_end].parse().unwrap();

        Ok(if let Some(high_start) = end_start {
            let end = range[high_start..].parse().unwrap();

            Range { start, end }
        } else {
            Range {
                start,
                end: start + 1,
            }
        })
    } else {
        Err(String::from("Empty string is an invalid range"))
    }
}

fn main() -> io::Result<()> {
    let matches = App::new("font to intel quartus MIF converter")
        .version("0.1")
        .author("João Capucho <jcapucho7@gmail.com>")
        .about(ABOUT)
        .arg(
            Arg::with_name("FONT")
                .help("Sets the font file to use")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("out")
                .value_name("FILE")
                .default_value("./font.mif")
                .help("Sets the path to the output file"),
        )
        .arg(
            Arg::with_name("range")
                .short("r")
                .long("range")
                .value_name("RANGE")
                .default_value("0..256")
                .validator(|value| range_parser(&value).map(|_| ()))
                .help("Sets the range of glyphs to process"),
        )
        .get_matches();

    let path = Path::new(matches.value_of("FONT").unwrap());
    let range = range_parser(matches.value_of("range").unwrap()).unwrap();

    let font_data = fs::read(path)?;

    let font = FontVec::try_from_vec(font_data).unwrap();

    let depth = range.len() * 64;

    let mut data: Vec<u8> = vec![0; depth / 8];

    for i in range {
        let glyph = font.glyph_id(From::from(i as u8)).with_scale(8.0);

        if let Some(outlined) = font.outline_glyph(glyph) {
            outlined.draw(|x, y, coverage| {
                if coverage != 0.0 {
                    data[i * 8 + y as usize] |= 1 << x;
                }
            });
        }
    }

    let mut out = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(matches.value_of("output").unwrap())?;

    writeln!(
        &mut out,
        "-- {} \n",
        path.file_name().unwrap().to_str().unwrap()
    )?;
    writeln!(&mut out, "DEPTH = {};", depth)?;
    writeln!(&mut out, "WIDTH = 1;")?;
    writeln!(&mut out, "ADDRESS_RADIX = HEX;")?;
    writeln!(&mut out, "DATA_RADIX = HEX;\n")?;
    writeln!(&mut out, "CONTENT")?;
    writeln!(&mut out, "BEGIN")?;
    writeln!(&mut out)?;

    for (addr, byte) in data.into_iter().enumerate() {
        writeln!(&mut out, "{:04X} : {:02X};", addr * 8, byte)?;
    }

    writeln!(&mut out)?;
    writeln!(&mut out, "END;")?;

    Ok(())
}
