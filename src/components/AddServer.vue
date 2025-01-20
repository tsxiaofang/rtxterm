<template>
    <v-dialog class="pa-0" v-model="openDialog" max-width="600" persistent>
        <v-card rounded="lg">
            <v-card-title class="d-flex justify-space-between align-center">

                <div>
                    <v-icon icon="mdi-server-plus" size="small" />
                    添加服务器
                </div>
                <v-btn icon="mdi-close" size="small" density="comfortable" variant="text"
                    @click="openDialog = false; onDialogEvent(false);"></v-btn>
            </v-card-title>

            <v-divider />

            <v-card-text class="px-3 py-3">
                <v-row>
                    <v-col class="pr-1 pt-5 pb-0" cols="7">
                        <v-text-field label="显示名称" v-model="server.name" />
                    </v-col>
                    <v-col class="pl-1 pt-5 pb-0" cols="5">
                        <v-combobox label="分租名称" v-model="server.group" :items="groups" />
                    </v-col>
                </v-row>
                <v-row>
                    <v-col class="pr-1 pt-0 pb-0" cols="7">
                        <v-text-field label="主机地址" v-model="server.host" />
                    </v-col>
                    <v-col class="pl-1 pt-0 pb-0" cols="5">
                        <v-text-field label="主机端口" v-model.number="server.port" type="number" min="1" max="65535" />
                    </v-col>
                </v-row>
                <v-row>
                    <v-col class="pr-1 pt-0 pb-0" cols="7">
                        <v-text-field label="用户名称" v-model="server.username" />
                    </v-col>
                    <v-col class="pl-1 pt-0 pb-0" cols="5">
                        <v-text-field label="用户密码" v-model="server.password" type="password" />
                    </v-col>
                </v-row>
                <v-row>
                    <v-col class="pr-1 pt-0 pb-0" cols="7">
                        <v-text-field label="私钥路径" v-model="server.cert_path" />
                    </v-col>
                    <v-col class="pl-1 pt-0 pb-0" cols="5">
                        <v-text-field label="私钥密码" v-model="server.cert_pass" type="password" />
                    </v-col>
                </v-row>
            </v-card-text>

            <v-divider />

            <v-card-actions class="d-flex pl-5 pr-6">
                <v-switch v-model="server.use_proxy" :label="`代理: ${server.use_proxy ? '开' : '关'}`"
                    density="compact"></v-switch>
                <v-spacer></v-spacer>
                <v-btn text="取消" variant="elevated" @click="openDialog = false; onDialogEvent(false);"></v-btn>
                <v-btn text="确定" variant="elevated"
                    @click="openDialog = false; addServer(); onDialogEvent(false);"></v-btn>
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>
<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { ServerDetail } from '../utils/server';
import emitter from '../utils/emitter';

const openDialog = ref(false);
const { onAddServer, onDialogEvent, groups } = defineProps(['onAddServer', 'onDialogEvent', 'groups']);
const server = ref<ServerDetail>({
    name: '',
    group: 'Default',
    host: '',
    port: 22,
    username: 'root',
    password: '',
    cert_pass: '',
    cert_path: '',
    use_proxy: false,
});

function addServer() {
    onAddServer(server.value);
    server.value.name = '';
    server.value.host = '';
    server.value.port = 22;
    server.value.username = 'root';
    server.value.password = '';
    server.value.cert_pass = '';
    server.value.cert_path = '';
}

onMounted(() => {
    emitter.on<string>('openAddServer', (info) => {
        server.value.group = info as string;
        openDialog.value = true;
        onDialogEvent(true);
    })
})

onUnmounted(() => {
    emitter.off('openAddServer');
})

</script>