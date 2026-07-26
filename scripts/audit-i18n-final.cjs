/**
 * 全量 i18n 接入审计脚本
 * - 扫描所有 .vue 文件的硬编码中文残留
 * - 验证 t()/$t() 调用的翻译键是否存在于 locales
 * - 统计接入率
 */
const fs = require('fs')
const path = require('path')

const FRONTEND_SRC = '/workspace/frontend/src'
const ZH_CN_PATH = path.join(FRONTEND_SRC, 'locales/zh-CN.ts')
const EN_US_PATH = path.join(FRONTEND_SRC, 'locales/en-US.ts')

// 读取 locales 文件并解析为对象
function loadLocale(filePath) {
  const content = fs.readFileSync(filePath, 'utf8')
  // 使用动态 require 加载 TS 文件 - 通过 eval 提取 export default
  const match = content.match(/export\s+default\s+({[\s\S]+})\s*;?\s*$/)
  if (!match) {
    throw new Error(`Cannot parse ${filePath}`)
  }
  // 用 Function 构造器加载（避免 TS 类型注解）
  const code = match[1]
  // eslint-disable-next-line no-new-func
  const obj = new Function(`return (${code})`)()
  return obj
}

// 检查翻译键是否存在
function keyExists(obj, keyPath) {
  const parts = keyPath.split('.')
  let cur = obj
  for (const p of parts) {
    if (cur == null || typeof cur !== 'object') return false
    if (!(p in cur)) return false
    cur = cur[p]
  }
  return true
}

// 收集所有 .vue 文件
function findVueFiles(dir) {
  const results = []
  const items = fs.readdirSync(dir, { withFileTypes: true })
  for (const item of items) {
    const full = path.join(dir, item.name)
    if (item.isDirectory()) {
      results.push(...findVueFiles(full))
    } else if (item.name.endsWith('.vue')) {
      results.push(full)
    }
  }
  return results
}

