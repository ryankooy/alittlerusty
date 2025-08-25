use std::error::Error;
use std::num::ParseIntError;
use std::path::PathBuf;

use clap::{Arg, Command, builder::PathBufValueParser};

/*
 * Binary, octal, decimal, and hexadecimal conversions.
 */

enum Numeral {
    Binary,
    Octal,
    Decimal,
    Hexadecimal,
    Invalid,
}

#[derive(Clone)]
struct NumeralInfo<'a> {
    name: &'a str,
    base: u32,
}

impl Numeral {
    fn new(num_type: &str) -> Numeral {
        match num_type {
            "bin" | "binary" => Numeral::Binary,
            "oct" | "octal" => Numeral::Octal,
            "" | "dec" | "decimal" => Numeral::Decimal,
            "hex" | "hexadecimal" => Numeral::Hexadecimal,
            _ => Numeral::Invalid
        }
    }

    fn info(&self) -> Result<NumeralInfo, &'static str> {
        match &self {
            Numeral::Binary => Ok(NumeralInfo { name: "binary", base: 2 }),
            Numeral::Octal => Ok(NumeralInfo { name: "octal", base: 8 }),
            Numeral::Decimal => Ok(NumeralInfo { name: "decimal", base: 10 }),
            Numeral::Hexadecimal => Ok(NumeralInfo { name: "hexadecimal", base: 16 }),
            Numeral::Invalid => Err("Invalid numeral type")
        }
    }
}

trait Number<'a> {
    fn new(number: &'a str, input_numeral_info: &'a NumeralInfo) -> Self;
    fn value(&self) -> Result<String, ParseIntError>;
    fn print(&self) -> Result<(), Box<dyn Error>>;
}

macro_rules! make_struct {
    ($name:ident, $numtype:expr, $fmtstr:expr) => {
        struct $name<'a> {
            number: &'a str,
            numeral_info: NumeralInfo<'a>,
            input_numeral_info: NumeralInfo<'a>,
        }

        impl <'a> Number<'a> for $name<'a> {
            fn new(number: &'a str, input_numeral_info: &'a NumeralInfo) -> $name<'a> {
                let numeral_info: NumeralInfo = $numtype.info().unwrap();
                let input_numeral_info: NumeralInfo = input_numeral_info.clone();

                $name { number, numeral_info, input_numeral_info }
            }

            fn value(&self) -> Result<String, ParseIntError> {
                match i128::from_str_radix(self.number, self.input_numeral_info.base) {
                    Ok(value) => Ok(format!($fmtstr, value)),
                    Err(e) => Err(e)
                }
            }

            fn print(&self) -> Result<(), Box<dyn Error>> {
                match self.value() {
                    Ok(value) => {
                        println!(
                            "{value}  <- {numtype} (base {base})",
                            value=value.to_string(),
                            numtype=capitalize(self.numeral_info.name),
                            base=self.numeral_info.base
                        );
                    }
                    Err(e) => {
                        return Err(
                            format!(
                                "Invalid {} value: {}", self.input_numeral_info.name, e.to_string()
                            ).into()
                        )
                    }
                }

                Ok(())
            }
        }
    };
}

make_struct!(Binary, Numeral::Binary, "{:b}");
make_struct!(Octal, Numeral::Octal, "{:#o}");
make_struct!(Decimal, Numeral::Decimal, "{}");
make_struct!(Hex, Numeral::Hexadecimal, "{:#x}");

fn main() -> Result<(), Box<dyn Error>> {
    let config = parse_config()?;
    let number: &str = config.number.as_str();
    let numeral_type = Numeral::new(config.numeral_type.as_str());
    let numeral_info: NumeralInfo = numeral_type.info()?;

    print_number_info(&number, &numeral_info);

    let binary: Binary = Number::new(&number, &numeral_info);
    let octal: Octal = Number::new(&number, &numeral_info);
    let decimal: Decimal = Number::new(&number, &numeral_info);
    let hex: Hex = Number::new(&number, &numeral_info);

    binary.print()?;
    octal.print()?;
    decimal.print()?;
    hex.print()?;

    Ok(())
}

struct Config {
    number: String,
    numeral_type: String,
}

fn parse_config() -> Result<Config, &'static str> {
    let matches = Command::new("Numeral Converter")
        .about("Convert between binary, octal, decimal, and hexadecimal numbers")
        .arg(Arg::new("number")
                 .short('n')
                 .long("number")
                 .help("Number value to convert"))
        .arg(Arg::new("numeral-type")
                 .short('t')
                 .long("numeral-type")
                 .help("Numeral system of provided number")
                 .value_parser(PathBufValueParser::default()))
        .get_matches();

    let default_num_type = PathBuf::from("decimal");
    let numeral_type: String = matches.get_one("numeral-type")
        .unwrap_or(&default_num_type)
        .display()
        .to_string();

    let number_str: Option<&String> = matches.get_one("number");
    match number_str {
        None => Err("Number argument required"),
        Some(n) => {
            let number = String::from(n);
            Ok(Config { number, numeral_type })
        }
    }
}

fn print_number_info<'a>(number: &'a str, numeral_info: &'a NumeralInfo) {
    let numeral_info: NumeralInfo = numeral_info.clone();
    let numeral_type: String = capitalize(numeral_info.name);

    println!("{} number {} (base {})\n", numeral_type, number, numeral_info.base);
}

fn capitalize<'a>(word: &'a str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str()
    }
}
