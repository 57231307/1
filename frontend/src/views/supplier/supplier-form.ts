/**
 * supplier-form.ts — 供应商表单模型单一定义
 *
 * 归属：views/supplier/**（SupplierDialog.vue 与 index.vue 共用）。
 * 契约：表单字段集合以后端 supplier::Model 的可编辑列为准（models/supplier.rs），
 * 父组件 index.vue 是表单数据的唯一权威源；emptySupplierFormData() 是"新建/重置"
 * 形态的单一事实来源，禁止在父子两处各写一份默认值（两份定义必然漂移）。
 */

export interface SupplierFormData {
  id: number | undefined;
  supplier_code: string;
  supplier_name: string;
  supplier_short_name: string;
  supplier_type: string;
  credit_code: string;
  registered_address: string;
  business_address: string;
  legal_representative: string;
  registered_capital: number;
  contact_phone: string;
  fax: string;
  website: string;
  contact_email: string;
  main_business: string;
  taxpayer_type: string;
  bank_name: string;
  bank_account: string;
  grade: string;
  status: string;
  remarks: string;
}

/** 新建/重置的空白表单（status 默认 active 与后端建单列默认对齐） */
export function emptySupplierFormData(): SupplierFormData {
  return {
    id: undefined,
    supplier_code: '',
    supplier_name: '',
    supplier_short_name: '',
    supplier_type: '',
    credit_code: '',
    registered_address: '',
    business_address: '',
    legal_representative: '',
    registered_capital: 0,
    contact_phone: '',
    fax: '',
    website: '',
    contact_email: '',
    main_business: '',
    taxpayer_type: '',
    bank_name: '',
    bank_account: '',
    grade: '',
    status: 'active',
    remarks: '',
  };
}
