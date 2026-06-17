/* English (US) translation - BingXi ERP */
/* Namespace pattern: {module}.{section}.{key} */
export default {
  /* ============ Common ============ */
  common: {
    confirm: 'Confirm',
    cancel: 'Cancel',
    save: 'Save',
    delete: 'Delete',
    edit: 'Edit',
    add: 'Add',
    search: 'Search',
    reset: 'Reset',
    export: 'Export',
    import: 'Import',
    refresh: 'Refresh',
    loading: 'Loading...',
    success: 'Success',
    failed: 'Failed',
    yes: 'Yes',
    no: 'No',
    all: 'All',
    none: 'None',
    more: 'More',
    detail: 'Detail',
    back: 'Back',
    submit: 'Submit',
    close: 'Close',
    enable: 'Enable',
    disable: 'Disable',
    status: 'Status',
    create: 'Create',
    update: 'Update',
    name: 'Name',
    code: 'Code',
    description: 'Description',
    createTime: 'Created At',
    updateTime: 'Updated At',
    creator: 'Creator',
    tenant: 'Tenant',
    pleaseInput: 'Please input',
    pleaseSelect: 'Please select',
    operation: 'Operation',
    page: 'Page',
    total: 'Total',
    items: 'items',
  },

  /* ============ Login ============ */
  login: {
    title: 'BingXi ERP',
    subtitle: 'Fabric Industry Management',
    username: 'Username',
    password: 'Password',
    captcha: 'Captcha',
    remember: 'Remember me',
    forgot: 'Forgot password',
    submit: 'Login',
    signingIn: 'Signing in...',
    success: 'Login successful',
    failed: 'Incorrect username or password',
    locked: 'Account locked, please retry in 30 minutes',
    networkError: 'Network error, please retry later',
  },

  /* ============ Dashboard ============ */
  dashboard: {
    title: 'Dashboard',
    welcome: 'Welcome back',
    todayOrders: "Today's Orders",
    todayRevenue: "Today's Revenue",
    pendingTasks: 'Pending Tasks',
    lowStock: 'Low Stock Alert',
    salesTrend: 'Sales Trend',
    topProducts: 'Top 10 Products',
    recentActivity: 'Recent Activity',
    quickActions: 'Quick Actions',
  },

  /* ============ Sales ============ */
  sales: {
    title: 'Sales',
    order: {
      title: 'Sales Orders',
      list: 'Order List',
      create: 'New Order',
      edit: 'Edit Order',
      detail: 'Order Detail',
      number: 'Order Number',
      customer: 'Customer',
      product: 'Product',
      quantity: 'Quantity',
      unitPrice: 'Unit Price',
      totalAmount: 'Total Amount',
      orderDate: 'Order Date',
      deliveryDate: 'Delivery Date',
      status: {
        draft: 'Draft',
        confirmed: 'Confirmed',
        shipped: 'Shipped',
        completed: 'Completed',
        cancelled: 'Cancelled',
      },
    },
    customer: {
      title: 'Customers',
      list: 'Customer List',
      code: 'Customer Code',
      name: 'Customer Name',
      contact: 'Contact',
      phone: 'Phone',
      creditLimit: 'Credit Limit',
      level: 'Level',
    },
    quotation: {
      title: 'Quotations',
      list: 'Quotation List',
      create: 'New Quotation',
    },
    return: {
      title: 'Sales Returns',
      list: 'Return List',
    },
  },

  /* ============ Inventory ============ */
  inventory: {
    title: 'Inventory',
    stock: {
      title: 'Stock Query',
      list: 'Stock List',
      product: 'Product',
      warehouse: 'Warehouse',
      batch: 'Batch',
      quantity: 'Quantity',
      availableQty: 'Available Qty',
      lockedQty: 'Locked Qty',
      unit: 'Unit',
      value: 'Stock Value',
      alert: 'Stock Alert',
    },
    inbound: {
      title: 'Inbound',
      list: 'Inbound List',
    },
    outbound: {
      title: 'Outbound',
      list: 'Outbound List',
    },
    transfer: {
      title: 'Transfers',
      list: 'Transfer List',
    },
    count: {
      title: 'Stock Taking',
      list: 'Count List',
    },
  },

  /* ============ Settings ============ */
  settings: {
    title: 'Settings',
    user: {
      title: 'User Management',
      list: 'User List',
      username: 'Username',
      realName: 'Real Name',
      email: 'Email',
      phone: 'Phone',
      role: 'Role',
      lastLogin: 'Last Login',
    },
    role: {
      title: 'Role Management',
      list: 'Role List',
    },
    permission: {
      title: 'Permissions',
    },
    tenant: {
      title: 'Tenants',
    },
    system: {
      title: 'System Config',
      basic: 'Basic',
      security: 'Security',
      notification: 'Notification',
    },
  },

  /* ============ Purchase ============ */
  purchase: {
    title: 'Purchase',
    order: {
      title: 'Purchase Orders',
      list: 'Purchase Order List',
    },
    supplier: {
      title: 'Suppliers',
      list: 'Supplier List',
    },
    receipt: {
      title: 'Purchase Receipts',
      list: 'Receipt List',
    },
  },

  /* ============ Finance ============ */
  finance: {
    title: 'Finance',
    ar: {
      title: 'Accounts Receivable',
      list: 'AR List',
    },
    ap: {
      title: 'Accounts Payable',
      list: 'AP List',
    },
    payment: {
      title: 'Payments',
    },
    report: {
      title: 'Financial Reports',
    },
  },

  /* ============ Errors ============ */
  error: {
    400: 'Bad Request',
    401: 'Unauthorized, please login again',
    403: 'Permission denied',
    404: 'Resource not found',
    429: 'Too many requests, please retry later',
    500: 'Internal server error',
    502: 'Bad gateway',
    503: 'Service unavailable',
    networkError: 'Network connection failed',
    timeout: 'Request timeout',
    unknown: 'Unknown error',
  },

  /* ============ Messages ============ */
  message: {
    saveSuccess: 'Saved successfully',
    saveFailed: 'Save failed',
    deleteSuccess: 'Deleted successfully',
    deleteConfirm: 'Are you sure to delete?',
    deleteFailed: 'Delete failed',
    importSuccess: 'Imported successfully',
    importFailed: 'Import failed',
    exportSuccess: 'Exported successfully',
    exportFailed: 'Export failed',
    networkError: 'Network error',
    permissionDenied: 'Permission denied',
    sessionExpired: 'Session expired, please login again',
  },
};
