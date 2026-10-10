/**
 * usePi.ts - 采购验货核心 composable
 * 任务编号: P14 批 2 I-3 第 5 批（拆分原 purchase-inspection/index.vue）
 * 提供检验单列表 / 统计 / 分页 / 过滤 / 表单 / 详情 / 选项加载等核心方法
 * 业务流程（查询 / 重置 / 创建 / 编辑 / 查看 / 提交 / 完成）由 usePiProc 提供
 * 行为完全保持一致（仅结构重构）
 * 批次 286：tableData 接入 useTableApi，移除手写分页逻辑
 *
 * 注意：返回值使用 reactive({...}) 包装，父组件可直接访问字段（自动解包 ref）
 * 子组件通过 :model-value/@update:model-value 模式传入；不会修改 prop
 */
import { ref, reactive, watch } from 'vue';
import { ElMessage } from 'element-plus';
import { msg } from '@/utils/message';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import {
  type PurchaseInspection,
  type PurchaseInspectionItem,
  type PurchaseInspectionQueryParams,
  type PurchaseInspectionStats,
  getPurchaseInspectionStats,
} from '@/api/purchase-inspection';
import {
  getPurchaseReceiptList,
  getReceiptItems,
  type ReceiptItem,
  type PurchaseReceiptEntity,
} from '@/api/purchase-receipt';
import { getSupplierList } from '@/api/supplier';
import { useTableApi } from '@/composables/useTableApi';
import { logger } from '@/utils/logger';
import { i18n } from '@/i18n';
// 统计分桶（pending/pass/failed+partial 归不合格）全部发生在服务端
// （backend services/purchase_inspection_service.rs::base_filtered_query，词表常量与写入侧同源），
// 前端不再对行集做任何词表比较，故本文件不引入 PURCHASE_INSPECTION_* 常量——
// 比较点收敛到唯一写入侧，禁止在浏览器里维持第二套计数口径。

/**
 * 采购验货主业务 composable
 * 集中管理列表、统计、过滤、表单、详情、选项加载
 */
