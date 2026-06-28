/* 中文（简体）翻译 - 冰溪 ERP */
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
    title: '冰溪 ERP',
    subtitle: '面料行业管理系统',
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
    deleteSuccess: '删除成功',
    deleteConfirm: '确定要删除吗？',
    deleteFailed: '删除失败',
    importSuccess: '导入成功',
    importFailed: '导入失败',
    exportSuccess: '导出成功',
    exportFailed: '导出失败',
    networkError: '网络异常',
    permissionDenied: '权限不足',
    sessionExpired: '会话已过期，请重新登录',
  },
};
