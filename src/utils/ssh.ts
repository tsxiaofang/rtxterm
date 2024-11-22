import { invoke, Channel } from '@tauri-apps/api/core'

export const CMD_DATA: number = 0;
export const CMD_RESIZE: number = 1;
export const CMD_CLOSE: number = 2;

export interface SSHMessage {
    code: number,
    data: string
}

export class SSHClient {
    channelId: number;
    private readonly listeners: Array<(arg: SSHMessage) => void>
    constructor(id: number, listeners: Array<(arg: SSHMessage) => void>) {
        this.channelId = id
        this.listeners = listeners
    }

    static async connect(
        id: string,
    ): Promise<SSHClient> {
        const listeners: Array<(arg: SSHMessage) => void> = []
        const onMessage = new Channel<SSHMessage>()

        onMessage.onmessage = (message: SSHMessage): void => {
            listeners.forEach((l) => {
                l(message)
            })
        }

        return await invoke<number>('ssh_connect', { id, onMessage: onMessage }).then((cid) =>
            new SSHClient(cid, listeners))
    }

    addListener(cb: (arg: SSHMessage) => void): void {
        this.listeners.push(cb)
    }

    async send(data: string): Promise<void> {
        await invoke('ssh_send', { id: this.channelId, msg: { code: CMD_DATA, data } })
    }

    async resize(cols: number, rows: number, width: number, height: number): Promise<void> {
        let data = JSON.stringify({
            cols,
            rows,
            width,
            height,
        });
        await invoke('ssh_send', { id: this.channelId, msg: { code: CMD_RESIZE, data } })
    }

    async close(): Promise<void> {
        await invoke('ssh_close', { id: this.channelId })
    }
}