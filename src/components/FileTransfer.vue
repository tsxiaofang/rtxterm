<template>
    <v-dialog class="pa-0" v-model="openDialog" max-width="600" persistent>
        <v-card rounded="lg">
            <v-card-title class="d-flex justify-space-between align-center">
                <div>
                    <v-icon icon="mdi-swap-vertical-circle-outline" size="small" />
                    上传下载
                </div>
                <v-btn icon="mdi-close" size="small" density="comfortable" variant="text"
                    @click="openDialog = false;"></v-btn>
            </v-card-title>

            <v-divider />

            <v-card-text>
                <v-row class="pb-0 pt-2">
                    <v-combobox label="远程路径" v-model="remotePath" :items="remoteGroups" />
                </v-row>
                <v-row class="pb-0">
                    <v-combobox label="文件名称" v-model="fileName" :items="fileGroups" />
                </v-row>
                <v-row class="pb-0">
                    <v-combobox label="本地路径" v-model="localPath" :items="localGroups" />
                </v-row>
                <v-row>
                    <div>
                        {{ fileProgressInfo }}
                    </div>
                </v-row>
                <v-row>
                    <v-progress-linear v-model="fileProgress" color="green" height="25">
                        <template v-slot:default="{ value }">
                            <strong>{{ Math.ceil(value) }}%</strong>
                        </template>
                    </v-progress-linear>
                </v-row>
            </v-card-text>

            <v-divider />

            <v-card-actions class="d-flex pa-6 justify-end">
                <v-btn prepend-icon="mdi-upload" text="上传" @click="onUpload()"></v-btn>
                <v-btn prepend-icon="mdi-download" text="下载" @click="onDownload()"></v-btn>
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>
<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import emitter from '../utils/emitter';
import { ID_CFG_L_GRPS, ID_CFG_LOCAL, ID_CFG_R_GRPS, ID_CFG_REMOTE, ID_CFG_F_GRPS, ID_CFG_F_NAME } from '../utils/server';

const { getCurrentServerId } = defineProps(['getCurrentServerId']);
const openDialog = ref(false);
const fileProgress = ref<number>(0);
const fileProgressInfo = ref<string>(' ');
const localPath = ref<string>('');
const remotePath = ref<string>('');
const fileName = ref<string>('');
const localGroups = ref<Array<string>>([]);
const remoteGroups = ref<Array<string>>([]);
const fileGroups = ref<Array<string>>([]);
let transfering = false;
let watch_timer: number;

onMounted(() => {
    emitter.on<string>('openFileTransfer', (local_file) => {
        if (local_file !== undefined && local_file !== null) {
            localPath.value = local_file as string;
        }

        if (openDialog.value) {
            return;
        }

        if (!transfering) {
            fileProgress.value = 0;
            fileProgressInfo.value = ' ';
        }
        openDialog.value = true;
    })

    emitter.on<string>('FileTransferMessage', (info) => {
        const pi = info as { rate: number, message: string };
        fileProgress.value = pi.rate;
        fileProgressInfo.value = pi.message;
        if (pi.rate >= 100) {
            transfering = false;
        }
    })

    emitter.on<string>('FileTransferePathChanged', (info) => {
        const pt = info as { local: string, remote: string, file: string };
        localPath.value = pt.local;
        remotePath.value = pt.remote;
        fileName.value = pt.file;
    })

    emitter.on<string>('FileTransfereGroupChanged', (info) => {
        const gp = info as { local: Array<string>, remote: Array<string>, files: Array<string> };
        localGroups.value = gp.local;
        remoteGroups.value = gp.remote;
        fileGroups.value = gp.files;
    })
})

onUnmounted(() => {
    emitter.off('openFileTransfer');
    emitter.off('FileTransferMessage');
    emitter.off('FileTransferePathChanged');
})

function get_file_name(path: string) {
    var n = path.length;
    var file_name = path;
    if (n > 1 && path[n - 1] !== '/' && path[n - 1] !== '\\') {
        file_name += '/';
    }

    if (fileName.value.length > 1 && (fileName.value[0] === '/' || fileName.value[0] === '\\')) {
        file_name += fileName.value.substring(1);
    } else {
        file_name += fileName.value;
    }

    return file_name;
}

