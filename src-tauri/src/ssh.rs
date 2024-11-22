use crate::{
    proxy::ssh_proxy_connect,
    server::{Config, ServerContext, ServerDetail},
};
use anyhow::Result;
use async_ssh2_lite::{
    ssh2::ExtendedData, AsyncChannel, AsyncSession, SessionConfiguration, TokioTcpStream,
};
use futures_util::AsyncReadExt;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::json;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};
use tauri::{async_runtime::Mutex, ipc::Channel, State};
use tokio::{io::AsyncWriteExt, sync::mpsc};

static SSH_ID_MGR: AtomicU32 = AtomicU32::new(100);
const CMD_DATA: i32 = 0;
const CMD_RESIZE: i32 = 1;
const CMD_CLOSE: i32 = 2;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SshMessage {
    pub code: i32,
    pub data: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TerminalSize {
    cols: u32,
    rows: u32,
    width: u32,
    height: u32,
}

pub struct SshContext {
    //pub session: AsyncSession<TokioTcpStream>,
    channel: AsyncChannel<TokioTcpStream>,
    tx: mpsc::Sender<()>,
}

pub type SShMgr = Mutex<HashMap<u32, Arc<Mutex<SshContext>>>>;
pub struct Error(anyhow::Error);
impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Self(err)
    }
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.0.to_string().as_str())
    }
}

pub fn into_essh<E: Into<anyhow::Error>>(err: E) -> Error {
    Error(err.into())
}

pub async fn ssh_create_session(
    server: &ServerDetail,
    cfg: &Config,
) -> Result<AsyncSession<TokioTcpStream>> {
    let stream = if server.use_proxy && !cfg.proxy_addr.is_empty() {
        ssh_proxy_connect(server, cfg).await?
    } else {
        TokioTcpStream::connect((server.host.as_str(), server.port)).await?
    };

    let mut configuration = SessionConfiguration::new();
    configuration.set_timeout(15000);
    configuration.set_keepalive(true, 60);

    let mut session = AsyncSession::new(stream, Some(configuration))?;

    session.handshake().await?;

    if !server.cert_path.is_empty() {
        let privatekey = server.cert_path.as_ref();
        let passphrase = match server.cert_pass.is_empty() {
            true => None,
            false => Some(server.cert_pass.as_ref()),
        };
        session
            .userauth_pubkey_file(&server.username, None, privatekey, passphrase)
            .await?;
    } else {
        session
            .userauth_password(&server.username, &server.password)
            .await?;
    }

    if !session.authenticated() {
        anyhow::bail!("{} authenticated failed", server.username);
    }

    Ok(session)
}

// UTF-8编码的字节结构如下：
// ‌1字节‌：用于ASCII字符（0x00-0x7F），即只有一个字节，最高位为0
// ‌2字节‌：用于Unicode编码范围0x0800-0xFFFF的字符，字节结构为110x xxxx 10xxxxxx
// ‌3字节‌：用于Unicode编码范围0x10000-0x1FFFF的字符，字节结构为1110xxxx 10xxxxxx 10xxxxxx
// ‌4字节‌：用于Unicode编码范围0x200000-0x10FFFF的字符，字节结构为11110xxx 10xxxxxx 10xxxxxx 10xxxxxx‌
fn calc_utf8_remaining(data: &[u8]) -> usize {
    let mut n = 0;
    for (i, v) in data.iter().rev().enumerate() {
        // 2 字节
        if (*v & 0xC0) == 0xC0 && (*v & 0x3F) <= 0x1F {
            if i != 1 {
                n = i + 1;
            }
            break;
        }
        // 3 字节
        else if (*v & 0xE0) == 0xE0 && (*v & 0x1F) <= 0x0F {
            if i != 2 {
                n = i + 1;
            }
            break;
        }
        // 4 字节
        else if (*v & 0xF0) == 0xF0 && (*v & 0x0F) <= 0x07 {
            if i != 3 {
                n = i + 1;
            }
            break;
        } else if i >= 3 {
            break;
        }
    }
    n
}

