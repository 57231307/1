/**
 * 简洁版 i18n 接入审计脚本 - 只输出文件清单
 */
const fs = require('fs')
const path = require('path')

const FRONTEND_SRC = '/workspace/frontend/src'
const ZH_CN_PATH = path.join(FRONTEND_SRC, 'locales/zh-CN.ts')

function loadLocale(filePath) {
  const content = fs.readFileSync(filePath, 'utf8')
  const match = content.match(/export\s+default\s+({[\s\S]+})\s*;?\s*$/)
  if (!match) throw new Error(`Cannot parse ${filePath}`)
  return new Function(`return (${match[1]})`)()
}

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

function findVueFiles(dir) {
  const results = []
  for (const item of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, item.name)
    if (item.isDirectory()) results.push(...findVueFiles(full))
    else if (item.name.endsWith('.vue')) results.push(full)
  }
  return results
}

function extractChinese(content) {
  const issues = []
  let cleaned = content.replace(/<!--[\s\S]*?-->/g, '')
  cleaned = cleaned.replace(/(^|[^:])\/\/[^\n]*/g, '$1')
  cleaned = cleaned.replace(/\/\*[\s\S]*?\*\//g, '')

  const chineseRegex = /['"`]([^'"`]*[\u4e00-\u9fa5]+[^'"`]*)['"`]/g
  let match
  while ((match = chineseRegex.exec(cleaned)) !== null) {
    const text = match[1]
    if (text.startsWith('http') || text.startsWith('/')) continue
    issues.push(text)
  }
  const attrRegex = /\s(?:placeholder|label|title|message|content|description|name|aria-label|sub-title|warning-text|tip)=["']([^"']*[\u4e00-\u9fa5]+[^"']*)["']/g
  while ((match = attrRegex.exec(cleaned)) !== null) {
    issues.push(`[attr] ${match[0].trim()}`)
  }
  const tagContentRegex = />([^<>]*[\u4e00-\u9fa5]+[^<>]*)</g
  while ((match = tagContentRegex.exec(cleaned)) !== null) {
    const text = match[1].trim()
    if (text && !text.includes('{{')) issues.push(`[tag] ${text}`)
  }
  return issues
}

const zhCN = loadLocale(ZH_CN_PATH)
const vueFiles = findVueFiles(FRONTEND_SRC)

let withI18n = 0
let withoutI18n = 0
const filesNoI18n = []
const filesWithChinese = []
const missingKeys = new Map()

for (const file of vueFiles) {
  const content = fs.readFileSync(file, 'utf8')
  const relFile = path.relative(FRONTEND_SRC, file)

  if (content.includes('useI18n')) withI18n++
  else { withoutI18n++; filesNoI18n.push(relFile) }

  // 缺失翻译键
  const tCallRegex = /\bt\(\s*['"]([^'"]+)['"]/g
  const dollarTCallRegex = /\$t\(\s*['"]([^'"]+)['"]/g
  let m
  while ((m = tCallRegex.exec(content)) !== null) {
    if (!keyExists(zhCN, m[1])) {
      if (!missingKeys.has(m[1])) missingKeys.set(m[1], [])
      missingKeys.get(m[1]).push(relFile)
    }
  }
  while ((m = dollarTCallRegex.exec(content)) !== null) {
    if (!keyExists(zhCN, m[1])) {
      if (!missingKeys.has(m[1])) missingKeys.set(m[1], [])
      missingKeys.get(m[1]).push(relFile)
    }
  }

  // 硬编码中文
  const issues = extractChinese(content)
  if (issues.length > 0) {
    filesWithChinese.push({ file: relFile, total: issues.length, sample: issues.slice(0, 3) })
  }
}

console.log(`=== 接入率 ===`)
console.log(`含 useI18n: ${withI18n}/${vueFiles.length} (${(withI18n/vueFiles.length*100).toFixed(1)}%)`)
console.log(`无 useI18n: ${withoutI18n}/${vueFiles.length}`)

console.log(`\n=== 缺失翻译键 (${missingKeys.size} 个) ===`)
for (const [key, files] of missingKeys) {
  console.log(`${key}\t${files[0]}`)
}

console.log(`\n=== 硬编码中文残留 (${filesWithChinese.length} 个文件) ===`)
console.log(`file\tcount\tsample`)
for (const { file, total, sample } of filesWithChinese) {
  console.log(`${file}\t${total}\t${sample[0] || ''}`)
}
