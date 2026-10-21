/**
 * 功能重叠入口登记册（重复入口债务的唯一账）
 *
 * 功能：登记"两条在册入口承担同一或高度重叠功能"的每一组事实，并记录其处置状态与裁定人。
 * 为什么要登记册而不是自动判定：机器只能判可形式化的部分（标题词干相同的组），
 *   语义重复判不了。实测把标题词干当唯一判据会既误报（财务管理 与 财务分析 是两个功能）
 *   又漏报（凭证管理 与 财务管理下的凭证 Tab 标题完全不同却各自建单）。
 *   用启发式当判据会把"没被抓到"当成"没有重复"，是假安心，因此这里改成人工声明加机器核验。
 * 调用方：frontend/tests/unit/route-overlap-registry.test.ts（vitest 与 CI ci-frontend-test job）。
 * 入参：无，纯静态声明。
 * 传给谁：只被登记册锁测消费，不进应用打包面（tests 目录不在 tsconfig include 内）。
 * 存什么：组标识、成员路径、机器可判来源、处置状态、默认建议、裁定人、依据说明。
 * 存哪里：只在本文件，新增或改状态必须同批改登记册锁测的棘轮基线，二者一起走评审。
 *
 * 处置状态取值含义：
 *   distinct-after-review   逐条看过，确认是两个不同功能，不构成重复入口
 *   pending-sign-off        疑似重复，保留哪个待产品裁定，裁定前不得单方面删入口
 *   resolved-by-redirect    已收口：其中一条改重定向或隐藏，账留着防止有人再把它加回来
 */

/** 一条重叠入口的登记项 */
export interface RouteOverlapEntry {
  /** 组标识，登记册内唯一 */
  id: string;
  /** 成员路径，至少两条 */
  members: string[];
  /** 该组是怎么进来的：机器判出的词干碰撞，还是人工声明的语义重叠 */
  detectedBy: 'title-stem' | 'declared';
  /** 处置状态 */
  disposition: 'distinct-after-review' | 'pending-sign-off' | 'resolved-by-redirect';
  /** 待裁项给出的默认建议，供裁定时直接采纳或改 */
  proposal?: string;
  /** 裁定或复核人 */
  decidedBy: string;
  /** 依据与影响面说明，必须写清"为什么这样判" */
  note: string;
  /** 已收口项的存活成员（重定向目标），仅 resolved-by-redirect 使用 */
  survivor?: string;
}

