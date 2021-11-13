use std::fs::File;
use std::io::{BufRead, BufReader};
use std::{thread, time};
use rand::Rng;

fn main() {
    let ignored_chars: Vec<&str> = vec![
        "ả", "á", "ú", "ū", "#", "ī", "-", "ç", "ı", "Å", "ü",
		"á", "š", "Þ", "ó", "ằ", "č", "í", "Ø", "ě", "İ", "ō",
		"í", "'", "ö", "é", "ž", "Ç", "ğ", "ý", "ê", "ł", "ā",
		"Š", "Ş", "É", "Ó", "ė", "â", "ï", "æ", "ð", "à", "Á",
		"ñ", "ř", "ē", "ă", "ø", "Ē", "À", "Ā"
    ];

    let file = File::open("./names.txt").unwrap();
    let file = BufReader::new(file);
    let lines: Vec<_> = file.lines().collect::<Result<_, _>>().unwrap();
    let wait = time::Duration::from_millis(500);

    let mut count: u16 = 0;
    while count < 100 {
        let num: usize = rand::thread_rng().gen_range(0..20000);
        let rand_name: Vec<&str> = lines[num].split("\t").collect();
        let name: &str = rand_name[0];
        let s: &str = rand_name[1];
        let mut is_ignored: bool = false;

        for chr in ignored_chars.iter() {
            if name.contains(chr) || name.len() > 8 {
                is_ignored = true;
                break;
            }
        }

        if !is_ignored {
            if count % 5 == 0 {
                println!("\n=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=\n");
            }

            let sex: &str = if s == "f" { "girl" } else { "boy" };
            let sex: &str = if s.len() == 2 { "either" } else { sex };

            println!("\t{:?}, {}", name, sex);

            thread::sleep(wait);
            count += 1;
        }
    }
}
