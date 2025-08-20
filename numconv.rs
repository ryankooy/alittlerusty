use std::{env, error, num};

/*
 * Binary, octal, and hexadecimal conversions.
 */

trait Number<'a> {
    fn new(number: &'a str, number_type: &'a str) -> Self;
    fn value(&self) -> Result<String, num::ParseIntError>;
    fn print(&self);
}

macro_rules! make_struct {
    ($name:ident, $numtype: expr, $base:expr, $fmtstr:expr) => {
        struct $name<'a> {
            number: &'a str,
            number_type: &'a str,
            input_number_type: &'a str,
            base: u32,
        }

        impl <'a> Number<'a> for $name<'a> {
            fn new(number: &'a str, number_type: &'a str) -> $name<'a> {
                let input_numtype = match number_type {
                    "dec" | "decimal" => "decimal",
                    "bin" | "binary" => "binary",
                    "oct" | "octal" => "octal",
                    "hex" | "hexadecimal" => "hexadecimal",
                    _ => "invalid"
                };

                $name {
                    number: number,
                    number_type: $numtype,
                    input_number_type: input_numtype,
                    base: $base,
                }
            }

            fn value(&self) -> Result<String, num::ParseIntError> {
                let base: u32 = match self.input_number_type {
                    "decimal" => 10,
                    "binary" => 2,
                    "octal" => 8,
                    "hexadecimal" => 16,
                    &_ => 0
                };

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
                let value: Result<String, num::ParseIntError> = self.value();
                match value {
                    Ok(value) => {
                        println!(
                            "{value}  <- Base {base} ({numtype}) of {inumtype} number {num}",
                            value=value.to_string(),
                            num=self.number,
                            numtype=self.number_type,
                            inumtype=self.input_number_type,
                            base=self.base
                        );
                    }
                    Err(e) => {
                        panic!(
                            "Invalid {inumtype} value: {error}",
                            inumtype=self.input_number_type,
                            error=e.to_string()
                        );
                    }
                }
            }
        }
    };
}

make_struct!(Decimal, "decimal", 10u32, "{}");
make_struct!(Binary, "binary", 2u32, "{:b}");
make_struct!(Octal, "octal", 8u32, "{:#o}");
make_struct!(Hex, "hexadecimal", 16u32, "{:#x}");

fn main() -> Result<(), Box<dyn error::Error>> {
    let args: Vec<String> = env::args().collect();
    let config = parse_config(&args)?;

    let number: &str = config.number.as_str();
    let number_type: &str = config.number_type.as_str();

    let decimal: Decimal = Number::new(&number, &number_type);
    let binary: Binary = Number::new(&number, &number_type);
    let octal: Octal = Number::new(&number, &number_type);
    let hex: Hex = Number::new(&number, &number_type);

    decimal.print();
    binary.print();
    octal.print();
    hex.print();

    Ok(())
}

struct Config {
    number: String,
    number_type: String,
}

fn parse_config(args: &[String]) -> Result<Config, &'static str> {
    if args.len() == 1 {
        return Err("Argument(s) required")
    }

    let number = args[1].clone();
    let mut number_type = String::from("decimal");

    if let Some(nt) = args.get(2) {
        number_type = nt.clone();
    }

    Ok(Config { number, number_type })
}