#[tauri::command]
pub async fn ssh_connect(
    id: String,
    on_message: Channel<serde_json::Value>,
    ssh_mgr: State<'_, SShMgr>,
    svr_ctx: State<'_, ServerContext>,
) -> Result<u32, Error> {
    let id_key = id.parse::<u32>().map_err(into_essh)?;
    let lsm = svr_ctx.lock().await;
    let cfg = lsm.config.clone();
    let server = lsm
        .servers
        .get(&id_key)
        .ok_or(into_essh(anyhow::anyhow!("server not found")))?
        .clone();

    drop(lsm);

    let session = ssh_create_session(&server, &cfg).await?;

    let (tx, mut rx) = mpsc::channel::<()>(10);

    let mut channel = session.channel_session().await.map_err(into_essh)?;

    channel
        .handle_extended_data(ExtendedData::Merge)
        .await
        .map_err(into_essh)?;

    channel
        .request_pty("xterm", None, None)
        .await
        .map_err(into_essh)?;

    channel.shell().await.map_err(into_essh)?;

    let id = SSH_ID_MGR.fetch_add(1, Ordering::Release);
    let mut stream = channel.stream(0);

    tauri::async_runtime::spawn(async move {
        let mut tmp_vec = vec![0u8; 16 * 1024];
        let buf = tmp_vec.as_mut_slice();
        let mut idx = 0;

        loop {
            tokio::select! {
                _ = rx.recv() => break,
                nr = stream.read(&mut buf[idx..]) => {
                    let mut nlen = match nr {
                        Ok(v) => v,
                        Err(_) => break,
                    };

                    if nlen == 0 {
                        break;
                    }

                    nlen += idx;
                    let db = &mut buf[..nlen];

                    // 解决半个utf8字符的问题
                    idx = calc_utf8_remaining(db);
                    let (mut l, r) = db.split_at_mut(nlen - idx);

                    let dm = SshMessage {
                        code: CMD_DATA,
                        data: String::from_utf8_lossy(l).to_string(),
                    };

                    if idx > 0 {
                        std::io::copy(&mut &(*r), &mut l).ok();
                    }

                    let json_value = match serde_json::to_value(dm) {
                        Ok(v) => v,
                        Err(_) => {
                            // log
                            break;
                        }
                    };

                    if let Err(_e) = on_message.send(json_value) {
                        // log
                        break;
                    }
                }
            }
        }
        // 通知前端报错，链接断开
        on_message
            .send(json!({
                "code": CMD_CLOSE,
                "data": id,
            }))
            .ok();
    });

    let mut l = ssh_mgr.lock().await;
    l.insert(
        id,
        Arc::new(Mutex::new(SshContext {
            tx,
            //session,
            channel,
        })),
    );

    Ok(id)
}

#[tauri::command]
pub async fn ssh_send(stat: State<'_, SShMgr>, id: u32, msg: SshMessage) -> Result<(), Error> {
    let l1 = stat.lock().await;

    let ctx = l1
        .get(&id)
        .ok_or(anyhow::anyhow!("ssh context not found"))?
        .clone();

    drop(l1);

    let mut l2 = ctx.lock().await;

    match msg.code {
        CMD_DATA => {
            l2.channel
                .write_all(msg.data.as_bytes())
                .await
                .map_err(into_essh)?;
        }
        CMD_RESIZE => {
            let ts: TerminalSize = serde_json::from_str(&msg.data).map_err(into_essh)?;
            l2.channel
                .request_pty_size(ts.cols, ts.rows, Some(ts.width), Some(ts.height))
                .await
                .map_err(into_essh)?;
        }
        _ => {
            return Err(anyhow::anyhow!("unknown msg code:{}", msg.code).into());
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn ssh_close(stat: State<'_, SShMgr>, id: u32) -> Result<(), Error> {
    let mut l = stat.lock().await;
    if let Some(v) = l.remove(&id) {
        v.lock().await.tx.send(()).await.ok();
    }
    Ok(())
}
