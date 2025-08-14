use std::env;

macro_rules! make_structs {
    ($name:ident) => {
        struct $name<'a> {
            name: &'a str,
            base: u32,
            number: &'a i128,
        }

        impl <'a> $name<'a> {
            fn name(&self) -> &str { self.name }
            fn base(&self) -> u32 { self.base }
            fn number(&self) -> &i128 { self.number }

            fn print(&self) {
                let value: String = self.value();
                println!(
                    "{value}  <- Base {base} ({name}) of {number}",
                    value=value.as_str(),
                    name=self.name(),
                    base=self.base(),
                    number=self.number()
                );
            }
        }
    };
}

make_structs!(Binary);
make_structs!(Octal);
make_structs!(Hex);

trait Number<'a> {
    fn new(number: &'a i128) -> Self;
    fn value(&self) -> String;
}

impl <'a> Number<'a> for Binary<'a> {
    fn new(number: &'a i128) -> Binary<'a> {
        Binary { name: "binary", base: 2u32, number: number }
    }

    fn value(&self) -> String {
        format!("{:b}", self.number)
    }
}

impl <'a> Number<'a> for Octal<'a> {
    fn new(number: &'a i128) -> Octal<'a> {
        Octal { name: "octal", base: 8u32, number: number }
    }

    fn value(&self) -> String {
        format!("{:o}", self.number)
    }
}

impl <'a> Number<'a> for Hex<'a> {
    fn new(number: &'a i128) -> Hex<'a> {
        Hex { name: "hexadecimal", base: 16u32, number: number }
    }

    fn value(&self) -> String {
        format!("{:x}", self.number)
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut num = 42i128;

    if args.len() == 2 {
        num = args[1].parse::<i128>().unwrap();
    }

    let binary: Binary = Number::new(&num);
    let octal: Octal = Number::new(&num);
    let hex: Hex = Number::new(&num);

    binary.print();
    octal.print();
    hex.print();
}
