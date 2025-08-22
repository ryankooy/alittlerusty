use std::error::Error;
use std::num::ParseIntError;
use std::path::PathBuf;

use clap::{Arg, Command, builder::PathBufValueParser};

/*
 * Binary, octal, and hexadecimal conversions.
 */

#[derive(Clone)]
enum Numeral {
    Binary,
    Octal,
    Decimal,
    Hexadecimal,
    Invalid,
}

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
            _ => Numeral::Invalid,
        }
    }

    fn info(&self) -> Result<NumeralInfo, &'static str> {
        match &self {
            Numeral::Binary => Ok(NumeralInfo { name: "binary", base: 2 }),
            Numeral::Octal => Ok(NumeralInfo { name: "octal", base: 8 }),
            Numeral::Decimal => Ok(NumeralInfo { name: "decimal", base: 10 }),
            Numeral::Hexadecimal => Ok(NumeralInfo { name: "hexadecimal", base: 16 }),
            Numeral::Invalid => Err("Invalid numeral type"),
        }
    }
}

trait Number<'a> {
    fn new(number: &'a str, numeral_type: &Numeral) -> Self;
    fn value(&self, base: u32) -> Result<String, ParseIntError>;
    fn print(&self);
}

macro_rules! make_struct {
    ($name:ident, $numtype:expr, $fmtstr:expr) => {
        struct $name<'a> {
            number: &'a str,
            numeral_type: Numeral,
            input_numeral_type: Numeral,
        }

        impl <'a> Number<'a> for $name<'a> {
            fn new(number: &'a str, numeral_type: &Numeral) -> $name<'a> {
                $name {
                    number: number,
                    numeral_type: $numtype,
                    input_numeral_type: numeral_type.clone(),
                }
            }

            fn value(&self, base: u32) -> Result<String, ParseIntError> {
                match i128::from_str_radix(self.number, base) {
                    Ok(value) => {
                        Ok(format!($fmtstr, value))
                    }
                    Err(e) => {
                        Err(e)
                    }
                }
            }

            fn print(&self) {
                let numinfo = self.numeral_type.info().unwrap();
                let inuminfo = match self.input_numeral_type.info() {
                    Ok(info) => info,
                    Err(e) => {
                        panic!("{}", e);
                    },
                };

                match self.value(inuminfo.base) {
                    Ok(value) => {
                        println!(
                            "{value}  <- Base {base} ({numtype}) of {inumtype} number {num}",
                            value=value.to_string(),
                            num=self.number,
                            numtype=numinfo.name,
                            inumtype=inuminfo.name,
                            base=numinfo.base,
                        );
                    }
                    Err(e) => {
                        panic!(
                            "Invalid {inumtype} value: {error}",
                            inumtype=inuminfo.name,
                            error=e.to_string(),
                        );
                    }
                }
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

    let decimal: Decimal = Number::new(&number, &numeral_type);
    let binary: Binary = Number::new(&number, &numeral_type);
    let octal: Octal = Number::new(&number, &numeral_type);
    let hex: Hex = Number::new(&number, &numeral_type);

    decimal.print();
    binary.print();
    octal.print();
    hex.print();

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
        Some(s) => {
            let number = String::from(s);
            Ok(Config { number, numeral_type })
        }
    }
}