// 提取 Vue 文件中 script 和 template 部分的中文（注释除外）
function extractChinese(content) {
  const issues = []
  // 移除 <!-- --> HTML 注释
  let cleaned = content.replace(/<!--[\s\S]*?-->/g, '')
  // 移除 // 行注释（但要保留 URL 中的 //）
  cleaned = cleaned.replace(/(^|[^:])\/\/[^\n]*/g, '$1')
  // 移除 /* */ 块注释
  cleaned = cleaned.replace(/\/\*[\s\S]*?\*\//g, '')
  // 移除 <script> 标签内的 /// doc 注释
  cleaned = cleaned.replace(/\/\/\/[^\n]*/g, '')

  // 检查模板和 script 中的硬编码中文（排除 import 字符串）
  // 找出所有中文字符串（在引号或属性中）
  const chineseRegex = /['"`]([^'"`]*[\u4e00-\u9fa5]+[^'"`]*)['"`]/g
  let match
  while ((match = chineseRegex.exec(cleaned)) !== null) {
    const text = match[1]
    // 排除：t('xxx')/t("xxx") 调用本身（这些是 key 不是中文）
    // 排除：注释
    // 排除：URL/路径
    if (text.startsWith('http') || text.startsWith('/')) continue
    issues.push(text)
  }

  // 检查模板属性中的中文（如 placeholder="中文" / label="中文"）
  const attrRegex = /\s(?:placeholder|label|title|message|content|description|name|aria-label|sub-title|warning-text|tip)=["']([^"']*[\u4e00-\u9fa5]+[^"']*)["']/g
  while ((match = attrRegex.exec(cleaned)) !== null) {
    issues.push(`[attr] ${match[0].trim()}`)
  }

  // 检查模板标签内容中的硬编码中文（如 <el-button>中文</el-button>）
  // 这是最常见的硬编码情况
  const tagContentRegex = />([^<>]*[\u4e00-\u9fa5]+[^<>]*)</g
  while ((match = tagContentRegex.exec(cleaned)) !== null) {
    const text = match[1].trim()
    if (text && !text.includes('{{')) {
      issues.push(`[tag-content] ${text}`)
    }
  }

  return issues
}

// 检查 t()/$t() 调用的翻译键是否存在
function auditTKeys(content, file, locales, missingKeys) {
  const tCallRegex = /\bt\(\s*['"]([^'"]+)['"]/g
  const dollarTCallRegex = /\$t\(\s*['"]([^'"]+)['"]/g
  let match
  while ((match = tCallRegex.exec(content)) !== null) {
    const key = match[1]
    if (!keyExists(locales, key)) {
      if (!missingKeys.has(key)) missingKeys.set(key, [])
      missingKeys.get(key).push(file)
    }
  }
  while ((match = dollarTCallRegex.exec(content)) !== null) {
    const key = match[1]
    if (!keyExists(locales, key)) {
      if (!missingKeys.has(key)) missingKeys.set(key, [])
      missingKeys.get(key).push(file)
    }
  }
}

console.log('Loading locales...')
const zhCN = loadLocale(ZH_CN_PATH)
const enUS = loadLocale(EN_US_PATH)
console.log(`zh-CN loaded, top-level keys: ${Object.keys(zhCN).length}`)
console.log(`en-US loaded, top-level keys: ${Object.keys(enUS).length}`)

console.log('\nScanning all .vue files...')
const vueFiles = findVueFiles(FRONTEND_SRC)
console.log(`Total .vue files: ${vueFiles.length}`)

let filesWithI18n = 0
let filesWithoutI18n = 0
const filesNoI18n = []
const filesWithChinese = []
const missingKeys = new Map()

for (const file of vueFiles) {
  const content = fs.readFileSync(file, 'utf8')
  const relFile = path.relative(FRONTEND_SRC, file)

  if (content.includes('useI18n')) {
    filesWithI18n++
  } else {
    filesWithoutI18n++
    filesNoI18n.push(relFile)
  }

  // 审计翻译键
  auditTKeys(content, relFile, zhCN, missingKeys)

  // 检查硬编码中文
  const chineseIssues = extractChinese(content)
  if (chineseIssues.length > 0) {
    filesWithChinese.push({ file: relFile, issues: chineseIssues.slice(0, 5), total: chineseIssues.length })
  }
}

console.log(`\n=== 接入率统计 ===`)
console.log(`含 useI18n: ${filesWithI18n}/${vueFiles.length} (${(filesWithI18n/vueFiles.length*100).toFixed(1)}%)`)
console.log(`无 useI18n: ${filesWithoutI18n}/${vueFiles.length}`)

if (filesNoI18n.length > 0) {
  console.log(`\n无 useI18n 文件清单:`)
  filesNoI18n.forEach(f => console.log(`  - ${f}`))
}

console.log(`\n=== 翻译键缺失审计 ===`)
if (missingKeys.size === 0) {
  console.log('✅ 所有 t()/$t() 调用键均存在于 zh-CN.ts')
} else {
  console.log(`❌ 缺失 ${missingKeys.size} 个翻译键:`)
  for (const [key, files] of missingKeys) {
    console.log(`  - ${key} (used in: ${files.slice(0, 3).join(', ')}${files.length > 3 ? '...' : ''})`)
  }
}

console.log(`\n=== 硬编码中文残留检查 ===`)
if (filesWithChinese.length === 0) {
  console.log('✅ 无硬编码中文残留')
} else {
  console.log(`⚠️ 发现 ${filesWithChinese.length} 个文件含硬编码中文残留:`)
  filesWithChinese.slice(0, 30).forEach(({ file, issues, total }) => {
    console.log(`  - ${file} (${total} 处)`)
    issues.slice(0, 3).forEach(i => console.log(`      ${i}`))
  })
  if (filesWithChinese.length > 30) {
    console.log(`  ... 共 ${filesWithChinese.length} 个文件`)
  }
}
