<template>
  <v-sheet class="align-content-center h-screen" @contextmenu="disabledContextMenu">
    <v-card class="mx-auto pa-6" variant="tonal" max-width="344">
      <v-form v-model="btn_enable" @submit.prevent="onSubmit">
        <v-text-field v-model="username" :readonly="loading" :rules="[required]" label="用户名称" clearable
          prepend-inner-icon="mdi-account-circle"></v-text-field>

        <v-text-field v-model="password" :readonly="loading" :rules="[required]" class="mb-2" label="密码"
          placeholder="请输入密码" autofocus clearable type="password" prepend-inner-icon="mdi-key"
          @update:focused="tip_errmsg = ''" @input="tip_errmsg = ''" :error-messages="tip_errmsg"></v-text-field>
        <v-btn :disabled="!btn_enable" prepend-icon="mdi-check-circle" :loading="loading" color="success" size="large"
          type="submit" variant="elevated" block>
          登录
        </v-btn>
      </v-form>
    </v-card>
  </v-sheet>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from "@tauri-apps/api/core";
import { disabledContextMenu } from '../utils/contextmenu';

const router = useRouter();
const username = ref('admin');
const password = ref('');

const tip_errmsg = ref("");
const btn_enable = ref(false);
const loading = ref(false);

function required(v: string) {
  return !!v || '必填项'
}

function onSubmit() {
  loading.value = true;
  invoke('ssh_login', { name: username.value, password: password.value }).then(() => {
    loading.value = false;
    router.replace("/main");
  }).catch((e) => {
    loading.value = false;
    tip_errmsg.value = e;
  })
}
</script>

<style>
/* 移除滚动条但允许滚动 */
::-webkit-scrollbar {
  display: none;
  width: 0;
  /* 对于WebKit浏览器 */
}
</style>