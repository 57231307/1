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
  {
    ts: 'VoucherEntry',
    rust: ['voucher_item:Model', 'VoucherItemDto', 'VoucherItemResponseDto'],
  },
  // 验布单建单/编辑入参：run 4623 的 54 用例暴露该页表单提交 fabric_batch_no/total_length_m
  // （后端两侧都无此列）又缺必填 inspection_date 被 422 拒掉。表单重建后把入参类型
  // 收进 api 层并纳入本对照表——同类漂移从此在提交前即被静态拦截，而不是等 E2E 撞。
  { ts: 'CreateFabricInspectionPayload', rust: ['CreateInspectionRequest'] },
  { ts: 'UpdateFabricInspectionPayload', rust: ['UpdateInspectionRequest'] },
  // 本轮/上轮逐字段核对过的三组契约一并纳管：库存台账出参（含 attach_master_names
  // 带出的主数据名称）与质检记录（前端类型曾按 record_no/result/inspector 等
  // 后端不存在的字段写死，导致列表四列恒空、建单必然 400）
  { ts: 'InventoryStock', rust: ['inventory_stock:Model', 'StockResponse'] },
  { ts: 'QualityRecord', rust: ['quality_inspection_record:Model'] },
  { ts: 'CreateQualityRecordPayload', rust: ['CreateInspectionRecordRequest'] },
  // 库存预警行：后端原先返回裸 json!（无类型，门禁看不见），本轮改为 StockAlertRow 后纳入对照
  { ts: 'StockAlert', rust: ['StockAlertRow'] },
  // 坯布（greige）入库/出库入参契约：本轮修复两处缺陷之一——前端 handleStock 曾提交
  // { quantity }，后端 stock_in 要求 StockInRequest{warehouse_id, weight_kg, length_m}、
  // stock_out 要求 StockOutRequest{weight_kg, length_m}（greige_fabric_handler.rs:110/123）
  // → 点击必 400、入库功能在 UI 上不可用。修复后前端对话框按契约提交，并把两组入参类型
  // 纳入本对照：同类漂移从此提交即被静态拦截，而非等 E2E 撞。
  // 注：响应型 GreigeFabric 暂不纳入——它是被 out-of-scope 的 /greige-fabrics 旧页共用的
  // 遗留类型（仍引用后端不存在的 fabric_code/supplier_name/quantity 等）。views/fabric 的
  // 列表列 prop 已直接绑到后端真实字段；待该旧页单独修复后再统一纳管 GreigeFabric。
  { ts: 'GreigeStockInPayload', rust: ['StockInRequest'] },
  { ts: 'GreigeStockOutPayload', rust: ['StockOutRequest'] },
];

// ---------- 存量幽灵字段挂账清单（允许通过，新增字段不在清单内立即拦截） ----------
// 修复一条删一条；新增契约漂移会直接 CI fail
//
// 2026-09-14 全部 13 项挂账已修复并移除（修复方式见 backend 对应文件）：
//   - Department.manager_name        → models/department.rs 加 #[sea_orm(ignore)] 字段，
//                                      department_service.rs list/get/create/tree 查 users 表填充
//   - Warehouse.contact_person       → migration m0053 加列 + Model/DTO/Service 接入
//   - Warehouse.is_default           → 同上（含默认仓库全局唯一互斥）
//   - DepartmentCreateRequest.code   → CreateDepartmentRequest 补字段，create 优先使用
//   - DepartmentCreateRequest.sort_order → 同上（默认 0）
//   - DepartmentUpdateRequest.sort_order → UpdateDepartmentRequest 补字段，update 接入
//   - VoucherEntry.subject_id / account_subject_id
//                                    → voucher_items 表迁移加 subject_id 列（finance 域）；
//                                      VoucherItemDto/VoucherItemRequest 补字段；
//                                      create/update 按 ID 反查科目补全 code/name 落库
//   - VoucherEntry.account_subject_code / account_subject_name
//                                    → VoucherItemDto 加 serde alias（请求）；响应侧
//                                      VoucherItemResponseDto 双命名序列化 + get_voucher
//                                      返回 entries（VoucherDetailResponse flatten）
//   - VoucherEntry.debit_amount / credit_amount / description
//                                    → 请求侧 serde alias；响应侧双命名冗余字段
// 所有字段均可在「TS 字段 vs Rust DTO/Model 字段存在性」上静态验证，无遗留挂账。
const ALLOWLIST = new Set([]);

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
