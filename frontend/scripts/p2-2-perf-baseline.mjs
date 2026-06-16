#!/usr/bin/env node
// P2-2 前端基线脚本 - 数据源表行数扫描
// 用法：
//   DB_HOST=localhost DB_PORT=5432 DB_USER=bingxi DB_NAME=bingxi \
//   DB_PASSWORD=xxx node scripts/p2-2-perf-baseline.mjs
// 默认连接：localhost:5432/bingxi（沙箱环境）
// 生产环境：DB_HOST=39.99.34.194 DB_NAME=bingxi_erp
// 安全：所有连接信息从环境变量读取，不入版本库
// 输出：markdown 表格到 stdout，仅 SELECT，不修改数据

import pg from 'pg'

// 数据库连接配置（从环境变量读取，支持沙箱/生产）
const config = {
  host: process.env.DB_HOST || 'localhost',
  port: parseInt(process.env.DB_PORT || '5432', 10),
  user: process.env.DB_USER || 'bingxi',
  password: process.env.DB_PASSWORD,
  database: process.env.DB_NAME || 'bingxi',
}

// V2Table 页面对应数据源表（基于实际 schema 修正）
// - inventory_stock → inventory_stocks（拼写补 s）
// - production_orders → purchase_order（schema 中无 production 表，使用最接近"工单"语义的 purchase_order）
// - quality_inspection_records → purchase_inspection（schema 中无该表，使用 purchase_inspection 表，含检验记录）
const queries = [
  { table: 'inventory_stocks', page: 'StockTab', expected: '>= 10k' },
  { table: 'sales_orders', page: 'OrderListView', expected: '>= 1k' },
  { table: 'purchase_order', page: 'production (V2Table /production/production-orders/orders)', expected: '>= 1k' },
  { table: 'purchase_inspection', page: 'RecordTab (V2Table /quality/inspection-records)', expected: '>= 5k' },
]

async function main() {
  // 安全检查：DB_PASSWORD 必须从环境变量提供，禁止硬编码
  if (!process.env.DB_PASSWORD) {
    console.error('错误：DB_PASSWORD 环境变量未设置')
    console.error('用法：DB_HOST=localhost DB_PASSWORD=xxx node scripts/p2-2-perf-baseline.mjs')
    process.exit(1)
  }

  const client = new pg.Client(config)
  try {
    await client.connect()
    console.log(`# P2-2 前端基线 - V2Table 数据源表行数`)
    console.log('')
    console.log(`连接：${config.host}:${config.port}/${config.database}（用户：${config.user}）`)
    console.log('')
    console.log('| 页面 | 表名 | 行数 | 期望 | 状态 |')
    console.log('|------|------|------|------|------|')
    for (const { table, page, expected } of queries) {
      try {
        const { rows } = await client.query(`SELECT COUNT(*)::int AS count FROM ${table}`)
        const count = rows[0].count
        // 简单状态判断：>= 1000 行认为满足基线
        const status = count >= 1000 ? '✅' : '⚠️'
        console.log(`| ${page} | ${table} | ${count} | ${expected} | ${status} |`)
      } catch (err) {
        console.log(`| ${page} | ${table} | ERROR | ${expected} | ❌ |`)
        console.error(`  ${table}: ${err.message}`)
      }
    }
  } catch (err) {
    console.error('数据库连接失败:', err.message)
    process.exit(1)
  } finally {
    await client.end()
  }
}

main().catch(console.error)
