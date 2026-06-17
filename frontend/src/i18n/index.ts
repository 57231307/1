/* vue-i18n 入口配置 - 冰溪 ERP (P4-4) */
import { createI18n } from 'vue-i18n'
import zhCN from '../locales/zh-CN'
import enUS from '../locales/en-US'

export type LocaleCode = 'zh-CN' | 'en-US'

/* 支持的语言 */
export const SUPPORTED_LOCALES: Array<{ code: LocaleCode; name: string; nativeName: string }> = [
  { code: 'zh-CN', name: 'Chinese (Simplified)', nativeName: '简体中文' },
  { code: 'en-US', name: 'English (US)', nativeName: 'English' },
]

/* 本地存储 key */
const STORAGE_KEY = 'bingxi.locale'

/* 从 localStorage 读取首选语言 */
function detectPreferredLocale(): LocaleCode {
  if (typeof window === 'undefined') return 'zh-CN'
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY)
    if (stored === 'zh-CN' || stored === 'en-US') {
      return stored as LocaleCode
    }
    /* 浏览器语言协商 */
    const browser = window.navigator.language
    if (browser.startsWith('en')) return 'en-US'
    if (browser.startsWith('zh')) return 'zh-CN'
  } catch (_e) {
    /* localStorage 可能被禁用 - 静默回退 */
  }
  return 'zh-CN'
}

/* 创建 i18n 实例 */
export const i18n = createI18n({
  legacy: false, // 使用 Composition API 模式
  globalInjection: true, // 全局 $t 注入
  locale: detectPreferredLocale(),
  fallbackLocale: 'zh-CN',
  messages: {
    'zh-CN': zhCN,
    'en-US': enUS,
  },
  /* 缺失 key 时回退到 fallbackLocale 而非控制台警告 */
  silentFallbackWarn: true,
  silentTranslationWarn: true,
})

/* 切换语言 */
export function setLocale(locale: LocaleCode): void {
  if (i18n.global.locale.value !== locale) {
    i18n.global.locale.value = locale
  }
  if (typeof window !== 'undefined') {
    try {
      window.localStorage.setItem(STORAGE_KEY, locale)
    } catch (_e) {
      /* 静默忽略 */
    }
    /* 同步更新 <html lang> 属性 */
    document.documentElement.lang = locale
  }
}

/* 获取当前语言 */
export function getCurrentLocale(): LocaleCode {
  return i18n.global.locale.value as LocaleCode
}
