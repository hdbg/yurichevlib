use std::str::FromStr;

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
impl FromStr for ProxyKind {
    type Err = ProxyParseError;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.to_ascii_lowercase().as_str() {
            "socks5" => Ok(Self::Socks5),
            "socks4" => Ok(Self::Socks4),
            "http" => Ok(Self::Http),
            "https" => Ok(Self::Https),

            _ => Err(InvalidProxyKindSnafu.build()),
        }
    }
}

#[cfg(feature = "proxy_socket")]
impl std::fmt::Display for ProxyKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_str(match self {
            Self::Socks4 => "socks4",
            Self::Socks5 => "socks5",
            Self::Http => "http",
            Self::Https => "https",
        })
    }
}

#[cfg(feature = "proxy_socket")]
impl std::fmt::Display for Proxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.kind.fmt(f)?;
        f.write_fmt(format_args!(":{}:{}", self.addr, self.port))?;

        if let Some(ref creds) = self.creds {
            f.write_fmt(format_args!(":{}:{}", creds.0, creds.1))?;
        }

        Ok(())
    }
}

#[cfg(feature = "proxy_socket")]
#[derive(Debug, snafu::Snafu)]
pub enum ProxyParseError {
    #[snafu(display("Verify correctness on proxy parts"))]
    InvalidChunkCount,

    #[snafu(display("Failed to parse port"))]
    InvalidPort,

    #[snafu(display("Failed to recognize proxy kind"))]
    InvalidProxyKind,
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
impl FromStr for Proxy {
    type Err = ProxyParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut input = input.split(":").collect::<Vec<_>>();

        if input.len() != 3 && input.len() != 5 {
            return Err(InvalidChunkCountSnafu.build());
        }

        let kind = ProxyKind::from_str(&input.remove(0))?;
        let addr = input.remove(0).to_string();
        let port: u16 = input
            .remove(0)
            .parse()
            .map_err(|_| InvalidProxyKindSnafu.build())?;

        let creds = {
            if input.is_empty() {
                None
            } else {
                let login = input.remove(0);
                let password = input.remove(0);

                Some((login.to_owned(), password.to_owned()))
            }
        };

        Ok(Self {
            kind,
            addr,
            port,
            creds,
        })
    }
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
