/**
 * useVchrLst.ts - 凭证列表核心 composable
 * 提供凭证列表查询、表单管理、科目加载、详情等核心方法
 * 业务流程（打印/导出/审核/记账）由 useVchrLstProc 提供
 *
 * 设计说明：使用 reactive 而非 ref 包装返回值，便于父组件
 *   const vchr = useVchrLst() 后直接以 plain value 形式访问 vchr.xxx
 *   避免子组件 prop 期望 boolean/array 等基础类型时类型不匹配
 *
 * 凭证提交/回显的契约要点：
 * - 原提交体携带前端自创键 type/entries ⇒ 创建 400（后端 CreateVoucherRequestDto
 *   必填 voucher_type/items 反序列化失败）、更新静默 no-op（后端两字段均 Option，
 *   收不到即全部保持原值）。现 handleSubmit 构造后端规范键 payload：
 *   { voucher_type, voucher_date, items:[{ subject_id, debit, credit, summary }] }。
 * - 凭证类型词表三端同源：下拉选项消费 GET /vouchers/types（后端
 *   VoucherService::available_voucher_types 静态配置，code=记/收/付/转），
 *   不再硬编码 general/customized；voucher_type 必填、无伪造默认值。
 * - 编辑回显走详情端点（flatten voucher::Model + entries 双命名分录），
 *   查看详情的借贷合计由 entries 前端派生（后端列表/详情无合计列）。
 */
import { ref, watch, reactive, computed } from 'vue';
import { msg } from '@/utils/message';
import {
  getVoucher,
  createVoucher,
  updateVoucher,
  getVoucherTypes,
  generateVoucherNo,
  type VoucherEntity,
  type CreateVoucherPayload,
  type UpdateVoucherPayload,
  type VoucherItemPayload,
} from '@/api/voucher';
import { getAccountSubjectTree } from '@/api/account-subject';
import { useTableApi } from '@/composables/useTableApi';
import { logger } from '@/utils/logger';

/** 科目下拉选项 */
interface SubjectOption {
  label: string;
  value: number;
}

/**
 * 分录编辑模型：绑定 el-input-number/el-select（数值/字符串混入），
 * UI 键沿用详情出参双命名中的 account_subject_* 族（详情回显零转换）。
 */
interface VoucherEntryForm {
  account_subject_id: number;
  debit_amount: number | string;
  credit_amount: number | string;
  description?: string | null;
}

/**
 * 凭证列表 composable
 * 集中管理列表、表单、详情等业务状态
 * 对话框可见性由父组件本地 ref 管理
 */
