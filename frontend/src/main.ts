import { createApp } from 'vue'
import { createPinia } from 'pinia'
import ElementPlus from 'element-plus'
import 'element-plus/dist/index.css'
import zhCn from 'element-plus/es/locale/lang/zh-cn'
import en from 'element-plus/es/locale/lang/en'
import App from './App.vue'
import router from './router'
import { i18n, getCurrentLocale } from './i18n'

const app = createApp(App)

app.use(createPinia())
app.use(router)
app.use(i18n)

/* 根据当前语言切换 ElementPlus locale */
const elementLocale = getCurrentLocale() === 'en-US' ? en : zhCn
app.use(ElementPlus, { locale: elementLocale })

app.mount('#app')
