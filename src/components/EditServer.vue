<template>
    <v-dialog class="pa-0" v-model="openDialog" max-width="600" persistent>
        <v-card rounded="lg">
            <v-card-title class="d-flex justify-space-between align-center">
                <div>
                    <v-icon icon="mdi-pencil-box-multiple" size="small" />
                    修改服务器
                </div>
                <v-btn icon="mdi-close" size="small" density="comfortable" variant="text"
                    @click="openDialog = false; onDialogEvent(false, eventId);"></v-btn>
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
                        <v-text-field label="用户密码" v-model="server.password" :type="inputType"
                            :append-inner-icon="mdi_icon_eye" @click:append-inner="onViewPassowrd" />
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
                <v-btn text="取消" variant="elevated" @click="openDialog = false; onDialogEvent(false, eventId);"></v-btn>
                <v-btn text="确定" variant="elevated"
                    @click="openDialog = false; editServer(); onDialogEvent(false, eventId);"></v-btn>
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>
<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { ServerDetail } from '../utils/server';
import emitter from '../utils/emitter';

const { eventId, onEditServer, onDialogEvent, groups } = defineProps(['eventId', 'onEditServer', 'onDialogEvent', 'groups']);
const openDialog = ref(false);
const inputType = ref('password');
const mdi_icon_eye = ref('mdi-eye');
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

onMounted(() => {
    emitter.on<string>(`openEditServer_${eventId}`, (val) => {
        server.value = val as ServerDetail;
        openDialog.value = true;
        onDialogEvent(true);
    })
})

onUnmounted(() => {
    emitter.off(`openEditServer_${eventId}`);
})

function editServer() {
    onEditServer(eventId, server.value);
}

function onViewPassowrd() {
    if (inputType.value === 'password') {
        inputType.value = 'text';
        mdi_icon_eye.value = 'mdi-eye-off';
    } else {
        inputType.value = 'password';
        mdi_icon_eye.value = 'mdi-eye';
    }
}
</script>