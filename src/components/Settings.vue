<template>
    <v-dialog class="pa-0" v-model="openDialog" max-width="600" persistent>
        <v-card rounded="lg">
            <v-card-title class="d-flex justify-space-between align-center">
                <div>
                    <v-icon icon="mdi-swap-vertical-circle-outline" size="small" />
                    系统配置
                </div>
                <v-btn icon="mdi-close" size="small" density="comfortable" variant="text"
                    @click="openDialog = false;"></v-btn>
            </v-card-title>

            <v-divider />

            <v-card-text class="px-3 py-3">
                <v-row class="pb-0 pt-6">
                    <v-text-field label="字体名称" v-model="fontName" />
                </v-row>

                <v-row class="pb-0 pt-0">
                    <v-text-field label="代理地址" v-model="proxyAddr" />
                </v-row>
            </v-card-text>

            <v-divider />

            <v-card-actions class="d-flex pl-5 pr-6">
                <v-btn text="取消" variant="elevated" @click="openDialog = false;"></v-btn>
                <v-btn text="确定" variant="elevated" @click="openDialog = false; onConfigChanged();"></v-btn>
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import emitter from '../utils/emitter';
import { ID_CFG_S_VALS } from '../utils/server';

const { onFontChanged } = defineProps(['onFontChanged']);
const openDialog = ref(false);
const proxyAddr = ref('');
const fontName = ref('');

onMounted(() => {
    emitter.on('OpenSettings', () => {
        openDialog.value = true;
    })
    emitter.on<string>('SettingsChanged', (info) => {
        const pt = info as { proxy_addr: string, font_name: string };
        proxyAddr.value = pt.proxy_addr;
        fontName.value = pt.font_name;
    })
})

onUnmounted(() => {
    emitter.off('OpenSettings');
    emitter.off('SettingsChanged');
})

function onConfigChanged() {
    onFontChanged(fontName.value);
    let value = JSON.stringify({ proxy_addr: proxyAddr.value, font_name: fontName.value });
    invoke('ssh_set_config', { id: ID_CFG_S_VALS, value }).catch((e) => {
        console.log(e);
    });
}

</script>