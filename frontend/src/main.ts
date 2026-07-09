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

// FE-P2-1 修复（v12 前端复审）：注册全局错误处理，防止组件渲染异常和未捕获 Promise rejection 静默丢失
app.config.errorHandler = (err, _instance, info) => {
  console.error('[Vue 错误]', err, info)
}

window.addEventListener('unhandledrejection', (event) => {
  console.error('[未捕获 Promise]', event.reason)
})

app.use(createPinia())
app.use(router)
app.use(i18n)

/* FE-P-1 修复（2026-06-26 第二次审计第二优先级）：
 * 注册 v-permission 和 v-role 全局指令，使组件中的
 * `<el-button v-permission="'inventory:update'">` 等使用生效。
 * 权限码格式为两段式 `{resource}:{action}`（如 `inventory:update`、`inventory:delete`）。
 * P2 4-4 修复：原注释示例 `'inventory:stock:edit'` 为三段式，与权限码规范不符，修正为两段式。
 * 原指令定义在 directives/permission.ts 但未在 main.ts 注册，
 * Vue 静默忽略 v-permission，按钮永远显示。 */
app.directive('permission', permission)
app.directive('role', role)

/* 根据当前语言切换 ElementPlus locale */
const elementLocale = getCurrentLocale() === 'en-US' ? en : zhCn
app.use(ElementPlus, { locale: elementLocale })

app.mount('#app')
