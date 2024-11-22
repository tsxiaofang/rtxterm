use crate::{
    server::ServerContext,
    ssh::{into_essh, ssh_create_session, Error},
};
use anyhow::Result;
use async_ssh2_lite::{AsyncSftp, TokioTcpStream};
use futures_util::AsyncWriteExt;
use serde_json::json;
use std::{fs::Metadata, path::Path, time::Instant};
use tauri::{Emitter, State};
use tokio::io::AsyncReadExt;

pub const ENT_FTM: &str = "tauri://FileTransferMessage";

#[tauri::command]
pub async fn ssh_upload(
    id: String,
    local_path: String,
    remote_path: String,
    wnd: tauri::Window,
    svr_ctx: State<'_, ServerContext>,
) -> Result<(), Error> {
    //println!("ssh_upload id:{id}, local_path:{local_path}, remote_path:{remote_path}");
    let id_key = id.parse::<u32>().map_err(into_essh)?;
    let lsm = svr_ctx.lock().await;

    let cfg = lsm.config.clone();
    let server = lsm
        .servers
        .get(&id_key)
        .ok_or(into_essh(anyhow::anyhow!("server not found")))?
        .clone();

    drop(lsm);

    let json_d1 = json!({
        "rate": 0,
        "message": format!("正在链接:{}:{}", server.host, server.port),
    });
    wnd.emit(ENT_FTM, json_d1).ok();

    let session = ssh_create_session(&server, &cfg).await.map_err(into_essh)?;
    let sftp = session.sftp().await.map_err(into_essh)?;

    let json_d2 = json!({
        "rate": 0,
        "message": "链接成功",
    });
    wnd.emit(ENT_FTM, json_d2).ok();

    upload_files(&wnd, sftp, &local_path, &remote_path)
        .await
        .map_err(into_essh)?;
    Ok(())
}

async fn upload_files<P: AsRef<Path>>(
    wnd: &tauri::Window,
    sftp: AsyncSftp<TokioTcpStream>,
    local: P,
    remote: P,
) -> Result<()> {
    let local_path = local.as_ref();
    let remote_path = remote.as_ref();

    let ft = tokio::fs::metadata(local_path).await?;

    if ft.is_dir() {
        anyhow::bail!("local_path is dir");
    } else {
        upload_onefile(wnd, &sftp, local_path, remote_path, &ft).await?;
    }

    Ok(())
}

async fn upload_onefile(
    wnd: &tauri::Window,
    sftp: &AsyncSftp<TokioTcpStream>,
    local_path: &Path,
    remote_path: &Path,
    ft: &Metadata,
) -> Result<()> {
    let time = Instant::now();
    let file_name = local_path.file_name().unwrap_or_default().to_string_lossy();
    let remote_file = remote_path.join(file_name.as_ref());

    let mut src = tokio::fs::File::open(local_path).await?;
    let mut dst = sftp.create(&remote_file).await?;

    let mut buf = vec![0u8; 256 * 1024];
    let data = buf.as_mut_slice();
    let total_size = ft.len();
    let mut rate = 0;
    let mut now_size = 0;

    while let Ok(n) = src.read(data).await {
        if n == 0 {
            break;
        }

        now_size += n as u64;
        dst.write_all(&data[..n]).await?;

        if total_size == 0 {
            continue;
        }

        let rate_tmp = now_size * 100 / total_size;
        if rate_tmp == rate {
            continue;
        }

        rate = rate_tmp;
        let json_data = json!({
            "rate": rate_tmp,
            "message": file_name.to_string(),
        });
        wnd.emit(ENT_FTM, json_data).ok();
    }

    if now_size >= total_size {
        let json_data = json!({
            "rate": 100,
            "message": format!("{}, time:{} ms, size:{}", file_name, time.elapsed().as_millis(), total_size),
        });
        wnd.emit(ENT_FTM, json_data).ok();
    }
    Ok(())
}
