use el_platform_api::{
    NativeTransportFactory, NativeTransportStream, NetworkTarget, PlatformError, PlatformErrorKind,
    PlatformFuture, PlatformResult, TransportFactory, TransportReader, TransportStream,
    TransportWriter,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct TokioTcpTransportFactory;

pub struct TokioTcpTransport {
    stream: TcpStream,
}

impl TokioTcpTransport {
    pub fn new(stream: TcpStream) -> Self {
        Self { stream }
    }
}

pub struct TokioTcpReader {
    stream: ReadHalf<TcpStream>,
}

pub struct TokioTcpWriter {
    stream: WriteHalf<TcpStream>,
}

fn network_error(operation: &'static str, error: std::io::Error) -> PlatformError {
    PlatformError::new(
        PlatformErrorKind::Network,
        format!("TCP {operation} failed: {error}"),
    )
}

async fn connect_tcp(target: NetworkTarget) -> PlatformResult<TokioTcpTransport> {
    let host = target.host().to_owned();
    let stream = TcpStream::connect((host, target.port()))
        .await
        .map_err(|error| network_error("connect", error))?;
    stream
        .set_nodelay(true)
        .map_err(|error| network_error("set TCP_NODELAY", error))?;
    Ok(TokioTcpTransport::new(stream))
}

impl TransportFactory for TokioTcpTransportFactory {
    fn connect<'a>(
        &'a self,
        target: NetworkTarget,
    ) -> PlatformFuture<'a, PlatformResult<Box<dyn TransportStream>>> {
        Box::pin(
            async move { Ok(Box::new(connect_tcp(target).await?) as Box<dyn TransportStream>) },
        )
    }
}

impl NativeTransportFactory for TokioTcpTransportFactory {
    fn connect_native<'a>(
        &'a self,
        target: NetworkTarget,
    ) -> PlatformFuture<'a, PlatformResult<Box<dyn NativeTransportStream>>> {
        Box::pin(async move {
            Ok(Box::new(connect_tcp(target).await?) as Box<dyn NativeTransportStream>)
        })
    }
}

impl TransportReader for TokioTcpReader {
    fn read<'a>(
        &'a mut self,
        destination: &'a mut [u8],
    ) -> PlatformFuture<'a, PlatformResult<usize>> {
        Box::pin(async move {
            self.stream
                .read(destination)
                .await
                .map_err(|error| network_error("read", error))
        })
    }
}

impl TransportWriter for TokioTcpWriter {
    fn write_all<'a>(&'a mut self, source: &'a [u8]) -> PlatformFuture<'a, PlatformResult<()>> {
        Box::pin(async move {
            self.stream
                .write_all(source)
                .await
                .map_err(|error| network_error("write", error))
        })
    }

    fn close<'a>(&'a mut self) -> PlatformFuture<'a, PlatformResult<()>> {
        Box::pin(async move {
            self.stream
                .shutdown()
                .await
                .map_err(|error| network_error("shutdown", error))
        })
    }
}

impl TransportStream for TokioTcpTransport {
    fn into_split(self: Box<Self>) -> (Box<dyn TransportReader>, Box<dyn TransportWriter>) {
        let Self { stream } = *self;
        let (reader, writer) = tokio::io::split(stream);
        (
            Box::new(TokioTcpReader { stream: reader }),
            Box::new(TokioTcpWriter { stream: writer }),
        )
    }
}

impl NativeTransportStream for TokioTcpTransport {
    fn into_native_split(
        self: Box<Self>,
    ) -> (
        Box<dyn tokio::io::AsyncRead + Send + Unpin>,
        Box<dyn tokio::io::AsyncWrite + Send + Unpin>,
    ) {
        let Self { stream } = *self;
        let (reader, writer) = stream.into_split();
        (Box::new(reader), Box::new(writer))
    }
}
