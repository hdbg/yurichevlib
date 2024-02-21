
    type TokioWsNoProxy = WebSocketStream<
        async_tungstenite::stream::Stream<
            TokioAdapter<tokio::net::TcpStream>,
            TokioAdapter<tokio_rustls::client::TlsStream<tokio::net::TcpStream>>,
        >,
    >;

    use std::sync::Arc;

    use async_tungstenite::{
        tokio::TokioAdapter,
        tungstenite::{self, Message},
        WebSocketStream,
    };
    use fast_socks5::client::Socks5Stream;
    use futures_util::{SinkExt, StreamExt};

    use snafu::ResultExt;
    use tokio::net::TcpStream;
    use tokio_rustls::TlsConnector;

    type TokioWsSocks5 = WebSocketStream<
        TokioAdapter<tokio_rustls::client::TlsStream<Socks5Stream<tokio::net::TcpStream>>>,
    >;

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
    }

    pub enum MaybeProxySocket {
        NoProxy(TokioWsNoProxy),
        Socks5(TokioWsSocks5),
    }
    impl MaybeProxySocket {
        pub async fn new_proxy(
            proxy: super::Socks5Proxy,
            domain: &'static str,
            port: u16,
            request: tungstenite::handshake::client::Request,
        ) -> Result<Self, SocketConnectError> {
            let proxy = connect_proxy(proxy, domain, port)
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
            }
        }
        pub async fn flush(&mut self) -> Result<(), async_tungstenite::tungstenite::Error> {
            match self {
                MaybeProxySocket::NoProxy(ws) => ws.flush().await,
                MaybeProxySocket::Socks5(ws) => ws.flush().await,
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
            }
        }
    }

  

    async fn connect_proxy(
        proxy: super::Socks5Proxy,
        domain: &'static str,
        port: u16,
    ) -> Result<fast_socks5::client::Socks5Stream<tokio::net::TcpStream>, fast_socks5::SocksError>
    {
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

    async fn connect_proxy_tls(
        domain: &'static str,
        proxy: Socks5Stream<TcpStream>,
    ) -> Result<tokio_rustls::client::TlsStream<Socks5Stream<TcpStream>>, std::io::Error> {
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
