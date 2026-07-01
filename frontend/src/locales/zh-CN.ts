/* 中文（简体）翻译 - 秉羲 ERP */
/* 命名空间：{module}.{section}.{key} */
export default {
  /* ============ 通用 ============ */
  common: {
    confirm: '确认',
    cancel: '取消',
    save: '保存',
    delete: '删除',
    edit: '编辑',
    add: '新增',
    search: '搜索',
    reset: '重置',
    export: '导出',
    import: '导入',
    refresh: '刷新',
    loading: '加载中...',
    success: '操作成功',
    failed: '操作失败',
    yes: '是',
    no: '否',
    all: '全部',
    none: '无',
    more: '更多',
    detail: '详情',
    back: '返回',
    submit: '提交',
    close: '关闭',
    enable: '启用',
    disable: '禁用',
    status: '状态',
    create: '创建',
    update: '更新',
    name: '名称',
    code: '编码',
    description: '描述',
    createTime: '创建时间',
    updateTime: '更新时间',
    creator: '创建人',
    pleaseInput: '请输入',
    pleaseSelect: '请选择',
    operation: '操作',
    page: '页',
    total: '共',
    items: '条',
  },

  /* ============ 登录 ============ */
  login: {
    title: '秉羲 ERP',
    subtitle: '秉羲 ERP 系统',
    username: '用户名',
    password: '密码',
    captcha: '验证码',
    remember: '记住我',
    forgot: '忘记密码',
    submit: '登录',
    signingIn: '登录中...',
    success: '登录成功',
    failed: '用户名或密码错误',
    locked: '账户已锁定，请 30 分钟后再试',
    networkError: '网络异常，请稍后重试',
    /* 批次 23 v5 P0-1：Login.vue 接入 i18n 时新增的 key（含动态占位符） */
    formLabel: '登录表单',
    usernameRequired: '请输入用户名',
    passwordRequired: '请输入密码',
    lockedAlert: '账号已被锁定，请 {minutes} 分钟后再试',
    failedAttempts: '连续登录失败 {count} 次，账号已锁定',
    remainingTime: '剩余等待时间：{minutes} 分 {seconds} 秒',
    unlocked: '账号已解除锁定，请重新登录',
    failedFallback: '登录失败',
  },

  /* ============ Dashboard ============ */
  dashboard: {
    title: '工作台',
    welcome: '欢迎回来',
    todayOrders: '今日订单',
    todayRevenue: '今日收入',
    pendingTasks: '待办任务',
    lowStock: '低库存预警',
    salesTrend: '销售趋势',
    topProducts: '热销商品 TOP 10',
    recentActivity: '最近动态',
    quickActions: '快捷入口',
  },

  /* ============ 销售管理 ============ */
  sales: {
    title: '销售管理',
    order: {
      title: '销售订单',
      list: '订单列表',
      create: '新建订单',
      edit: '编辑订单',
      detail: '订单详情',
      number: '订单编号',
      customer: '客户',
      product: '商品',
      quantity: '数量',
      unitPrice: '单价',
      totalAmount: '总金额',
      orderDate: '下单日期',
      deliveryDate: '交货日期',
      status: {
        draft: '草稿',
        confirmed: '已确认',
        shipped: '已发货',
        completed: '已完成',
        cancelled: '已取消',
      },
    },
    customer: {
      title: '客户管理',
      list: '客户列表',
      code: '客户编码',
      name: '客户名称',
      contact: '联系人',
      phone: '联系电话',
      creditLimit: '信用额度',
      level: '客户等级',
    },
    quotation: {
      title: '销售报价',
      list: '报价列表',
      create: '新建报价',
    },
    return: {
      title: '销售退货',
      list: '退货列表',
    },
  },

  /* ============ 库存管理 ============ */
  inventory: {
    title: '库存管理',
    stock: {
      title: '库存查询',
      list: '库存列表',
      product: '商品',
      warehouse: '仓库',
      batch: '批次',
      quantity: '数量',
      availableQty: '可用数量',
      lockedQty: '锁定数量',
      unit: '单位',
      value: '库存价值',
      alert: '库存预警',
    },
    inbound: {
      title: '入库单',
      list: '入库列表',
    },
    outbound: {
      title: '出库单',
      list: '出库列表',
    },
    transfer: {
      title: '调拨单',
      list: '调拨列表',
    },
    count: {
      title: '库存盘点',
      list: '盘点列表',
    },
  },

  /* ============ 系统设置 ============ */
  settings: {
    title: '系统设置',
    user: {
      title: '用户管理',
      list: '用户列表',
      username: '用户名',
      realName: '姓名',
      email: '邮箱',
      phone: '手机号',
      role: '角色',
      lastLogin: '最后登录',
      // 批次 32 v7 P0-2：用户管理 ElMessage 国际化 key
      updateSuccess: '更新成功',
      createSuccess: '创建成功',
      deleteSuccess: '删除成功',
    },
    role: {
      title: '角色管理',
      list: '角色列表',
    },
    permission: {
      title: '权限管理',
    },
    system: {
      title: '系统配置',
      basic: '基础设置',
      security: '安全设置',
      notification: '通知设置',
    },
  },

  /* ============ 采购管理 ============ */
  purchase: {
    title: '采购管理',
    order: {
      title: '采购订单',
      list: '采购订单列表',
    },
    supplier: {
      title: '供应商管理',
      list: '供应商列表',
    },
    receipt: {
      title: '采购收货',
      list: '收货列表',
    },
  },

  /* ============ 财务管理 ============ */
  finance: {
    title: '财务管理',
    ar: {
      title: '应收账款',
      list: '应收列表',
    },
    ap: {
      title: '应付账款',
      list: '应付列表',
    },
    payment: {
      title: '收付款',
    },
    report: {
      title: '财务报表',
    },
  },

  /* ============ 错误信息 ============ */
  error: {
    400: '请求参数错误',
    401: '未授权，请重新登录',
    403: '权限不足',
    404: '资源不存在',
    429: '请求过于频繁，请稍后重试',
    500: '服务器内部错误',
    502: '网关错误',
    503: '服务暂不可用',
    networkError: '网络连接失败',
    timeout: '请求超时',
    unknown: '未知错误',
  },

  /* ============ 通用消息 ============ */
  message: {
    saveSuccess: '保存成功',
    saveFailed: '保存失败',
    createSuccess: '创建成功',
    createFailed: '创建失败',
    updateSuccess: '更新成功',
    updateFailed: '更新失败',
    deleteSuccess: '删除成功',
    deleteConfirm: '确定要删除吗？',
    deleteFailed: '删除失败',
    operationSuccess: '操作成功',
    loadFailed: '加载失败',
    confirmTitle: '确认',
    deleteConfirmTitle: '删除确认',
    auditConfirmTitle: '审核确认',
    rejectConfirmTitle: '确认驳回',
    importSuccess: '导入成功',
    importFailed: '导入失败',
    exportSuccess: '导出成功',
    exportFailed: '导出失败',
    networkError: '网络异常',
    permissionDenied: '权限不足',
    sessionExpired: '会话已过期，请重新登录',
  },

  /* ============ AI 扩展 ============ */
  aiExtend: {
    qualityPrediction: {
      loadListFailed: '加载列表失败',
      productIdRequired: '请填写产品 ID',
      confirmDelete: '确定删除产品 {name} 的质量预测记录？',
      global: '全局',
    },
    process: {
      invalidId: '无效的工艺优化 ID',
      loadDetailFailed: '加载详情失败',
      confirmDelete: '确定删除此工艺优化记录？',
      createFailed: '创建失败',
    },
  },

  /* ============ 预算管理 ============ */
  budget: {
    validation: {
      budgetNoRequired: '请输入预算编号',
      nameRequired: '请输入预算名称',
      periodRequired: '请输入期间',
      totalAmountRequired: '请输入预算总额',
    },
    confirmAudit: '确定审核预算 "{name}" 吗？',
    auditSuccess: '审核成功',
  },

  /* ============ 库存调拨 ============ */
  inventoryTransfer: {
    approvePassed: '审批通过',
    confirmReject: '确定要驳回此调拨单吗？',
    rejected: '已驳回',
  },

  /* ============ 成本管理 ============ */
  cost: {
    validation: {
      collectionDateRequired: '请选择归集日期',
      directMaterialRequired: '请输入直接材料',
      directLaborRequired: '请输入直接人工',
      manufacturingOverheadRequired: '请输入制造费用',
    },
    confirmDelete: '确定删除归集单 "{name}" 吗？',
    confirmAction: '确定{action}该成本归集吗？',
    actionConfirmTitle: '{action}确认',
  },
};
