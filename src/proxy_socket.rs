type TokioWsNoProxy = WebSocketStream<
    async_tungstenite::stream::Stream<
        TokioAdapter<tokio::net::TcpStream>,
        TokioAdapter<tokio_rustls::client::TlsStream<tokio::net::TcpStream>>,
    >,
>;
use super::ProxyKind;

use std::sync::Arc;

use async_tungstenite::{
    tokio::TokioAdapter,
    tungstenite::{self, Message},
    WebSocketStream,
};
use fast_socks5::client::Socks5Stream;
use futures_util::{SinkExt, StreamExt};

use snafu::ResultExt;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use tokio_rustls::TlsConnector;

type TokioWsSocks5 = WebSocketStream<
    TokioAdapter<tokio_rustls::client::TlsStream<Socks5Stream<tokio::net::TcpStream>>>,
>;

type TokioWsHttp =
    WebSocketStream<TokioAdapter<tokio_rustls::client::TlsStream<tokio::net::TcpStream>>>;

#[derive(snafu::Snafu, Debug)]
pub enum SocketConnectError {
    #[snafu(display("fast_socks5 stream error"))]
    Socks5Error { source: fast_socks5::SocksError },

    #[snafu(display("tokio-rustls connection error"))]
    RustlsError { source: std::io::Error },

    #[snafu(display("async-tungstenite failed connecting"))]
    Tungstenite {
        source: async_tungstenite::tungstenite::Error,
    },

    #[snafu(display("http proxy failed connecting"))]
    AsyncHttp { source: async_http_proxy::HttpError },

    #[snafu(display("tokio failed connecting tunnel"))]
    Tokio { source: std::io::Error },

    #[snafu(display("proxy address parse failed"))]
    UrlParse { source: url::ParseError },

    #[snafu(display("proxy host is not present"))]
    HostIsNotPresent,
}

pub enum MaybeProxySocket {
    NoProxy(TokioWsNoProxy),
    Socks5(TokioWsSocks5),
    Http(TokioWsHttp),
}
impl MaybeProxySocket {
    pub async fn new_proxy(
        proxy: super::Proxy,
        domain: &'static str,
        port: u16,
        request: tungstenite::handshake::client::Request,
    ) -> Result<Self, SocketConnectError> {
        match &proxy.kind {
            ProxyKind::Socks5 => {
                let proxy = connect_socks5_proxy(proxy, domain, port)
                    .await
                    .context(Socks5Snafu)?;

                let tls = connect_proxy_tls(domain, proxy)
                    .await
                    .context(RustlsSnafu)?;

                let socket = async_tungstenite::tokio::client_async(request, tls)
                    .await
                    .context(TungsteniteSnafu)?;

                Ok(Self::Socks5(socket.0))
            }
            ProxyKind::Http => {
                let proxy = connect_http_proxy(proxy, domain, port).await?;
                let tls = connect_proxy_tls(domain, proxy)
                    .await
                    .context(RustlsSnafu)?;

                let socket = async_tungstenite::tokio::client_async(request, tls)
                    .await
                    .context(TungsteniteSnafu)?;

                Ok(Self::Http(socket.0))
            }
            _ => todo!(),
        }
    }

    pub async fn new(
        request: tungstenite::handshake::client::Request,
    ) -> Result<Self, SocketConnectError> {
        let socket = async_tungstenite::tokio::connect_async(request)
            .await
            .context(TungsteniteSnafu)?;

        Ok(Self::NoProxy(socket.0))
    }

    pub async fn send(
        &mut self,
        msg: Message,
    ) -> Result<(), async_tungstenite::tungstenite::Error> {
        match self {
            MaybeProxySocket::NoProxy(ws) => ws.send(msg).await,
            MaybeProxySocket::Socks5(ws) => ws.send(msg).await,
            MaybeProxySocket::Http(ws) => ws.send(msg).await,
        }
    }
    pub async fn flush(&mut self) -> Result<(), async_tungstenite::tungstenite::Error> {
        match self {
            MaybeProxySocket::NoProxy(ws) => ws.flush().await,
            MaybeProxySocket::Socks5(ws) => ws.flush().await,
            MaybeProxySocket::Http(ws) => ws.flush().await,
        }
    }

    pub async fn next(
        &mut self,
    ) -> Option<
        Result<async_tungstenite::tungstenite::Message, async_tungstenite::tungstenite::Error>,
    > {
        match self {
            MaybeProxySocket::NoProxy(ws) => ws.next().await,
            MaybeProxySocket::Socks5(ws) => ws.next().await,
            MaybeProxySocket::Http(ws) => ws.next().await,
        }
    }
}

use core::str::FromStr;
async fn connect_http_proxy(
    proxy: super::Proxy,
    domain: &'static str,
    port: u16,
) -> Result<tokio::net::TcpStream, SocketConnectError> {
    let addr = url::Url::from_str(&proxy.addr).context(UrlParseSnafu)?;

    let host = addr.host().ok_or(HostIsNotPresentSnafu.build())?;
    let host = match host {
        url::Host::Domain(domain) => {
            let resolved_ip = tokio::net::lookup_host(format!("{}:{}", domain.to_string(), port))
                .await
                .context(TokioSnafu)?
                .next()
                .ok_or(HostIsNotPresentSnafu.build())?;
            resolved_ip.to_string()
        }
        url::Host::Ipv4(addr) => addr.to_string(),
        _ => todo!(),
    };

    let mut stream = tokio::net::TcpStream::connect(format!("{}:{}", addr, proxy.port))
        .await
        .context(TokioSnafu)?;

    match proxy.creds {
        Some((login, password)) => async_http_proxy::http_connect_tokio_with_basic_auth(
            &mut stream,
            domain,
            port,
            &login,
            &password,
        )
        .await
        .context(AsyncHttpSnafu)?,

        None => async_http_proxy::http_connect_tokio(&mut stream, domain, port)
            .await
            .context(AsyncHttpSnafu)?,
    };

    Ok(stream)
}

async fn connect_socks5_proxy(
    proxy: super::Proxy,
    domain: &'static str,
    port: u16,
) -> Result<fast_socks5::client::Socks5Stream<tokio::net::TcpStream>, fast_socks5::SocksError> {
    let socks_ip = format!("{}:{}", proxy.addr, proxy.port);

    let socks;

    match proxy.creds {
        Some(creds) => {
            socks = Socks5Stream::connect_with_password(
                socks_ip,
                domain.to_owned(),
                port,
                creds.0,
                creds.1,
                fast_socks5::client::Config::default(),
            )
            .await?;
        }
        None => {
            socks = Socks5Stream::connect(
                socks_ip,
                domain.to_owned(),
                port,
                fast_socks5::client::Config::default(),
            )
            .await?;
        }
    }

    Ok(socks)
}

async fn connect_proxy_tls<T: AsyncRead + AsyncWrite + Unpin>(
    domain: &'static str,
    proxy: T,
) -> Result<tokio_rustls::client::TlsStream<T>, std::io::Error> {
    let mut root_store = rustls::RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(config));

    Ok(connector
        .connect(
            rustls_pki_types::ServerName::try_from(domain).unwrap(),
            proxy,
        )
        .await?)
}
