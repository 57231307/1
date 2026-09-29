/**
 * 单据号前端自动生成工具
 *
 * 单据号统一由系统自动生成，表单输入框只读展示，禁止手动输入（防止重复）。
 * 格式：{前缀}{YYYYMMDD}{HHmmss}，秒级时间戳段保证同日并发不重复；
 * 生成后经后端 /document-no/check 查重，已存在则重新生成；
 * 提交后单据号由后端直接采用，数据库 UNIQUE 约束最终兜底。
 */
import { request } from '@/api/request';
import type { ApiResponse } from '@/types/api';

export function generateDocNo(prefix: string): string {
  const d = new Date();
  const ymd =
    `${d.getFullYear()}` +
    `${String(d.getMonth() + 1).padStart(2, '0')}` +
    `${String(d.getDate()).padStart(2, '0')}`;
  const hms =
    `${String(d.getHours()).padStart(2, '0')}` +
    `${String(d.getMinutes()).padStart(2, '0')}` +
    `${String(d.getSeconds()).padStart(2, '0')}`;
  return `${prefix}${ymd}${hms}`;
}

/**
 * 生成并校验唯一单据号：查重已存在则重试，重试耗尽追加随机后缀兜底
 * @param prefix 单据号前缀（如 OUT / DB / DR / PC / SC / LC / INV）
 * @param docType 单据类型（后端查重映射：outsourcing_order / dye_batch / dye_recipe /
 *                sales_contract / purchase_contract / labor_contract / finance_invoice）
 */
export async function generateUniqueDocNo(prefix: string, docType: string): Promise<string> {
  const MAX_RETRY = 5;
  for (let i = 0; i < MAX_RETRY; i++) {
    const no = generateDocNo(prefix);
    const res = await request.get<ApiResponse<boolean>>('/document-no/check', {
      params: { doc_type: docType, no },
    });
    if (!res.data) {
      return no;
    }
  }
  // 兜底：秒级号段冲突时追加两位随机数
  return `${generateDocNo(prefix)}${Math.floor(Math.random() * 90) + 10}`;
}
