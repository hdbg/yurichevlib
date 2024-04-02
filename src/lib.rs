use colored::{Color, Colorize};
use rand::{seq::SliceRandom, thread_rng, Rng};

#[cfg(feature = "duplex")]
pub mod duplex;

#[cfg(feature = "proxy_socket")]
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub enum ProxyKind {
    Socks5,
    Socks4,
    Http,
    Https,
}

#[cfg(feature = "proxy_socket")]
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct Proxy {
    pub kind: ProxyKind,
    pub addr: String,
    pub port: u16,
    pub creds: Option<(String, String)>,
}
#[cfg(feature = "proxy_socket")]
pub mod proxy_socket;

const SLOGANS: [&'static str; 4] = [
    "If not us, then who?",
    "Bad actor you can trust",
    "All your base belong to us",
    "Play with us, or lose the game",
];

pub fn print_logo() {
    let mut rng = thread_rng();

    let slogan = SLOGANS[rng.gen_range(0..SLOGANS.len())];
    let copyright = format!("{} (c) 2023", slogan);

    const LOGO: &str = include_str!("logo.txt");
    let logo_longest_line = LOGO.split("\n").map(|x| x.len()).max().unwrap();

    let colors = vec![
        Color::Red,
        Color::Yellow,
        Color::Blue,
        Color::Magenta,
        Color::Cyan,
    ];

    let current_color = colors.choose(&mut rng).unwrap();

    println!("{}", LOGO.color(current_color.clone()));
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
