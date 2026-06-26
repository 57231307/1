import { createApp } from 'vue'
import { createPinia } from 'pinia'
import ElementPlus from 'element-plus'
import 'element-plus/dist/index.css'
import zhCn from 'element-plus/es/locale/lang/zh-cn'
import en from 'element-plus/es/locale/lang/en'
import App from './App.vue'
import router from './router'
import { i18n, getCurrentLocale } from './i18n'
import { permission, role } from './directives/permission'

const app = createApp(App)

app.use(createPinia())
app.use(router)
app.use(i18n)

/* FE-P-1 修复（2026-06-26 第二次审计第二优先级）：
 * 注册 v-permission 和 v-role 全局指令，使组件中的
 * `<el-button v-permission="'inventory:stock:edit'">` 等使用生效。
 * 原指令定义在 directives/permission.ts 但未在 main.ts 注册，
 * Vue 静默忽略 v-permission，按钮永远显示。 */
app.directive('permission', permission)
app.directive('role', role)

/* 根据当前语言切换 ElementPlus locale */
const elementLocale = getCurrentLocale() === 'en-US' ? en : zhCn
app.use(ElementPlus, { locale: elementLocale })

app.mount('#app')
