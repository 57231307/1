import { createApp } from 'vue';
import { createPinia } from 'pinia';
import piniaPluginPersistedstate from 'pinia-plugin-persistedstate';
import ElementPlus from 'element-plus';
import 'element-plus/dist/index.css';
// V15 P1-20-16 Element Plus 暗黑模式 CSS 变量（配合 html.dark 类切换）
import 'element-plus/theme-chalk/dark/css-vars.css';
import zhCn from 'element-plus/es/locale/lang/zh-cn';
import en from 'element-plus/es/locale/lang/en';
import App from './App.vue';
import router from './router';
import { i18n, getCurrentLocale } from './i18n';
import { permission } from './directives/permission';
// V15 P1-20-15 全局 CSS 变量主题（支持亮色/暗黑模式切换）
import './styles/theme.css';
// V15 P1-20-10 前端错误监控 SDK（自研轻量方案，监听 error + unhandledrejection + 5min 去重）
import { initMonitor } from './utils/monitor';

const app = createApp(App);

// FE-P2-1 修复（v12 前端复审）：注册全局错误处理，防止组件渲染异常和未捕获 Promise rejection 静默丢失
app.config.errorHandler = (err, _instance, info) => {
  console.error('[Vue 错误]', err, info);
};

window.addEventListener('unhandledrejection', event => {
  console.error('[未捕获 Promise]', event.reason);
});

// V15 P1-20-10 初始化前端错误监控（监听 error/unhandledrejection，best-effort 上报后端）
initMonitor();

const pinia = createPinia();
pinia.use(piniaPluginPersistedstate);
app.use(pinia);
app.use(router);
app.use(i18n);

/* FE-P-1 修复（2026-06-26 第二次审计第二优先级）：
 * 注册 v-permission 全局指令，使组件中的
 * `<el-button v-permission="'inventory:update'">` 等使用生效。
 * 权限码格式为两段式 `{resource}:{action}`（如 `inventory:update`、`inventory:delete`）。
 * P2 4-4 修复：原注释示例 `'inventory:stock:edit'` 为三段式，与权限码规范不符，修正为两段式。
 * 原指令定义在 directives/permission.ts 但未在 main.ts 注册，
 * Vue 静默忽略 v-permission，按钮永远显示。
 * V15 P2 B10-P2-6：删除 v-role 指令注册，统一使用 v-permission 权限码 */
app.directive('permission', permission);

/* 根据当前语言切换 ElementPlus locale */
const elementLocale = getCurrentLocale() === 'en-US' ? en : zhCn;
app.use(ElementPlus, { locale: elementLocale });

app.mount('#app');
