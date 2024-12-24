import { invoke } from '@tauri-apps/api/core'

export const ID_CFG_LOCAL: number = 1;
export const ID_CFG_REMOTE: number = 2;
export const ID_CFG_EXPLST: number = 3;
export const ID_CFG_L_GRPS: number = 4;
export const ID_CFG_R_GRPS: number = 5;
export const ID_CFG_F_NAME: number = 6;
export const ID_CFG_F_GRPS: number = 7;

export interface ServerItem {
    id: string,
    name: string,
    active?: boolean,
}

export interface TerminalItem {
    id: string,
    server: ServerItem,
}

export interface ServerConfig {
    local_path: string,
    remote_path: string,
    file_name: string,
    font_name: string,
    expand_list: Array<string>,
    local_grps: Array<string>,
    remote_grps: Array<string>,
    file_grps: Array<string>,
}

export interface ServerDetail {
    name: string,
    group: string,
    host: string,
    port: number,
    username: string,
    password: string,
    cert_pass: string,
    cert_path: string,
    use_proxy: boolean,
}

export interface ServerGroup {
    name: string,
    servers: Array<ServerItem>,
}

export class ServerMgr {
    constructor() {
    }

    async getServerConfig(): Promise<ServerConfig> {
        return await invoke<ServerConfig>('ssh_config_all').then((cfg) => cfg);
    }

    async getServerList(): Promise<Array<ServerGroup>> {
        return await invoke<Array<ServerGroup>>('ssh_get_servers').then((servers) => servers);
    }

    async addServer(server: ServerDetail): Promise<void> {
        await invoke('ssh_add_server', { server });
    }

    async delServer(id: string): Promise<void> {
        await invoke('ssh_del_server', { id });
    }

    async getServerDetail(id: string): Promise<ServerDetail> {
        return await invoke<ServerDetail>('ssh_server_detail', { id });
    }

    async updateServer(id: string, server: ServerDetail): Promise<void> {
        await invoke('ssh_update_server', { id, server });
    }
}