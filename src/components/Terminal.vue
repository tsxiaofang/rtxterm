<template>
    <div :id="terminalId" style="padding-left:1px;"></div>
</template>

<script setup lang="ts">
import { onMounted, onBeforeUnmount, watch } from 'vue';
import { useTheme } from 'vuetify';
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit';
import { SSHClient, SSHMessage, CMD_DATA, CMD_CLOSE } from '../utils/ssh';
import { readText, writeText } from '@tauri-apps/plugin-clipboard-manager';
import '@xterm/xterm/css/xterm.css'
import emitter from '../utils/emitter';

const { tid, sid, select, closeTab, fontFamily } = defineProps(['tid', 'sid', 'select', 'closeTab', 'fontFamily']);

const theme = useTheme()

const xterm = new Terminal({
    windowOptions: {
        fullscreenWin: true,
    },
    //lineHeight: 1.2,
    fontSize: 15,
    fontWeight: 'normal',
    fontWeightBold: 'normal',
    //fontFamily: "Monaco, Menlo, Consolas, 'Courier New', monospace",
    fontFamily: fontFamily,
    theme: {
        background: theme.current.value.colors.background,
        foreground: '#C1C2C3',
    },
    // 光标闪烁
    cursorBlink: false,
    cursorStyle: 'block', // 'block' | 'underline' | 'bar';
    // scrollback: 0,
    // scrollback: 10000,
    // tabStopWidth: 4,
});
const fitAddon = new FitAddon();
let sshClient: SSHClient;
let terminalCtrl: HTMLElement;
const terminalId: string = `terminal_${tid}`;

const handleResize = () => {
    if (tid === select) {
        fitAddon.fit();
        setTimeout(() => {
            fitAddon.fit();
        }, 200);
    }
}

xterm.onResize(({ cols, rows }) => {
    if (sshClient !== undefined && terminalCtrl !== undefined) {
        let w = terminalCtrl.offsetWidth;
        let h = terminalCtrl.offsetHeight;
        sshClient.resize(cols, rows, w, h).catch((e) => {
            console.log('xterm resize ssh_send error:', e);
        });
        xterm.scrollToBottom();
    }
});

xterm.onSelectionChange(() => {
    let selText = xterm.getSelection();
    if (selText.length > 0) {
        writeText(selText).catch((e) => {
            console.log('xterm selection clipboard error:', e);
        });
    }
})

xterm.attachCustomKeyEventHandler((e: KeyboardEvent) => {
    if (!e.ctrlKey) {
        return true;
    }

    if (e.shiftKey) {
        if (e.key === 'c' || e.key === 'C') {
            let selText = xterm.getSelection();
            if (selText.length > 0) {
                writeText(selText).catch((e) => {
                    console.log('xterm selection clipboard error:', e);
                });
            }
            return false;
        } else if (e.key === 'v' || e.key === 'V') {
            readText().then((text) => {
                xterm.paste(text);
            })
            return false;
        }
    } else if (e.key === 'v' || e.key === 'V') {
        return false;
    }

    return true;
})

onMounted(() => {
    emitter.on<string>(terminalId, (val) => {
        let f = val as (tid: number) => void;
        if (sshClient !== undefined && sshClient !== null) {
            f(sshClient.channelId);
        }
    })
    terminalCtrl = document.getElementById(terminalId) as HTMLElement;

    xterm.open(terminalCtrl);
    xterm.loadAddon(fitAddon);
    // 去掉 Terminal 的滚动条
    (xterm as any)._core.viewport.scrollBarWidth = 0;

    window.addEventListener('resize', handleResize);
    SSHClient.connect(sid).then((client: SSHClient) => {
        sshClient = client;
        sshClient.addListener((msg: SSHMessage): void => {
            switch (msg.code) {
                case CMD_DATA: {
                    xterm.write(msg.data);
                    break;
                }
                case CMD_CLOSE: {
                    xterm.write('ssh connection closed');
                    closeTab(tid);
                    break;
                }
                default: {
                    console.log('unknown msg:', msg);
                }
            }
        });

        fitAddon.fit();

        let w = terminalCtrl.offsetWidth;
        let h = terminalCtrl.offsetHeight;

        sshClient.resize(xterm.cols, xterm.rows, w, h);
        xterm.focus();
    }).catch((e) => {
        console.log('ssh_connect error:', e);
        xterm.write(e);
    })

    setTimeout(() => {
        fitAddon.fit();
    }, 10);
})

xterm.onData(data => {
    sshClient.send(data).catch((e) => {
        console.log('xterm data ssh_send error:', e);
    });
})

onBeforeUnmount(() => {
    if (sshClient !== undefined) {
        sshClient.close().catch((e) => {
            console.log('ssh_close error:', e);
        })
    }
    emitter.off(terminalId);
    window.removeEventListener('resize', handleResize);
})

watch(() => select, () => {
    if (tid === select) {
        setTimeout(() => {
            fitAddon.fit();
            xterm.focus();
        }, 10)
    }
})
</script>
