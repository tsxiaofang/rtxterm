use anyhow::Result;
use async_ssh2_lite::{
    ssh2::ExtendedData,
    tokio::io::{AsyncReadExt, AsyncWriteExt},
    AsyncChannel, AsyncSession, TokioTcpStream,
};
use serde::{Deserialize, Serialize, Serializer};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};
use tauri::{async_runtime::Mutex, ipc::Channel, Runtime, State, Window};
use tokio::sync::mpsc;

static SSH_ID_MGR: AtomicU32 = AtomicU32::new(100);

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SshMessage {
    pub code: i32,
    pub data: String,
}

pub struct SshContext {
    _session: AsyncSession<TokioTcpStream>,
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

fn into_essh<E: Into<anyhow::Error>>(err: E) -> Error {
    Error(err.into())
}

#[tauri::command]
pub async fn ssh_connect<R: Runtime>(
    _window: Window<R>,
    name: String,
    on_message: Channel<serde_json::Value>,
    manager: State<'_, SShMgr>,
) -> Result<u32, Error> {
    let addr: SocketAddr = "192.168.6.142:22".parse().map_err(into_essh)?;
    let mut session = AsyncSession::connect(addr, None).await.map_err(into_essh)?;

    session.handshake().await.map_err(into_essh)?;

    session
        .userauth_password("root", "BxyVrv@Qnh5dnJ2QDE5MQo=")
        .await
        .map_err(into_essh)?;

    if !session.authenticated() {
        return Err(anyhow::anyhow!("not authenticated").into());
    }

    let (tx, mut rx) = mpsc::channel::<()>(10);

    let mut channel = session.channel_session().await.map_err(into_essh)?;
    channel
        .request_pty("xterm", None, None)
        .await
        .map_err(into_essh)?;
    channel.shell().await.map_err(into_essh)?;
    channel
        .handle_extended_data(ExtendedData::Merge)
        .await
        .map_err(into_essh)?;

    let id = SSH_ID_MGR.fetch_add(1, Ordering::Release);
    let mut stream = channel.stream(id as i32);

    tauri::async_runtime::spawn(async move {
        let mut tmp_vec = vec![0u8; 16 * 1024];
        let mut buf = tmp_vec.as_mut_slice();

        loop {
            tokio::select! {
                _ = rx.recv() => break,
                nr = stream.read(&mut buf) => {
                    let n = match nr {
                        Ok(n) => n,
                        Err(_) => break,
                    };

                    if n == 0 {
                        break;
                    }

                    let data = SshMessage {
                        code: 0,
                        data: String::from_utf8_lossy(&buf[..n]).to_string(),
                    };

                    let json_value = match serde_json::to_value(data) {
                        Ok(v) => v,
                        Err(_) => {
                            // log
                            break;
                        }
                    };

                    if let Err(_) = on_message.send(json_value) {
                        // log
                        break;
                    }
                }
            }
        }
        // 通知前端报错，链接断开
        // println!("ssh closed");
    });

    let mut l = manager.lock().await;
    l.insert(
        id,
        Arc::new(Mutex::new(SshContext {
            tx,
            _session: session,
            channel,
        })),
    );

    Ok(id)
}

#[tauri::command]
pub async fn ssh_send<R: Runtime>(
    _window: Window<R>,
    manager: State<'_, SShMgr>,
    id: u32,
    data: String,
) -> Result<(), Error> {
    let l1 = manager.lock().await;

    let ctx = l1
        .get(&id)
        .ok_or(anyhow::anyhow!("ssh context not found"))?
        .clone();

    drop(l1);

    let mut l2 = ctx.lock().await;

    l2.channel
        .write_all(data.as_bytes())
        .await
        .map_err(into_essh)?;
    Ok(())
}

#[tauri::command]
pub async fn ssh_close<R: Runtime>(
    _window: Window<R>,
    manager: State<'_, SShMgr>,
    id: u32,
) -> Result<(), Error> {
    let mut l = manager.lock().await;
    if let Some(v) = l.remove(&id) {
        v.lock().await.tx.send(()).await.ok();
    }
    Ok(())
}
