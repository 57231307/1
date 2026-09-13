#!/usr/bin/env node
/**
 * 前后端契约对齐检查（L7 防线）
 *
 * 背景：18 轮 CI 修复中多起缺陷源于 TS interface 与 Rust DTO 字段不对齐：
 *   - is_active/status（department：前端提交 status，后端 DTO 无字段被忽略）
 *   - real_name 幽灵字段（user 表无此列，前端 required 校验拦截 PUT 不发出）
 *   - credit_code 21/18 位
 * 本脚本静态比对核心资源的 TS interface 字段 vs Rust DTO/Model 字段，
 * 幽灵字段（TS 有 Rust 无）即 fail——把"上线后撞见"提前到"提交即拦截"。
 *
 * 规则：
 *   - snake_case 精确匹配；serde rename 改名、alias 增加别名，均已处理
 *   - TS 字段在 Rust 侧找不到（含别名）→ ERROR（提交会被 serde 忽略）
 *   - Rust 多出的字段不算错（后端可能返回前端未用的字段）
 */
import { readFileSync, readdirSync } from 'fs';
import { join, resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const FRONTEND = resolve(__dirname, '..');
const BACKEND = resolve(FRONTEND, '..', 'backend');

// ---------- TS 解析 ----------
function parseTsInterfaces(dir) {
  const out = {}; // name -> { field -> type }
  const walk = d => {
    for (const f of readdirSync(d, { withFileTypes: true })) {
      const p = join(d, f.name);
      if (f.isDirectory()) walk(p);
      else if (f.name.endsWith('.ts') && !f.name.endsWith('.d.ts')) {
        const src = readFileSync(p, 'utf-8');
        for (const m of src.matchAll(/export\s+interface\s+(\w+)\s*\{([^}]*)\}/g)) {
          const name = m[1];
          const fields = {};
          for (const line of m[2].split('\n')) {
            const fm = line.match(/^\s*([A-Za-z_]\w*)\??\s*:\s*([^;]+);?\s*$/);
            if (fm && !/^\s*\/\//.test(line)) fields[fm[1]] = fm[2].trim();
          }
          out[name] = { ...(out[name] ?? {}), ...fields };
        }
      }
    }
  };
  walk(join(dir, 'src', 'api'));
  return out;
}

// ---------- Rust 解析 ----------
function parseRustStructs(dir) {
  const out = {}; // name -> Set(acceptedFieldNames)
  const walk = d => {
    for (const f of readdirSync(d, { withFileTypes: true })) {
      const p = join(d, f.name);
      if (f.isDirectory()) walk(p);
      else if (f.name.endsWith('.rs')) {
        const src = readFileSync(p, 'utf-8');
        for (const m of src.matchAll(/pub\s+struct\s+(\w+)[^{]*\{([^}]*)\}/g)) {
          let name = m[1];
          // SeaORM DeriveEntityModel：models/xxx.rs 的 `pub struct Model` 按 `xxx.rs:Model` 注册
          if (name === 'Model' && /models\//.test(p.replace(/\\/g, '/'))) {
            name = f.name.replace('.rs', '') + ':Model';
          }
          const fields = new Set();
          let pendingRenames = []; // 当前字段前的 serde 属性
          for (const raw of m[2].split('\n')) {
            const line = raw.trim();
            const serde = line.match(/#\[\s*serde\s*\(([^)]*)\)\s*\]/);
            if (serde) {
              pendingRenames.push(serde[1]);
              continue;
            }
            const fm = line.match(/^pub\s+([A-Za-z_]\w*)\s*:/);
            if (fm) {
              fields.add(fm[1]);
              for (const attr of pendingRenames) {
                const rename = attr.match(/rename\s*=\s*"([^"]+)"/);
                if (rename) fields.add(rename[1]);
                for (const al of attr.matchAll(/alias\s*=\s*"([^"]+)"/g)) fields.add(al[1]);
              }
              pendingRenames = [];
            } else if (line && !line.startsWith('//') && !line.startsWith('#[')) {
              pendingRenames = [];
            }
          }
          if (out[name]) for (const f2 of fields) out[name].add(f2);
          else out[name] = fields;
        }
      }
    }
  };
  walk(join(dir, 'src', 'handlers'));
  walk(join(dir, 'src', 'models'));
  walk(join(dir, 'src', 'services'));
  return out;
}

// ---------- 对照表：TS interface → Rust struct（允许多个候选取并集） ----------
const MAPPING = [
  // Response 型：前端读接口 vs 后端 Model + 树/包装节点并集
  { ts: 'Department', rust: ['department:Model', 'DepartmentTreeNode'] },
  {
    ts: 'Warehouse',
    rust: ['warehouse:Model', 'CreateWarehouseRequest', 'UpdateWarehouseRequest'],
  },
  { ts: 'DepartmentCreateRequest', rust: ['CreateDepartmentRequest'] },
  { ts: 'DepartmentUpdateRequest', rust: ['UpdateDepartmentRequest'] },
  { ts: 'VoucherEntry', rust: ['voucher_item:Model', 'VoucherItemDto'] },
];

// ---------- 存量幽灵字段挂账清单（允许通过，新增字段不在清单内立即拦截） ----------
// 修复一条删一条；新增契约漂移会直接 CI fail
const ALLOWLIST = new Set([
  // Department：前端「负责人」列后端 Model/TreeNode 均无此字段——部门负责人列恒空
  'Department.manager_name',
  // Warehouse：contact_person/is_default 前端声明但后端不接收不返回
  'Warehouse.contact_person',
  'Warehouse.is_default',
  // Department 创建/更新：code/sort_order 提交被 serde 忽略（后端自动生成 code）
  'DepartmentCreateRequest.code',
  'DepartmentCreateRequest.sort_order',
  'DepartmentUpdateRequest.sort_order',
  // VoucherEntry：前端分录类型与后端 voucher_item:Model/VoucherItemDto 大面积漂移
  'VoucherEntry.subject_id',
  'VoucherEntry.account_subject_id',
  'VoucherEntry.account_subject_code',
  'VoucherEntry.account_subject_name',
  'VoucherEntry.debit_amount',
  'VoucherEntry.credit_amount',
  'VoucherEntry.description',
]);

const tsInterfaces = parseTsInterfaces(FRONTEND);
const rustStructs = parseRustStructs(BACKEND);

let errors = 0;
let checked = 0;
let allowlisted = 0;

for (const rule of MAPPING) {
  const tsIface = tsInterfaces[rule.ts];
  if (!tsIface) {
    console.error(`[contract] TS interface 未找到: ${rule.ts}`);
    errors++;
    continue;
  }
  // 收集所有候选 Rust struct 字段并集（Response/Dto 可能分散）
  let rustFields = new Set();
  let foundAny = false;
  for (const rn of rule.rust) {
    if (rustStructs[rn]) {
      foundAny = true;
      for (const f of rustStructs[rn]) rustFields.add(f);
    }
  }
  if (!foundAny) {
    console.error(`[contract] Rust struct 未找到: ${rule.rust.join('/')}（TS: ${rule.ts}）`);
    errors++;
    continue;
  }
  for (const [field, type] of Object.entries(tsIface)) {
    checked++;
    if (!rustFields.has(field)) {
      const key = `${rule.ts}.${field}`;
      if (ALLOWLIST.has(key)) {
        allowlisted++;
        continue;
      }
      console.error(
        `[contract] ❌ 幽灵字段: ${key}: ${type} — 前端声明但后端 ${rule.rust.join('/')} 不接收/不返回（serde 忽略）`
      );
      errors++;
    }
  }
}

console.log(
  `[contract] 检查完成: ${MAPPING.length} 组映射, ${checked} 字段, ${errors} 错误, ${allowlisted} 挂账`
);
if (errors > 0) {
  console.error(
    '[contract] 前后端契约不对齐——参考 department is_active/status 案例，幽灵字段会被后端 serde 静默忽略'
  );
  process.exit(1);
}
