use crate::server::{Config, ServerDetail};
use anyhow::Result;
use async_ssh2_lite::TokioTcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn ssh_proxy_connect(server: &ServerDetail, cfg: &Config) -> Result<TokioTcpStream> {
    let mut stream = TokioTcpStream::connect(&cfg.proxy_addr).await?;
    let req = format!(
        "CONNECT {}:{} HTTP/1.1\r\nHost: {0}\r\n\r\n",
        server.host, server.port
    );
    stream.write_all(req.as_bytes()).await?;

    let mut buf = [0; 1024];
    let _ = stream.read(&mut buf).await?;

    Ok(stream)
}
