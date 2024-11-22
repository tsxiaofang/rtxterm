import { createRouter, createWebHistory } from 'vue-router'
import Main from '../views/Main.vue'
import Login from '../views/Login.vue'

const router = createRouter({
    history: createWebHistory(),
    routes: [
        {
            path: '/',
            redirect: '/login'
        },
        {
            path: '/login',
            component: Login
        },
        {
            path: '/main',
            component: Main
        }
    ]
})
export default router