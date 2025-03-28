<template>
    <v-app>
        <v-navigation-drawer rail rail-width="45" permanent>
            <template v-slot:default>
                <v-container>
                    <v-row align-content-sm="center" justify="center">
                        <v-btn class="included" icon="mdi-list-box-outline" density="comfortable" variant="text"
                            @click="drawer = !drawer" />
                    </v-row>
                    <v-row align-content-sm="center" justify="center">
                        <v-btn class="included" icon="mdi-plus-box-outline" density="comfortable" variant="text"
                            @click="openAddServer()" />
                        <AddServer :onAddServer="onAddServer" :onDialogEvent="onDialogEvent" :groups="groupNames" />
                    </v-row>
                    <v-row align-content-sm="center" justify="center">
                        <v-btn class="included" icon="mdi-swap-vertical-circle-outline" density="comfortable"
                            variant="text" @click="emitter.emit('openFileTransfer')" />
                        <FileTransfer :getCurrentServerId="onCurrentServerId" />
                    </v-row>
                </v-container>
            </template>
            <template v-slot:append>
                <v-container>
                    <v-row align-content-sm="center" justify="center">
                        <v-btn icon="mdi-cog-outline" density="comfortable" variant="text"
                            @click="emitter.emit('OpenSettings')" />
                        <Settings :onFontChanged="onFontChanged" />
                    </v-row>
                </v-container>
            </template>
        </v-navigation-drawer>
        <v-navigation-drawer v-model="drawer" width="250" permanent>
            <div v-click-outside="{
                handler: hideServersPanel,
                include
            }">
                <v-list v-model:opened="expandList" density="compact">
                    <v-list-group class="list_header_padding" v-for="item in serverGroups" :key="item.name"
                        :value="item.name" :title="item.name">
                        <template v-slot:activator="{ props }">
                            <v-list-item v-bind="props" />
                        </template>
                        <v-list-item class="list_item_padding" v-for="server in item.servers" :key="server.id">
                            <template v-slot:title>
                                <v-btn :id="server.id" @click="openTab(server)" class="justify-start text-subtitle-2"
                                    prepend-icon="mdi-network-outline" @mouseover="server.active = true" variant="flat"
                                    @mouseleave="onMouseleave() ? server.active = false : server.active = true"
                                    size="small" block>
                                    {{ server.name }}
                                    <div class="position-absolute right-0 pr-1" v-if="server.active">
                                        <v-btn icon="mdi-pencil-box-outline" size="small" density="compact"
                                            variant="text" @click.stop="onGetEditServer(server.id)" />
                                        <EditServer class="included" :onEditServer="onEditServer"
                                            :onDialogEvent="onDialogEvent" :eventId="server.id" :groups="groupNames" />
                                        <v-btn icon="mdi-delete-outline" size="small" density="compact" variant="text"
                                            @click.stop="delServer(server)" />
                                    </div>
                                </v-btn>
                            </template>
                        </v-list-item>
                    </v-list-group>
                </v-list>
            </div>
        </v-navigation-drawer>
        <v-app-bar scroll-behavior="elevate" height="50" density="compact" :color="appbarColor">
            <v-tabs :model-value="tab" @update:model-value="updateModelValue($event as string)" show-arrows>
                <v-tab class="text-subtitle-2" v-for="item in tabs" :key="item.id" :value="item.id"
                    :text="item.server.name" prepend-icon="mdi-network-outline">
                    <template v-slot:append>
                        <v-btn icon="mdi-close" size="x-small" density="comfortable" variant="text"
                            @click="closeTab(item.id)">
                        </v-btn>
                    </template>
                </v-tab>
            </v-tabs>
        </v-app-bar>
        <v-main v-if="tabs.length === 0">
        </v-main>
        <v-main class="h-screen" v-else>
            <v-tabs-window :model-value="tab" class="h-100">
                <v-tabs-window-item v-for="item in tabs" :key="item.id" :value="item.id" class="h-100">
                    <Terminal :tid="item.id" :sid="item.server.id" :select="tab" :closeTab="closeTab"
                        :fontFamily="fontFamily" class="h-100" />
                </v-tabs-window-item>
            </v-tabs-window>
        </v-main>
    </v-app>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, computed } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useTheme } from 'vuetify';
