/**
 * useVchr.ts - 凭证管理核心 composable
 * 任务编号: P14 批 1 B3 I-2（拆分原 VoucherTab.vue）
 * 提供凭证列表查询、表单管理、科目加载等核心方法
 * 流程操作（提交/审核/过账/导出/打印）由 useVchrProc 提供
 * 行为完全保持一致（仅结构重构）
 * 批次 289：vouchers 接入 useTableApi，移除手写分页逻辑，返回 reactive 包装
 */
import { ref, reactive, computed } from 'vue';
import { ElMessage } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import { msg } from '@/utils/message';
import {
  getSubjectTree,
  createVoucher,
  getVoucher,
  getVoucherTypesApi,
  type AccountSubject,
  type Voucher,
} from '@/api/finance';
import { useTableApi } from '@/composables/useTableApi';
import { logger } from '@/utils/logger';
import { formatMoney, getVchrStatusLabel, getVchrStatusType } from './vchrFmts';

/**
 * 凭证管理 composable
 * 集中管理凭证列表、表单、科目等业务状态
 */
export function useVchr() {
  // 列表数据接入 useTableApi
  // 凭证 API 返回 ApiResponse<Voucher[]>，data 为裸数组；useTableApi detectList 兼容裸数组
  // 分页参数使用 snake_case（page/page_size），匹配 useTableApi 默认配置
  const {
    data: vouchers,
    total: voucherTotal,
    loading: voucherLoading,
    page,
    pageSize,
    queryParams,
    refresh: fetchVouchers,
  } = useTableApi<Voucher>({
    url: '/vouchers',
    defaultPageSize: 20,
    defaultParams: {
      voucher_no: '',
      date_range: [] as string[],
      status: '',
    },
    onError: (err: unknown) => {
      logger.error('获取凭证列表失败', err);
      msg.error('loadVoucherListFailed');
    },
  });

  const subjects = ref<AccountSubject[]>([]);

  // 凭证类型选项（P0 三端同源：消费 GET /vouchers/types，
  // 后端 VoucherService::available_voucher_types 词表 code=记/收/付/转；
  // 原表单硬编码 'JZ'/'SK' 等编号前缀非类型词表值，已从前端移除）
  const voucherTypes = ref<{ label: string; value: string }[]>([]);
  const loadVoucherTypes = async () => {
    try {
      const res = await getVoucherTypesApi();
      voucherTypes.value = (Array.isArray(res.data) ? res.data : []).map(t => ({
        label: t.name,
        value: t.code,
      }));
    } catch (error) {
      logger.error('获取凭证类型失败', error);
    }
  };

  // 表单相关
  const voucherFormRef = ref<FormInstance>();
  const voucherSubmitLoading = ref(false);
  const voucherForm = reactive({
    voucher_date: '',
    voucher_type: '',
    entries: [
      { subject_id: undefined as number | undefined, debit: 0, credit: 0, summary: '' },
      { subject_id: undefined as number | undefined, debit: 0, credit: 0, summary: '' },
    ],
  });

  const voucherRules: FormRules = {
    voucher_date: [{ required: true, message: '请选择凭证日期', trigger: 'change' }],
    voucher_type: [{ required: true, message: '请选择凭证类型', trigger: 'change' }],
  };

  // 详情相关
  const currentVoucher = ref<Voucher | null>(null);

  // 叶子科目（用于凭证分录的 el-tree-select 数据源）。
  //
  // 根因（本轮取证订正）：科目树端点 `GET /subjects/tree`（handlers/account_subject_handler.rs
  // → services/account_subject_service.rs get_tree）出参为 SubjectTreeNode{id, code, name, level,
  // status, children}——后端从不输出 is_leaf（account_subjects 主键 Model 也无 is_leaf 列，见
  // models/account_subject.rs）。旧实现按 `item.is_leaf` 过滤叶子，运行期该字段恒 undefined →
  // 判定为假 → leafSubjects 恒空数组 → el-tree-select :data 为空 → 面板渲染 el-empty「无数据」，
  // 根本不存在 .el-tree-node__content → e2e pickSubjectInTreeSelect 的 node.waitFor 恒 15s 超时。
  // 故改为以真实存在的结构字段 children 判定叶子：树中「无子节点」即叶子（与账户科目层级语义一致，
  // 非兜底掩盖——children 是端点确凿返回的权威结构）。递归收集所有叶子，父节点本身不作为可选项。
  //
  // 叶子节点必须剥离 children 属性再传入 el-tree-select：后端 SubjectTreeNode.children 类型为
  // Vec<SubjectTreeNode>（非 Option），叶子序列化为 children: []。el-tree 见到 children 为数组
  // （即使空）即视为可展开分支，渲染出折叠箭头并改变 DOM 结构，导致 e2e 点击 node content 时
  // 可能触发折叠而非选中，面板收起→节点不可见。此处仅构造 tree-select 所需字段(id/code/name/level/status)。
  const leafSubjects = computed(() => {
    const flatten = (list: AccountSubject[]): AccountSubject[] =>
      list.reduce((acc, item) => {
        const hasChildren = Array.isArray(item.children) && item.children.length > 0;
        if (hasChildren) {
          acc.push(...flatten(item.children as AccountSubject[]));
        } else {
          const leaf = { ...item } as Record<string, unknown>;
          delete leaf.children;
          acc.push(leaf as unknown as AccountSubject);
        }
        return acc;
      }, [] as AccountSubject[]);
    return flatten(subjects.value);
  });

  // 借贷合计
  const totalDebit = computed(() =>
    voucherForm.entries.reduce((sum, e) => sum + (e.debit || 0), 0)
  );
  const totalCredit = computed(() =>
    voucherForm.entries.reduce((sum, e) => sum + (e.credit || 0), 0)
  );
  const isBalanced = computed(() => Math.abs(totalDebit.value - totalCredit.value) < 0.01);

  // 科目加载
  const fetchSubjects = async () => {
    try {
      const res = await getSubjectTree();
      const d = res.data as
        AccountSubject[] | { items?: AccountSubject[]; data?: AccountSubject[] };
      subjects.value = Array.isArray(d) ? d : d?.items || d?.data || [];
    } catch (error) {
      const err = error as Error;
      logger.warn('获取科目列表失败', err.message);
    }
  };

  /** 查询：重置页码，触发加载（筛选条件已由父组件同步到 queryParams） */
  const handleSearch = () => {
    page.value = 1;
    fetchVouchers();
  };

  /** 重置过滤：清空筛选条件 + 重置页码，触发加载 */
  const handleReset = () => {
    queryParams.value = {
      ...queryParams.value,
      voucher_no: '',
      date_range: [],
      status: '',
    };
    page.value = 1;
    fetchVouchers();
  };

  // 分录管理
  const addEntry = () => {
    voucherForm.entries.push({ subject_id: undefined, debit: 0, credit: 0, summary: '' });
  };

  const removeEntry = (index: number) => {
    if (voucherForm.entries.length > 2) {
      voucherForm.entries.splice(index, 1);
    } else {
      msg.warning('keepAtLeastTwoEntries');
    }
  };

  // 表单提交
  const submitVoucherForm = async () => {
    const valid = await voucherFormRef.value?.validate();
    if (!valid) return false;

    if (!isBalanced.value) {
      msg.warning('entriesUnbalanced');
      return false;
    }

    voucherSubmitLoading.value = true;
    try {
      await createVoucher({
        voucher_date: voucherForm.voucher_date,
        voucher_type: voucherForm.voucher_type,
        items: voucherForm.entries
          .filter(e => e.subject_id)
          .map(e => ({
            subject_id: e.subject_id!,
            debit: e.debit || 0,
            credit: e.credit || 0,
            summary: e.summary,
          })),
      });
      msg.success('createSuccess');
      await fetchVouchers();
      return true;
    } catch (error) {
      const err = error as Error;
      ElMessage.error(err.message || msg.translate('operationFailed'));
      return false;
    } finally {
      voucherSubmitLoading.value = false;
    }
  };

  // 详情查看
  // P0 修复（回源）：列表行不含 entries（后端 GET /vouchers 返回 Vec<voucher::Model>），
  // 原实现直接把列表行塞进详情弹窗 ⇒ 分录明细恒空、合计恒 0。现改为按 id 调详情端点，
  // 借贷合计由后端真实返回的 entries 前端求和派生（后端 vouchers 无合计列）。
  const viewVoucher = async (row: Voucher) => {
    try {
      const res = await getVoucher(row.id);
      const detail = res.data;
      const entries = detail.entries ?? [];
      detail.total_debit = entries.reduce((s, e) => s + Number(e.debit ?? 0), 0);
      detail.total_credit = entries.reduce((s, e) => s + Number(e.credit ?? 0), 0);
      currentVoucher.value = detail;
    } catch (error) {
      logger.error('获取凭证详情失败', error);
      msg.error('loadDetailFailed');
    }
  };

  // 使用 reactive 包装所有 ref 字段，访问 reactive 字段时 Vue 自动解包 ref，
  // 父组件通过 vchr.vouchers 即可直接获得 Voucher[] 类型的值
  return reactive({
    // 列表
    vouchers,
    voucherLoading,
    voucherTotal,
    page,
    pageSize,
    queryParams,
    fetchVouchers,
    handleSearch,
    handleReset,
    // 科目
    subjects,
    leafSubjects,
    fetchSubjects,
    // 凭证类型（单一真源：GET /vouchers/types）
    voucherTypes,
    loadVoucherTypes,
    // 表单
    voucherFormRef,
    voucherForm,
    voucherSubmitLoading,
    voucherRules,
    submitVoucherForm,
    addEntry,
    removeEntry,
    // 详情
    currentVoucher,
    viewVoucher,
    // 工具
    formatMoney,
    getVchrStatusLabel,
    getVchrStatusType,
    totalDebit,
    totalCredit,
    isBalanced,
  });
}