export const ROUTE_OVERLAPS: RouteOverlapEntry[] = [
  {
    id: 'finance-container-vs-financial-analysis',
    members: ['/finance', '/financial-analysis'],
    detectedBy: 'title-stem',
    disposition: 'distinct-after-review',
    decidedBy: '57231307',
    note: '财务管理是凭证、科目、账期等 Tab 的容器页，财务分析是报表族的独立计算页，'
      + '二者读写的端点与承担动作不同，不构成重复入口。',
  },
  {
    id: 'sales-container-vs-sales-analysis',
    members: ['/sales', '/sales-analysis'],
    detectedBy: 'title-stem',
    disposition: 'distinct-after-review',
    decidedBy: '57231307',
    note: '销售管理是订单列表与建单入口，销售分析是只读统计页，端点与用途不同。',
  },
  {
    id: 'budget-draft-vs-approval',
    members: ['/budget', '/budgets'],
    detectedBy: 'title-stem',
    disposition: 'pending-sign-off',
    proposal: '保留预算审批一侧为唯一入口，预算编制改为该页内的动作或 Tab',
    decidedBy: '待产品裁定',
    note: '两条都在 finance:read 下，标题只差编制与审批，属同一业务流的两段被拆成两个入口。'
      + '合并会变更可达集，牵动 access-map 基线重采与人工签署，不得单方面改路由。',
  },
  {
    id: 'supplier-management-vs-supplier-360',
    members: ['/supplier', '/supplier-enhanced'],
    detectedBy: 'title-stem',
    disposition: 'pending-sign-off',
    proposal: '保留供应商管理为主入口，供应商 360 收成该页内的一个视图',
    decidedBy: '待产品裁定',
    note: '两条同为 suppliers:read，供应商 360 是聚合视图，与主档管理页存在字段与动作重叠。'
      + '合并同样变更可达集，需基线重采加人工签署。',
  },
  {
    id: 'voucher-entry-duplication',
    members: ['/voucher', '/finance'],
    detectedBy: 'declared',
    disposition: 'pending-sign-off',
    proposal: '保留财务管理页内的凭证 Tab 为主入口，凭证管理页改重定向，'
      + '并把建单表单收敛为同一组件',
    decidedBy: '待产品裁定',
    note: '两套独立的凭证新建 UI，标题不同所以机器判不出来。实测差异：财务侧建单按钮绑了'
      + '在途锁并有按钮级权限常量（含 VOUCHER_CREATE），凭证侧此前既无锁也无按钮级门，'
      + '只在最近的提交里补上锁；全部既有凭证 UI 用例都走财务侧。'
      + '收口会变更可达集（凭证管理登记在遍历集里），必须走基线重采与人工签署。',
  },
  {
    id: 'crm-vs-crm-enhanced',
    members: ['/crm', '/crm-enhanced'],
    detectedBy: 'declared',
    disposition: 'pending-sign-off',
    proposal: '先核对两条页面的实际列与动作集合，再决定是并入还是各自保留',
    decidedBy: '待产品裁定',
    note: '标题一个是客户关系管理、一个是 CRM 高级分析，词干不同机器不判；'
      + '两条同持 customers:read，功能面重叠但未逐字段核对，不预设结论。',
  },
  {
    id: 'supplier-evaluation-vs-supplier-enhanced',
    members: ['/supplier-evaluation', '/supplier-enhanced'],
    detectedBy: 'declared',
    disposition: 'pending-sign-off',
    proposal: '与供应商 360 那条一起裁，避免同域两次改动可达集',
    decidedBy: '待产品裁定',
    note: '供应商评估与供应商 360 都读供应商主数据并给出评价类字段，'
      + '是否重复取决于评估指标是否已在 360 视图中呈现，尚未逐列核对。',
  },
  {
    id: 'sales-analysis-vs-bi-sales-analysis',
    members: ['/sales-analysis', '/bi/sales-analysis'],
    detectedBy: 'declared',
    disposition: 'pending-sign-off',
    proposal: '保留 BI 多维分析一侧，旧销售分析页改为只读跳转或下线',
    decidedBy: '待产品裁定',
    note: '两条同持 sales:read，BI 侧是多维透视、旧侧是固定报表，'
      + '口径若不统一会出现同名指标两个数，属需产品定口径的一类。',
  },
  {
    id: 'ar-reconciliation-vs-enhanced',
    members: ['/ar-reconciliation', '/ar-reconciliation/enhanced'],
    detectedBy: 'declared',
    disposition: 'pending-sign-off',
    proposal: '增强版为主面，基础对账页并入其只读视图',
    decidedBy: '待产品裁定',
    note: '子路径挂在同一资源下，权限键同值，属最典型的新旧两版并存。'
      + '与既有裁定一致：基础页保持 admin-only 孤儿形态不改动会把泄漏留在那里，需一并裁。',
  },
  {
    id: 'accounting-period-entry-collapse',
    members: ['/periods', '/accounting-period'],
    detectedBy: 'title-stem',
    disposition: 'resolved-by-redirect',
    survivor: '/accounting-period',
    decidedBy: '57231307',
    note: '旧会计期间页已删视图，/periods 改为 hidden 重定向到会计期间管理，'
      + '导航树同步移除该叶子，侧栏只剩一条入口。此行留账，防止有人把旧入口再加回来。',
  },
];
