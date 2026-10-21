pub mod protocol {
    use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
    use std::fmt;

    #[derive(Clone, Default, PartialEq, Eq)]
    pub struct UserKey {
        pub username: String,
        pub password: String,
    }

    impl UserKey {
        pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
            Self {
                username: username.into(),
                password: password.into(),
            }
        }
    }

    impl fmt::Debug for UserKey {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter
                .debug_struct("UserKey")
                .field("username", &"<redacted>")
                .field("password", &"<redacted>")
                .finish()
        }
    }

    impl fmt::Display for UserKey {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                formatter,
                "{}:{}",
                utf8_percent_encode(&self.username, NON_ALPHANUMERIC),
                utf8_percent_encode(&self.password, NON_ALPHANUMERIC),
            )
        }
    }
}

pub mod client {
    use crate::protocol::UserKey;
    use std::{io, net::SocketAddr};
    use tokio::{io::BufStream, net::TcpStream};
    use tokio_socks::tcp::Socks5Stream;

    pub async fn connect(
        stream: &mut BufStream<TcpStream>,
        address: &SocketAddr,
        authentication: Option<UserKey>,
    ) -> io::Result<()> {
        let socket = stream.get_mut();
        match authentication {
            Some(credentials) => {
                Socks5Stream::connect_with_password_and_socket(
                    socket,
                    *address,
                    &credentials.username,
                    &credentials.password,
                )
                .await
            }
            None => Socks5Stream::connect_with_socket(socket, *address).await,
        }
        .map(|connection| {
            let _ = connection.into_inner();
        })
        .map_err(io::Error::other)
    }
}