import Terminal from '../components/Terminal.vue';
import AddServer from '../components/AddServer.vue';
import EditServer from '../components/EditServer.vue';
import FileTransfer from '../components/FileTransfer.vue';
import { ServerDetail, ServerItem, ServerGroup, ServerMgr, TerminalItem, ID_CFG_EXPLST, ID_CFG_S_DGRP } from '../utils/server';
import emitter from '../utils/emitter';
import { UnlistenFn, TauriEvent } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import Settings from '../components/Settings.vue';

const drawer = ref(false);
const tab = ref<string | null>(null);
const tabs = ref<Array<TerminalItem>>([]);
const theme = useTheme();
const appbarColor = ref<string>(theme.current.value.colors.background);
const serverGroups = ref<Array<ServerGroup> | null>(null);
const defGroup = ref<string>('Default');
const expandList = ref<Array<string>>([]);
const serverMgr = new ServerMgr();
const currentwindow = getCurrentWindow();
const fontFamily = ref<string>('DejaVuSansMono Nerd Font Mono');

let mouseleave_exclude_click = false;
let dialogOpened = false;
let terminalId = 10001;
let unlistenDrag: UnlistenFn;
let unlistenEvent: UnlistenFn;

currentwindow.listen(TauriEvent.DRAG_DROP, (event: { payload: { paths: string[] } }) => {
    if (event.payload.paths.length > 0) {
        emitter.emit('openFileTransfer', event.payload.paths[0]);
    }
}).then((unlisten) => {
    unlistenDrag = unlisten;
})

currentwindow.listen('tauri://FileTransferMessage', (event: { payload: { rate: number, message: string } }) => {
    emitter.emit('FileTransferMessage', event.payload);
}).then((unlisten) => {
    unlistenEvent = unlisten;
})

onMounted(() => {
    serverMgr.getServerConfig().then((config) => {
        fontFamily.value = config.font_name;
        expandList.value = config.expand_list;
        defGroup.value = config.server_group;
        emitter.emit('FileTransferePathChanged', { local: config.local_path, remote: config.remote_path, file: config.file_name });
        emitter.emit('FileTransfereGroupChanged', { local: config.local_grps, remote: config.remote_grps, files: config.file_grps });
        emitter.emit('SettingsChanged', { proxy_addr: config.proxy_addr, font_name: config.font_name });
    });

    serverMgr.getServerList().then((servers) => {
        serverGroups.value = servers;
    });
})

onUnmounted(() => {
    if (unlistenDrag !== undefined && unlistenDrag !== null) {
        unlistenDrag();
    }
    if (unlistenEvent !== undefined && unlistenEvent !== null) {
        unlistenEvent();
    }
})

function updateModelValue(value: string) {
    if (value !== undefined) {
        tab.value = value;
    }
}

function openAddServer() {
    if (!drawer.value) {
        drawer.value = true;
    }
    emitter.emit('openAddServer', defGroup.value);
}

function onAddServer(server: ServerDetail) {
    defGroup.value = server.group;
    serverMgr.addServer(server).then(() => {
        serverMgr.getServerList().then((servers) => {
            serverGroups.value = servers;
        });
    }).catch((err) => {
        console.log('addServer error:', err);
    });
}

function delServer(server: ServerItem) {
    serverMgr.delServer(server.id).then(() => {
        serverMgr.getServerList().then((servers) => {
            serverGroups.value = servers;
        });
    }).catch((err) => {
        console.log('delServer error:', err);
    });
}

