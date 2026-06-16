#!/usr/bin/env node
// P2-2 前端基线脚本 - 数据源表行数扫描
// 用法：DB_PASSWORD=xxx node scripts/p2-2-perf-baseline.mjs
// 安全：DB 密码从环境变量 DB_PASSWORD 读取，不入版本库
// 输出：markdown 表格到 stdout，仅 SELECT，不修改数据

import pg from 'pg'

// 数据库连接配置（生产库 39.99.34.194:5432）
const config = {
  host: '39.99.34.194',
  port: 5432,
  user: 'bingxi',
  password: process.env.DB_PASSWORD,
  database: 'bingxi_erp',
}

// 4 V2Table 页面对应数据源表
const queries = [
  { table: 'inventory_stock', page: 'StockTab', expected: '>= 10k' },
  { table: 'sales_orders', page: 'OrderListView', expected: '>= 1k' },
  { table: 'production_orders', page: 'production', expected: '>= 1k' },
  { table: 'quality_inspection_records', page: 'RecordTab', expected: '>= 5k' },
]

async function main() {
  // 安全检查：DB_PASSWORD 必须从环境变量提供，禁止硬编码
  if (!process.env.DB_PASSWORD) {
    console.error('错误：DB_PASSWORD 环境变量未设置')
    console.error('用法：DB_PASSWORD=xxx node scripts/p2-2-perf-baseline.mjs')
    process.exit(1)
  }

  const client = new pg.Client(config)
  try {
    await client.connect()
    console.log('# P2-2 前端基线 - V2Table 数据源表行数')
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
