import { request } from './request';
import type { ApiResponse } from '@/types/api';

/**
 * 公司档案——对外硬投影。
 *
 * 对齐后端 `GET /company-profile`（登录即可读，auth-only）的**硬投影**载荷 `{ name, short_name, logo }`：
 * 后端在该端点仅外显这三列（不含任何敏感列），前端据此收敛为三键。
 * 可空列按后端 Option 序列化写 `string | null`（键恒存在、值可为 null），
 * 非 NOT NULL 的 `name` 不标 `?`——那是掩盖缺键，本类型不采用。
 */
export interface CompanyProfilePublic {
  name: string;
  short_name: string | null;
  logo: string | null;
}

/**
 * 公司档案——全量列（管理端）。
 *
 * 对齐后端 `GET /company-profile/full`（`company-profile:read`）返回的全量列：
 * `id,name,short_name,credit_code,legal_representative,tax_registration_number,bank_name,bank_account,`
 * `taxpayer_type,phone,fax,email,website,address,registered_capital,establishment_date,logo,description,`
 * `updated_at,updated_by`。
 * 逐键 snake_case 与后端实体列名严格一致；`name` NOT NULL 不标 `?`，其余可空列写 `T | null`。
 * `registered_capital` 为 sea_orm `Decimal`，serde 序列化为**字符串**（本仓金额列统一按字符串处理，
 * 见 api/account-subject.ts 同款注释），故可空写 `string | null`；`establishment_date` 为 NaiveDate
 * 序列化为 ISO 日期文本，可空写 `string | null`；`updated_at` 为 TIMESTAMPTZ NOT NULL → `string`（不标可空）；
 * `updated_by` 为 i32 NOT NULL（0=播种后从未被编辑）→ `number`（不标可空）。
 */
export interface CompanyProfileFull {
  id: number;
  name: string;
  short_name: string | null;
  credit_code: string | null;
  legal_representative: string | null;
  tax_registration_number: string | null;
  bank_name: string | null;
  bank_account: string | null;
  taxpayer_type: string | null;
  phone: string | null;
  fax: string | null;
  email: string | null;
  website: string | null;
  address: string | null;
  registered_capital: string | null;
  establishment_date: string | null;
  logo: string | null;
  description: string | null;
  updated_at: string;
  updated_by: number;
}

/**
 * 公司档案——全量可编辑载荷（`PUT /company-profile/full`，`company-profile:update`）。
 *
 * 取 CompanyProfileFull 的可编辑子集：`id / updated_at / updated_by` 由后端维护，前端不提交，故不在载荷内。
 * `name` 必填（NOT NULL，无 `?`）；其余可空列写 `T | null`，空串/空白由 request.ts serializeParams 之外
 * 的 JSON body 原样上送（后端按列语义落库），前端不在此处做 `?? 默认` 掩盖缺键。
 * `logo` 为 base64 裸串（无 `data:` 前缀），本地已按 image/png + ≤512KB 校验，后端再校验 PNG 魔数。
 */
export interface CompanyProfileUpdatePayload {
  name: string;
  short_name: string | null;
  credit_code: string | null;
  legal_representative: string | null;
  tax_registration_number: string | null;
  bank_name: string | null;
  bank_account: string | null;
  taxpayer_type: string | null;
  phone: string | null;
  fax: string | null;
  email: string | null;
  website: string | null;
  address: string | null;
  registered_capital: string | null;
  establishment_date: string | null;
  logo: string | null;
  description: string | null;
}

/**
 * 读取公司档案硬投影（登录即可读，auth-only）。
 * @returns 硬投影载荷（name/short_name/logo）；失败经 request.ts 拦截器统一显式提示并 reject。
 */
export function getCompanyProfilePublic(): Promise<ApiResponse<CompanyProfilePublic>> {
  return request.get('/company-profile');
}

/**
 * 读取公司档案全量列（company-profile:read）。
 * @returns 全量列载荷；失败经 request.ts 拦截器统一显式提示并 reject。
 */
export function getCompanyProfileFull(): Promise<ApiResponse<CompanyProfileFull>> {
  return request.get('/company-profile/full');
}

/**
 * 更新公司档案全量列（company-profile:update）。
 * @param data - 全量可编辑载荷（name 必填）。
 * @returns 更新后的全量列载荷；失败经 request.ts 拦截器统一显式提示并 reject。
 */
export function updateCompanyProfile(
  data: CompanyProfileUpdatePayload
): Promise<ApiResponse<CompanyProfileFull>> {
  return request.put('/company-profile/full', data);
}