function onUpload() {
    let tid = getCurrentServerId();

    if (tid === undefined || tid === null) {
        fileProgressInfo.value = '请先打开一个服务器';
        return;
    }

    transfering = true;
    invoke('ssh_upload', {
        id: tid,
        localPath: get_file_name(localPath.value),
        remotePath: remotePath.value
    }).catch((e) => {
        transfering = false;
        fileProgressInfo.value = e.toString();
    });
}

function onDownload() {
    let tid = getCurrentServerId();

    if (tid === undefined || tid === null) {
        fileProgressInfo.value = '请先打开一个服务器';
        return;
    }

    transfering = true;
    fileProgress.value = 0;
    invoke<void>('ssh_download', {
        id: tid as number,
        localPath: localPath.value,
        remotePath: get_file_name(remotePath.value)
    }).catch((e) => {
        transfering = false;
        fileProgressInfo.value = e.toString();
    });
}

watch(localPath, (_newVal, oldVal) => {
    if (oldVal === undefined || oldVal.length === 0) {
        return;
    }
    if (watch_timer !== undefined) {
        clearTimeout(watch_timer);
    }
    watch_timer = setTimeout(() => {
        console.log('localPath', localPath.value);
        const index: number = (localGroups.value.findIndex((p) => p === localPath.value));
        if (index === -1) {
            if (localGroups.value.length >= 8) {
                localGroups.value.pop();
            }
        } else {
            // 移除当前路径
            localGroups.value.splice(index, 1);
        }
        // 从前面插入
        localGroups.value.unshift(localPath.value);
        invoke('ssh_set_config', { id: ID_CFG_LOCAL, value: localPath.value }).catch((e) => {
            console.log(e);
        });
    }, 500);
}, { immediate: true })

watch(remotePath, (_newVal, oldVal) => {
    if (oldVal === undefined || oldVal.length === 0) {
        return;
    }
    if (watch_timer !== undefined) {
        clearTimeout(watch_timer);
    }
    watch_timer = setTimeout(() => {
        const index: number = (remoteGroups.value.findIndex((p) => p === remotePath.value));
        if (index === -1) {
            if (remoteGroups.value.length >= 8) {
                remoteGroups.value.pop();
            }
        } else {
            // 移除当前路径
            remoteGroups.value.splice(index, 1);
        }
        // 从前面插入
        remoteGroups.value.unshift(remotePath.value);
        invoke('ssh_set_config', { id: ID_CFG_REMOTE, value: remotePath.value }).catch((e) => {
            console.log(e);
        });
    }, 500);
}, { immediate: true })

watch(fileName, (_newVal, oldVal) => {
    if (oldVal === undefined || oldVal.length === 0) {
        return;
    }
    if (watch_timer !== undefined) {
        clearTimeout(watch_timer);
    }
    watch_timer = setTimeout(() => {
        const index: number = (fileGroups.value.findIndex((p) => p === fileName.value));
        if (index === -1) {
            if (fileGroups.value.length >= 8) {
                fileGroups.value.pop();
            }
        } else {
            // 移除当前路径
            fileGroups.value.splice(index, 1);
        }
        // 从前面插入
        fileGroups.value.unshift(fileName.value);
        invoke('ssh_set_config', { id: ID_CFG_F_NAME, value: fileName.value }).catch((e) => {
            console.log(e);
        });
    }, 500);
}, { immediate: true })


watch(localGroups, (_newVal, oldVal) => {
    if (oldVal.length === 0) {
        return;
    }
    let value = JSON.stringify(localGroups.value);
    invoke('ssh_set_config', { id: ID_CFG_L_GRPS, value }).catch((e) => {
        console.log(e);
    });
}, { deep: true })

watch(remoteGroups, (_newVal, oldVal) => {
    if (oldVal.length === 0) {
        return;
    }
    let value = JSON.stringify(remoteGroups.value);
    invoke('ssh_set_config', { id: ID_CFG_R_GRPS, value }).catch((e) => {
        console.log(e);
    });
}, { deep: true })

watch(fileGroups, (_newVal, oldVal) => {
    if (oldVal.length === 0) {
        return;
    }
    let value = JSON.stringify(fileGroups.value);
    invoke('ssh_set_config', { id: ID_CFG_F_GRPS, value }).catch((e) => {
        console.log(e);
    });
}, { deep: true })

</script>