function onGetEditServer(id: string) {
    mouseleave_exclude_click = true;
    serverMgr.getServerDetail(id).then((server) => {
        emitter.emit(`openEditServer_${id}`, server);
    }).catch((err) => {
        console.log('getServerDetail error:', err);
    });
}

function onCurrentServerId() {
    if (tab.value === null || tab.value === undefined) {
        return;
    }

    const index = tabs.value.findIndex((item) => item.id === tab.value);
    if (index === -1) {
        return;
    }

    return tabs.value[index].server.id;
}

function onEditServer(id: string, server: ServerDetail) {
    defGroup.value = server.group;
    serverMgr.updateServer(id, server).then(() => {
        serverMgr.getServerList().then((servers) => {
            serverGroups.value = servers;
        });
    }).catch((err) => {
        console.log('updateServer error:', err);
    });
}

function onFontChanged(font_name: string) {
    fontFamily.value = font_name;
}

function openTab(item: ServerItem) {
    terminalId++;
    let tabId = terminalId.toString();
    tabs.value.push({ id: tabId, server: item });
    tab.value = tabId;
    drawer.value = false;
}

function closeTab(id: string) {
    if (tabs.value === null || tabs.value === undefined) {
        return;
    }

    const index = tabs.value.findIndex((item) => item.id === id);
    if (index === -1) {
        return;
    }

    // 判断关闭是不是当前激活的tab
    const td = tabs.value.splice(index, 1);
    if (td[0].id !== tab.value) {
        return;
    }

    if (tabs.value.length > 0) {
        if (index === tabs.value.length) {
            tab.value = tabs.value[index - 1].id;
        } else {
            tab.value = tabs.value[index].id;
        }
    }
}

function onMouseleave() {
    if (mouseleave_exclude_click) {
        mouseleave_exclude_click = false;
        return false;
    }
    return true;
}

watch(expandList, () => {
    setTimeout(() => {
        let value = JSON.stringify(expandList.value);
        invoke('ssh_set_config', { id: ID_CFG_EXPLST, value }).catch((err) => {
            console.log('ssh_set_config error:', err);
        })
    }, 500);
});

watch(tabs, tabsChanged, { deep: true });

function tabsChanged() {
    if (tabs.value.length === 0) {
        appbarColor.value = theme.current.value.colors.background;
    } else {
        appbarColor.value = "--v-theme-surface";
    }
}

watch(defGroup, (newVal, oldVal) => {
    if (oldVal === undefined || oldVal === null || oldVal.length === 0 || newVal === "Default") {
        return;
    }
    setTimeout(() => {
        invoke('ssh_set_config', { id: ID_CFG_S_DGRP, value: defGroup.value }).catch((err) => {
            console.log('ssh_set_config error:', err);
        })
    }, 500);
});

function hideServersPanel() {
    if (!dialogOpened) {
        drawer.value = false;
    }
}

function onDialogEvent(show: boolean, id?: string) {
    if (show) {
        dialogOpened = show;
    } else {
        setTimeout(() => {
            dialogOpened = show;
        }, 10)
    }

    if (show === true || id === undefined || id === null) {
        return;
    }

    if (serverGroups.value === null || serverGroups.value === undefined) {
        return;
    }

    serverGroups.value.forEach((item) => {
        item.servers.forEach((server) => {
            if (server.id === id) {
                server.active = false;
            }
        })
    })
}

function include() {
    return [document.querySelector('.included')]
}

const groupNames = computed(() => {
    if (serverGroups.value === null || serverGroups.value === undefined || serverGroups.value.length === 0) {
        return [defGroup.value];
    }
    return serverGroups.value.map((item) => item.name);
});
</script>

<style scoped>
.v-tab--selected {
    background-color: rgba(var(--v-theme-background));
}

.list_item_padding {
    padding-inline-start: 10px !important;
    padding-top: 0px !important;
    padding-bottom: 0px !important;
    padding-right: 10px !important;
    min-height: 28px !important;
}
</style>