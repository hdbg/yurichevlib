use colored::Colorize;
use rand::{thread_rng, Rng};

const SLOGANS: [&'static str; 3] = [
    "If not us, then who?",
    "Malware you can trust",
    "All your base belong to us",
];

pub fn print_logo() {
    let mut rng = thread_rng();

    let slogan = SLOGANS[rng.gen_range(0..SLOGANS.len())];
    let copyright = format!("{} (c) 2023", slogan);

    const LOGO: &str = include_str!("logo.txt");
    let logo_longest_line = LOGO.split("\n").map(|x| x.len()).max().unwrap();

    println!("{}", LOGO.magenta());
    println!(
        "{}{}",
        " ".repeat(logo_longest_line - copyright.len()),
        copyright.bright_green()
    );
}

#[cfg(test)]
mod tests {

    #[test]
    fn should_compile() {
        super::print_logo();
    }
}
