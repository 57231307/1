import type { FormRules } from 'element-plus'

/**
 * 通用表单验证规则
 */

// 必填验证
export const required = (message: string, trigger: string = 'blur'): any => ({
  required: true,
  message,
  trigger
})

// 手机号验证
export const phone = {
  pattern: /^1[3-9]\d{9}$/,
  message: '请输入正确的手机号码',
  trigger: 'blur'
}

// 邮箱验证
export const email = {
  type: 'email' as const,
  message: '请输入正确的邮箱地址',
  trigger: 'blur'
}

// 数字验证
export const number = {
  pattern: /^\d+$/,
  message: '请输入数字',
  trigger: 'blur'
}

// 金额验证
export const amount = {
  pattern: /^\d+(\.\d{1,2})?$/,
  message: '请输入正确的金额',
  trigger: 'blur'
}

// 长度限制
export const maxLength = (max: number, message?: string) => ({
  max,
  message: message || `长度不能超过${max}个字符`,
  trigger: 'blur'
})

// 最小长度
export const minLength = (min: number, message?: string) => ({
  min,
  message: message || `长度不能少于${min}个字符`,
  trigger: 'blur'
})

// 身份证验证
export const idCard = {
  pattern: /(^\d{15}$)|(^\d{18}$)|(^\d{17}(\d|X|x)$)/,
  message: '请输入正确的身份证号码',
  trigger: 'blur'
}

// 统一社会信用代码验证
export const creditCode = {
  pattern: /^[0-9A-HJ-NPQRTUWXY]{2}\d{6}[0-9A-HJ-NPQRTUWXY]{10}$/,
  message: '请输入正确的统一社会信用代码',
  trigger: 'blur'
}

// 银行卡号验证
export const bankCard = {
  pattern: /^\d{16,19}$/,
  message: '请输入正确的银行卡号',
  trigger: 'blur'
}

// URL验证
export const url = {
  type: 'url' as const,
  message: '请输入正确的URL地址',
  trigger: 'blur'
}

// 密码强度验证
export const password = {
  pattern: /^(?=.*[a-z])(?=.*[A-Z])(?=.*\d)[a-zA-Z\d]{8,}$/,
  message: '密码必须包含大小写字母和数字，且长度不少于8位',
  trigger: 'blur'
}

/**
 * 常用表单规则组合
 */
export const commonRules: FormRules = {
  username: [
    required('请输入用户名'),
    maxLength(50),
    minLength(2)
  ],
  password: [
    required('请输入密码'),
    password
  ],
  email: [
    required('请输入邮箱'),
    email
  ],
  phone: [
    required('请输入手机号'),
    phone
  ],
  name: [
    required('请输入名称'),
    maxLength(100)
  ],
  code: [
    required('请输入编码'),
    maxLength(50)
  ],
  amount: [
    required('请输入金额'),
    amount
  ],
  remark: [
    maxLength(500, '备注长度不能超过500个字符')
  ]
}

/**
 * 业务表单规则
 */
export const businessRules: FormRules = {
  // 产品表单
  product: {
    code: [required('请输入产品编码'), maxLength(50)],
    name: [required('请输入产品名称'), maxLength(200)],
    category_id: [required('请选择产品分类')],
    unit: [required('请输入单位'), maxLength(20)],
    standard_price: [required('请输入标准价格'), amount]
  },
  // 客户表单
  customer: {
    customer_code: [required('请输入客户编码'), maxLength(50)],
    customer_name: [required('请输入客户名称'), maxLength(200)],
    contact_person: [required('请输入联系人'), maxLength(50)],
    contact_phone: [required('请输入联系电话'), phone]
  },
  // 供应商表单
  supplier: {
    supplier_code: [required('请输入供应商编码'), maxLength(50)],
    supplier_name: [required('请输入供应商名称'), maxLength(200)],
    contact_phone: [required('请输入联系电话'), phone],
    credit_code: [creditCode]
  },
  // 销售订单表单
  salesOrder: {
    customer_id: [required('请选择客户')],
    order_date: [required('请选择订单日期')],
    delivery_date: [required('请选择交货日期')]
  },
  // 采购订单表单
  purchaseOrder: {
    supplier_id: [required('请选择供应商')],
    order_date: [required('请选择订单日期')],
    expected_delivery_date: [required('请选择预计交货日期')],
    warehouse_id: [required('请选择入库仓库')]
  }
}

export default {
  required,
  phone,
  email,
  number,
  amount,
  maxLength,
  minLength,
  idCard,
  creditCode,
  bankCard,
  url,
  password,
  commonRules,
  businessRules
}