export function useVchrLst() {
  // 列表数据接入 useTableApi
  // 凭证 API 返回 ApiResponse<VoucherEntity[]>，data 为裸数组；useTableApi detectList 兼容裸数组
  // 分页参数使用 snake_case（page/page_size），匹配 useTableApi 默认配置
  const {
    data: tableData,
    total,
    loading,
    page,
    pageSize,
    queryParams,
    refresh: loadData,
  } = useTableApi<VoucherEntity>({
    url: '/vouchers',
    defaultPageSize: 20,
    // P0 修复：查询键对齐后端 VoucherQuery（voucher_type/start_date/end_date，
    // 原 type/voucher_date_start/voucher_date_end 后端不识别 ⇒ 筛选恒不生效）
    defaultParams: {
      voucher_no: '',
      start_date: '',
      end_date: '',
      voucher_type: '',
      status: '',
    },
    onError: (err: unknown) => {
      logger.error('获取凭证列表失败', err);
      msg.error('loadVoucherListFailed');
    },
  });

  // 新建/编辑对话框表单（reactive 包装以便子组件双向同步字段）
  const dialogTitle = ref('新增凭证');
  const form = reactive<{
    id?: number;
    voucher_no: string;
    voucher_date: string;
    voucher_type: string;
    status: string;
    entries: VoucherEntryForm[];
  }>({
    voucher_no: '',
    voucher_date: new Date().toISOString().split('T')[0],
    // 无凭证类型不再硬编码 'general'（后端词表无此值）：置空 + 提交必填校验
    voucher_type: '',
    status: 'draft',
    entries: [{ account_subject_id: 0, debit_amount: 0, credit_amount: 0, description: '' }],
  });

  // 详情对话框数据
  const viewData = ref<VoucherEntity | null>(null);

  /** 详情借贷合计：后端无合计列，由 entries 分录派生（真实数据求和） */
  const viewTotalDebit = computed(() =>
    (viewData.value?.entries ?? []).reduce((sum, e) => sum + Number(e.debit_amount ?? 0), 0)
  );
  const viewTotalCredit = computed(() =>
    (viewData.value?.entries ?? []).reduce((sum, e) => sum + Number(e.credit_amount ?? 0), 0)
  );

  // 凭证类型选项（后端 available_voucher_types 单一真源）
  const voucherTypes = ref<{ label: string; value: string }[]>([]);

  // 科目选项（扁平化）
  const accountSubjectOptions = ref<SubjectOption[]>([]);

  /** 借贷合计（表单实时派生，用于提交前平衡校验） */
  const totalDebit = computed(() =>
    form.entries.reduce((sum, e) => sum + Number(e.debit_amount ?? 0), 0)
  );
  const totalCredit = computed(() =>
    form.entries.reduce((sum, e) => sum + Number(e.credit_amount ?? 0), 0)
  );

  /** 加载凭证类型（P0 修复：后端返回 {code,name}[]，原按 string[] 解析恒空） */
  const loadVoucherTypes = async () => {
    try {
      const res = await getVoucherTypes();
      const items = Array.isArray(res.data) ? res.data : [];
      voucherTypes.value = items.map(t => ({ label: t.name, value: t.code }));
    } catch (error) {
      logger.error('获取凭证类型失败', error);
    }
  };

  /** 加载科目（扁平化为下拉选项） */
  const loadAccountSubjects = async () => {
    try {
      const res = await getAccountSubjectTree();
      const d = (res as { data?: unknown }).data;
      const items = (Array.isArray(d) ? d : []) as {
        id: number;
        name: string;
        children?: unknown[];
      }[];
      const flattenOptions = (): SubjectOption[] => {
        const result: SubjectOption[] = [];
        const traverse = (ns: { id: number; name: string; children?: unknown[] }[]) => {
          ns.forEach(node => {
            result.push({ label: node.name, value: node.id });
            if (
              node.children &&
              (node.children as { id: number; name: string; children?: unknown[] }[]).length > 0
            ) {
              traverse(node.children as { id: number; name: string; children?: unknown[] }[]);
            }
          });
        };
        traverse(items);
        return result;
      };
      accountSubjectOptions.value = flattenOptions();
    } catch (error) {
      logger.error('获取科目列表失败', error);
    }
  };

  /** 添加分录 */
  const addEntry = () => {
    form.entries.push({
      account_subject_id: 0,
      debit_amount: 0,
      credit_amount: 0,
      description: '',
    });
  };

  /** 删除分录 */
  const removeEntry = (index: number) => {
    if (form.entries.length > 1) {
      form.entries.splice(index, 1);
    }
  };

  /** 重置表单为初始态（新增/编辑共用） */
  const resetForm = (partial?: Partial<typeof form>) => {
    Object.assign(form, {
      id: undefined,
      voucher_no: '',
      voucher_date: new Date().toISOString().split('T')[0],
      voucher_type: '',
      status: 'draft',
      entries: [{ account_subject_id: 0, debit_amount: 0, credit_amount: 0, description: '' }],
      ...partial,
    });
  };

  /** 准备新增对话框数据（父组件调用后需自行打开对话框） */
  const openAddDialog = async () => {
    dialogTitle.value = '新增凭证';
    resetForm();
    try {
      const res = await generateVoucherNo();
      const data = (res as { data?: { voucher_no?: string } | string }).data;
      const voucherNo =
        typeof data === 'string' ? data : (data as { voucher_no?: string })?.voucher_no || '';
      form.voucher_no = voucherNo;
    } catch (error) {
      logger.error('生成凭证号失败', error);
      msg.error('generateVoucherNoFailed');
    }
  };

  /**
   * 准备编辑对话框数据（父组件调用后需自行打开对话框）
   * 详情端点返回 flatten voucher::Model（含 voucher_type/voucher_date）+ entries（双命名分录，
   * Decimal→string），按 UI 编辑模型归一后回填。
   */
  const openEditDialog = async (row: VoucherEntity) => {
    dialogTitle.value = '编辑凭证';
    const res = await getVoucher(row.id!);
    const detail = res.data;
    resetForm({
      id: detail.id,
      voucher_no: detail.voucher_no,
      voucher_date: detail.voucher_date,
      voucher_type: detail.voucher_type,
      status: detail.status,
      entries: (detail.entries ?? []).map(e => ({
        account_subject_id: e.account_subject_id ?? 0,
        debit_amount: Number(e.debit_amount ?? 0),
        credit_amount: Number(e.credit_amount ?? 0),
        description: e.description ?? '',
      })),
    });
  };

  /** 准备查看详情数据（父组件调用后需自行打开对话框） */
  const openViewDialog = async (row: VoucherEntity) => {
    try {
      const res = await getVoucher(row.id!);
      // 安全检查：防止后端返回 data 为 null 时崩溃
      if (res.data) viewData.value = res.data;
    } catch (error) {
      logger.error('获取详情失败', error);
      msg.error('loadDetailFailed');
    }
  };

  /** UI 分录行 → 后端 VoucherItemDto 规范键（subject_id/debit/credit/summary） */
  const buildItemsPayload = (entries: VoucherEntryForm[]): VoucherItemPayload[] =>
    entries.map(e => ({
      subject_id: e.account_subject_id,
      debit: Number(e.debit_amount ?? 0),
      credit: Number(e.credit_amount ?? 0),
      summary: e.description || undefined,
    }));

  /** 提交表单（新增/编辑） */
  const handleSubmit = async () => {
    if (!form.voucher_no || !form.voucher_date) {
      msg.warning('requiredFieldsMissing');
      return false;
    }
    // P0 修复：凭证类型为后端词表必填键（原前端自创 type 键 + 'general' 值后端不识别）
    if (!form.voucher_type) {
      msg.warning('requiredFieldsMissing');
      return false;
    }
    if (Math.abs(totalDebit.value - totalCredit.value) > 0.01) {
      msg.warning('entriesUnbalanced');
      return false;
    }
    const validEntries = form.entries.filter(
      e =>
        e.account_subject_id > 0 &&
        (Number(e.debit_amount ?? 0) > 0 || Number(e.credit_amount ?? 0) > 0)
    );
    if (validEntries.length === 0) {
      msg.warning('pleaseAddEntry');
      return false;
    }
    try {
      const items = buildItemsPayload(validEntries);
      if (form.id) {
        const payload: UpdateVoucherPayload = {
          voucher_type: form.voucher_type,
          voucher_date: form.voucher_date,
          items,
        };
        await updateVoucher(form.id, payload);
        msg.success('updateSuccess');
      } else {
        const payload: CreateVoucherPayload = {
          voucher_type: form.voucher_type,
          voucher_date: form.voucher_date,
          items,
        };
        await createVoucher(payload);
        msg.success('createSuccess');
      }
      await loadData();
      return true;
    } catch (error) {
      logger.error('操作失败', error);
      msg.error('operationFailed');
      return false;
    }
  };

  /** 查询：重置页码，触发加载（筛选条件已由父组件同步到 queryParams） */
  const handleSearch = () => {
    page.value = 1;
    loadData();
  };

  /** 重置过滤：清空筛选条件 + 重置页码，触发加载 */
  const handleReset = () => {
    queryParams.value = {
      ...queryParams.value,
      voucher_no: '',
      start_date: '',
      end_date: '',
      voucher_type: '',
      status: '',
    };
    page.value = 1;
    loadData();
  };

  /** 监听 entries 变化仅用于保持引用稳定（合计为 computed 派生，无需手动重算） */
  watch(
    () => form.entries,
    () => {
      // no-op：合计已由 totalDebit/totalCredit computed 实时派生
    },
    { deep: true }
  );

  // 使用 reactive 包装所有 ref 字段，访问 reactive 字段时自动解包 ref，
  // 父组件通过 vchr.tableData 即可直接获得 VoucherEntity[] 类型的值
  return reactive({
    tableData,
    total,
    loading,
    page,
    pageSize,
    queryParams,
    dialogTitle,
    form,
    viewData,
    voucherTypes,
    accountSubjectOptions,
    totalDebit,
    totalCredit,
    viewTotalDebit,
    viewTotalCredit,
    loadData,
    handleSearch,
    handleReset,
    openAddDialog,
    openEditDialog,
    handleSubmit,
    addEntry,
    removeEntry,
    openViewDialog,
    loadVoucherTypes,
    loadAccountSubjects,
  });
}
