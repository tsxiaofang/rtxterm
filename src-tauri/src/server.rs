use crate::{
    crypt::{load_server, save_server, verify_password},
    ssh::{into_essh, Error},
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, env, path::PathBuf};
use tauri::{async_runtime::Mutex, State};

const ID_CFG_LOCAL: u32 = 1;
const ID_CFG_REMOTE: u32 = 2;
const ID_CFG_EXPLST: u32 = 3;
const ID_CFG_L_GRPS: u32 = 4;
const ID_CFG_R_GRPS: u32 = 5;
const ID_CFG_F_NAME: u32 = 6;
const ID_CFG_F_GRPS: u32 = 7;

const SERVER_FILE: &str = "servers.json";
const CONFIG_FILE: &str = "config.json";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ServerItem {
    id: String,
    name: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ServerDetail {
    pub name: String,
    pub group: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub cert_pass: String,
    pub cert_path: String,
    pub use_proxy: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ServerGroup {
    name: String,
    servers: Vec<ServerItem>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub proxy_addr: String,
    #[serde(default = "Config::default_font_name")]
    pub font_name: String,
    #[serde(default)]
    pub file_name: String,
    pub local_path: String,
    pub remote_path: String,
    pub expand_list: Vec<String>,
    #[serde(default)]
    pub local_grps: Vec<String>,
    #[serde(default)]
    pub remote_grps: Vec<String>,
    #[serde(default)]
    pub file_grps: Vec<String>,
}

impl Config {
    fn default_font_name() -> String {
        String::from("DejaVuSansMono Nerd Font Mono")
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct ServerMgr {
    #[serde(skip)]
    pub config: Config,
    #[serde(skip)]
    pub app_path: PathBuf,
    #[serde(skip)]
    pub user_key: Vec<u8>,
    #[serde(skip)]
    pub data_key: Vec<u8>,
    pub servers: BTreeMap<u32, ServerDetail>,
}

impl ServerMgr {
    pub fn new() -> Self {
        let mut app_path = env::current_exe().unwrap_or_default();
        app_path.pop();

        let cfg_path = app_path.join(CONFIG_FILE);
        let json_str = std::fs::read_to_string(&cfg_path).unwrap_or_default();
        let config: Config = serde_json::from_str(&json_str).unwrap_or(Config {
            proxy_addr: String::default(),
            local_path: String::from("F:\\"),
            remote_path: String::from("/home"),
            expand_list: vec![String::from("Default")],
            font_name: Config::default_font_name(),
            ..Default::default()
        });

        Self {
            config,
            app_path,
            ..Default::default()
        }
    }

    pub fn save(&mut self) -> Result<()> {
        let file_name = self.app_path.join(SERVER_FILE);
        let servers = serde_json::to_string(self)?;
        save_server(&file_name, &self.user_key, &self.data_key, &servers)
    }

    pub fn save_config(&self) -> Result<()> {
        let cfg_path = self.app_path.join(CONFIG_FILE);
        let json_str = serde_json::to_string(&self.config)?;
        std::fs::write(cfg_path, json_str)?;
        Ok(())
    }
}

pub type ServerContext = Mutex<ServerMgr>;

#[tauri::command]
pub async fn ssh_login(
    name: String,
    password: String,
    stat: State<'_, ServerContext>,
) -> Result<(), Error> {
    let mut server_mgr = stat.lock().await;

    let file_name = server_mgr.app_path.join(SERVER_FILE);
    let (user_key, data_key) = verify_password(&file_name, &name, &password).map_err(into_essh)?;

    if file_name.exists() {
        let servers = load_server(&file_name, &user_key, &data_key)?;
        let mgr: ServerMgr = serde_json::from_str(&servers).map_err(into_essh)?;
        server_mgr.servers = mgr.servers;
    }

    server_mgr.user_key = user_key;
    server_mgr.data_key = data_key;
    Ok(())
}

#[tauri::command]
pub async fn ssh_get_servers(stat: State<'_, ServerContext>) -> Result<Vec<ServerGroup>, Error> {
    let mut group_map: BTreeMap<String, BTreeMap<String, ServerItem>> = BTreeMap::new();
    let server_mgr = stat.lock().await;

    for (k, v) in server_mgr.servers.iter() {
        let group = v.group.clone();
        if let Some(g) = group_map.get_mut(&group) {
            g.insert(
                v.name.clone(),
                ServerItem {
                    id: k.to_string(),
                    name: v.name.clone(),
                },
            );
        } else {
            group_map.insert(
                group,
                BTreeMap::from([(
                    v.name.clone(),
                    ServerItem {
                        id: k.to_string(),
                        name: v.name.clone(),
                    },
                )]),
            );
        }
    }

    Ok(group_map
        .into_iter()
        .map(|(k, v)| ServerGroup {
            name: k,
            servers: v.into_values().collect(),
        })
        .collect())
}

#[tauri::command]
pub async fn ssh_add_server(
    server: ServerDetail,
    stat: State<'_, ServerContext>,
) -> Result<(), Error> {
    let key = format!(
        "{}/{}",
        server.group.to_lowercase(),
        server.name.to_lowercase()
    );
    let id = crc32fast::hash(key.as_bytes());

    let mut server_mgr = stat.lock().await;

    if server_mgr.servers.contains_key(&id) {
        return Err(anyhow::anyhow!("server already exists:{key}").into());
    }

    server_mgr.servers.insert(id, server);
    server_mgr.save().map_err(into_essh)
}

#[tauri::command]
pub async fn ssh_del_server(id: String, stat: State<'_, ServerContext>) -> Result<(), Error> {
    let key = id.parse::<u32>().map_err(into_essh)?;
    let mut server_mgr = stat.lock().await;

    if server_mgr.servers.remove(&key).is_some() {
        server_mgr.save()?;
    }
    Ok(())
}

#[tauri::command]
pub async fn ssh_server_detail(
    id: String,
    stat: State<'_, ServerContext>,
) -> Result<ServerDetail, Error> {
    let key = id.parse::<u32>().map_err(into_essh)?;

    let server_mgr = stat.lock().await;

    let server = server_mgr
        .servers
        .get(&key)
        .ok_or(Error::from(anyhow::anyhow!("server not found:{id}")))?;

    Ok(server.clone())
}

#[tauri::command]
pub async fn ssh_update_server(
    id: String,
    server: ServerDetail,
    stat: State<'_, ServerContext>,
) -> Result<(), Error> {
    let key = format!(
        "{}/{}",
        server.group.to_lowercase(),
        server.name.to_lowercase()
    );
    let old_id = id.parse::<u32>().map_err(into_essh)?;
    let new_id = crc32fast::hash(key.as_bytes());

    let mut server_mgr = stat.lock().await;

    if old_id != new_id {
        if server_mgr.servers.contains_key(&new_id) {
            return Err(anyhow::anyhow!("server already exists:{key}").into());
        }

        server_mgr.servers.remove(&old_id);
    }

    server_mgr.servers.insert(new_id, server);
    server_mgr.save().map_err(into_essh)
}

#[tauri::command]
pub async fn ssh_config_all(stat: State<'_, ServerContext>) -> Result<Config, Error> {
    let server_mgr = stat.lock().await;
    Ok(server_mgr.config.clone())
}

#[tauri::command]
pub async fn ssh_set_config(
    id: u32,
    value: String,
    stat: State<'_, ServerContext>,
) -> Result<(), Error> {
    let mut server_mgr = stat.lock().await;
    match id {
        ID_CFG_LOCAL => server_mgr.config.local_path = value,
        ID_CFG_REMOTE => server_mgr.config.remote_path = value,
        ID_CFG_F_NAME => server_mgr.config.file_name = value,
        ID_CFG_L_GRPS => {
            server_mgr.config.local_grps = serde_json::from_str(&value).map_err(into_essh)?
        }
        ID_CFG_R_GRPS => {
            server_mgr.config.remote_grps = serde_json::from_str(&value).map_err(into_essh)?
        }
        ID_CFG_F_GRPS => {
            server_mgr.config.file_grps = serde_json::from_str(&value).map_err(into_essh)?
        }
        ID_CFG_EXPLST => {
            server_mgr.config.expand_list = serde_json::from_str(&value).map_err(into_essh)?
        }
        _ => return Err(into_essh(anyhow::anyhow!("invalid config id:{id}"))),
    }
    server_mgr.save_config().map_err(into_essh)?;
    Ok(())
}