export function usePi() {
  // 统计卡数据：服务端聚合四值（GET /purchase/inspections/stats）。
  // null = 未取到（失败或从未返回），卡片显示占位符「—」，不以 0 顶替真实未知。
  const stats = ref<PurchaseInspectionStats | null>(null);

  // 日期范围（独立 ref，便于 PurchaseInspectionFilter 双向绑定；fetch 前注入 queryParams.inspection_date_from/to）
  const dateRange = ref<[Date, Date] | null>(null);

  // 列表数据接入 useTableApi
  // 采购验货 API 使用 snake_case 分页参数（page/page_size），匹配 useTableApi 默认配置
  const {
    data: tableData,
    total,
    loading,
    page,
    pageSize,
    queryParams,
    refresh: fetchData,
  } = useTableApi<PurchaseInspection>({
    url: '/purchase/inspections',
    defaultPageSize: 20,
    defaultParams: {
      keyword: '',
      supplier_id: undefined as number | undefined,
      status: '',
      result: '',
      inspection_date_from: '',
      inspection_date_to: '',
    },
    onError: (err: unknown) => {
      logger.error('获取数据失败:', err);
    },
  });

  // 统计卡消费契约（服务端聚合，四值不在前端重算）：
  // 1) 数据源：GET /purchase/inspections/stats，出参四键 total/pending/passed/failed
  //    与后端 DTO PurchaseInspectionStats（models/purchase_inspection 内定义）逐键对齐，
  //    均为整数计数（u64）；partial 归入 failed 的分桶裁定在服务端与写入词表同源。
  // 2) 同参数同刷新：触发点为 watch(tableData)——useTableApi 每次列表取数成功都会把
  //    响应行集以**新数组**写回 data.value（含初始加载、翻页自动重载、handleQuery/
  //    handleReset/创建/更新/完成后的显式 refresh），这是"列表刚以当前参数完成一次取数"
  //    的唯一汇聚信号；stats 请求参数与列表请求实发的集合取**同一份快照**
  //    （{...queryParams, page, page_size}），不在前端另装配第二套参数。
  //    刻意不以 queryParams 变化为触发：筛选表单每次键入都改 queryParams 而列表只在
  //    点「查询」后才取数，事件不同步会让卡片先于表格换口径（口径分叉正根）。
  // 3) 竞态保护：请求序号 statsSeq——后发请求为代表，旧响应晚到一律丢弃，
  //    既不得覆盖新参数取到的值，也不得用旧参数的失败清空新参数的数据。
  // 4) 失败处置（fail-visible）：请求失败或四键形状不符契约（requireStatsShape 显式抛错，
  //    不做 ?? 0 兜底）→ stats 置 null（卡片渲染「—」明确留空）+ msg.error('loadFailed')
  //    即时提示 + logger 细节；列表自身取数失败时 tableData 不更新 → stats 与表格行同步
  //    保持旧数据，两侧陈旧一致，不会出现卡片新表格旧的单边刷新。
  // 5) 已知恒等式事实（非缺陷，分桶口径见服务端 purchase_inspection_service 的
  //    inspection_stats 文书，partial 归入 failed）：
  //    pending+passed+failed===total 在当前写入规则下成立但无 DB 约束兜底，
  //    词表外异常行只进 total——四卡之和偶小于总数时差值即异常行数，如实呈现不掩盖。
  let statsSeq = 0;

  /** 四键逐键校验为有限数字；缺键/非数字=契约漂移，显式抛错进失败分支，绝不兜底成 0 */
  const requireStatsShape = (raw: unknown): PurchaseInspectionStats => {
    const obj = raw as Partial<PurchaseInspectionStats> | null | undefined;
    const invalid = (['total', 'pending', 'passed', 'failed'] as const).filter(
      key => typeof obj?.[key] !== 'number' || !Number.isFinite(obj[key] as number)
    );
    if (invalid.length > 0) {
      throw new Error(
        `[purchase-inspection] stats 响应契约不符：键 ${invalid.join('/')} 缺失或非整数计数，实际=${JSON.stringify(raw)}`
      );
    }
    return obj as PurchaseInspectionStats;
  };

  const fetchStats = async (): Promise<void> => {
    const seq = ++statsSeq;
    try {
      // queryParams 的键集由上方 useTableApi defaultParams 按 PurchaseInspectionQueryParams
      // 形状构造（keyword/supplier_id/status/result/inspection_date_from/to），
      // Record<string, unknown> → 具名形状是类型桥接断言，键名单一来源、无第二处装配点。
      const params: PurchaseInspectionQueryParams = {
        ...queryParams.value,
        page: page.value,
        page_size: pageSize.value,
      } as PurchaseInspectionQueryParams;
      const res = await getPurchaseInspectionStats(params);
      if (seq !== statsSeq) return;
      stats.value = requireStatsShape(res.data);
    } catch (error) {
      if (seq !== statsSeq) return;
      stats.value = null;
      logger.error('[purchase-inspection] 统计卡服务端聚合取数失败', error);
      msg.error('loadFailed');
    }
  };

  watch(tableData, () => {
    void fetchStats();
  });

  // 选项
  const suppliers = ref<{ id: number; name: string }[]>([]);
  // receipts 用于创建质检单时选入库单；supplier_id 由入库单携带（CreatePurchaseInspectionRequest 实际必填 supplier_id）
  const receipts = ref<{ id: number; receipt_no: string; supplier_id: number }[]>([]);

  // 对话框
  const dialogVisible = ref(false);
  const isEdit = ref(false);
  const submitLoading = ref(false);
  const formData = reactive<{
    id?: number;
    receipt_id?: number;
    supplier_id?: number;
    inspection_date: string;
    remark: string;
    items: Partial<PurchaseInspectionItem>[];
  }>({
    id: undefined,
    receipt_id: undefined,
    supplier_id: undefined,
    inspection_date: '',
    remark: '',
    items: [],
  });

  const formRules = {
    receipt_id: [
      {
        required: true,
        message: i18n.global.t('purchaseInspection.validation.receiptRequired'),
        trigger: 'change',
      },
    ],
    inspection_date: [
      {
        required: true,
        message: i18n.global.t('purchaseInspection.validation.inspectionDateRequired'),
        trigger: 'change',
      },
    ],
  };

  // 详情对话框
  const detailDialogVisible = ref(false);
  const detailData = ref<PurchaseInspection>({} as PurchaseInspection);
  // 表头响应不含 items；handleView 异步加载明细后写入此 ref，PurchaseInspectionDetail.vue 读取此数组
  const detailItems = ref<PurchaseInspectionItem[]>([]);

  // 入库单明细加载状态
  const receiptItemsLoading = ref(false);

  /** 同步 dateRange 到 queryParams.inspection_date_from/to */
  const syncDateRangeToQuery = () => {
    if (dateRange.value) {
      queryParams.value = {
        ...queryParams.value,
        inspection_date_from: dateRange.value[0].toISOString(),
        inspection_date_to: dateRange.value[1].toISOString(),
      };
    } else {
      queryParams.value = {
        ...queryParams.value,
        inspection_date_from: '',
        inspection_date_to: '',
      };
    }
  };

  /**
   * 加载供应商列表（真实 API，替换原硬编码 mock 数组）
   * suppliers 用于 CreatePurchaseInspectionRequest.supplier_id 选择（后端实际必填，缺失则校验失败）
   */
  const fetchSuppliers = async () => {
    try {
      const res = await getSupplierList({ page: 1, page_size: 1000 });
      suppliers.value = res.data.items.map((s: { id: number; supplier_name: string }) => ({
        id: s.id,
        name: s.supplier_name,
      }));
    } catch (error) {
      logger.error('[purchase-inspection] 加载供应商列表失败', error);
    }
  };

  /**
   * 加载入库单列表（真实 API，替换原硬编码 mock 数组）
   */
  const fetchReceipts = async () => {
    try {
      const res = await getPurchaseReceiptList({ page: 1, page_size: 500 });
      receipts.value = res.data.items
        .filter((r): r is PurchaseReceiptEntity & { id: number } => r.id !== undefined)
        .map((r: PurchaseReceiptEntity & { id: number }) => ({
          id: r.id,
          receipt_no: r.receipt_no,
          supplier_id: r.supplier_id,
        }));
    } catch (error) {
      logger.error('[purchase-inspection] 加载入库单列表失败', error);
    }
  };

  /**
   * 入库单变化时：
   * 1. 把选中的入库单落回 formData.receipt_id（表单项 prop="receipt_id" 的必填校验与
   *    创建 payload 都读这一处；表单子组件的明细镜像只读不写该键，父组件是唯一写入方，
   *    否则选完入库单后 receipt_id 恒为 undefined，el-form 校验挡下提交、质检单永远建不出来）
   * 2. 从入库单派生 supplier_id（CreatePurchaseInspectionRequest 实际必填 supplier_id，缺失时后端拒绝）
   * 3. 加载入库单明细作为质检明细初始值
   */
  const handleReceiptChange = async (receiptId: number) => {
    if (!receiptId) {
      formData.receipt_id = undefined;
      formData.items = [];
      formData.supplier_id = undefined;
      return;
    }
    formData.receipt_id = receiptId;
    // 从已加载的 receipts 列表中找到所选入库单的 supplier_id
    const receipt = receipts.value.find(r => r.id === receiptId);
    if (receipt) {
      formData.supplier_id = receipt.supplier_id;
    }
    receiptItemsLoading.value = true;
    try {
      const res = await getReceiptItems(receiptId);
      const items: ReceiptItem[] = res.data;
      if (items.length === 0) {
        msg.info('noReceiptDetails');
        formData.items = [];
        return;
      }
      // 将入库单明细映射为检验单明细，初始化各数量字段
      formData.items = items.map(item => ({
        product_id: item.product_id,
        // 入库明细行按后端契约是 material_code/material_name
        product_name: item.material_name,
        product_code: item.material_code,
        expected_quantity: item.quantity,
        inspected_quantity: 0,
        passed_quantity: 0,
        failed_quantity: 0,
        defect_reason: '',
      }));
    } catch (error) {
      const errMsg = error instanceof Error ? error.message : '获取入库单明细失败，请稍后重试';
      ElMessage.error(errMsg);
      logger.error('获取入库单明细失败:', error);
      formData.items = [];
    } finally {
      receiptItemsLoading.value = false;
    }
  };

  // 懒加载标记
  const hasLoaded = createLazyLoader();

  // 使用 reactive 包装，父组件可直接访问字段
  return reactive({
    // 统计
    stats,
    // 列表
    tableData,
    loading,
    total,
    dateRange,
    page,
    pageSize,
    queryParams,
    // 选项
    suppliers,
    receipts,
    // 表单对话框
    dialogVisible,
    isEdit,
    submitLoading,
    formData,
    formRules,
    // 详情对话框
    detailDialogVisible,
    detailData,
    detailItems,
    // 入库单明细加载
    receiptItemsLoading,
    // 加载方法
    fetchData,
    fetchSuppliers,
    fetchReceipts,
    handleReceiptChange,
    syncDateRangeToQuery,
    // 懒加载标记
    hasLoaded,
    // 兼容旧名（loadIfNot 接受字符串 key）
    loadIfNot,
  });
}
