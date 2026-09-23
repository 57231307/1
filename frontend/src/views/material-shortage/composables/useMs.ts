/**
 * useMs.ts - 物料缺料核心 composable
 * 任务编号: P14 批 2 I-3 第 5 批（拆分原 material-shortage/index.vue）
 * 提供汇总 / 列表 / 分页 / 过滤等核心方法
 * 业务流程（触发检查 / 预警状态推进 / 筛选）由 useMsProc 提供
 */
import { ref, reactive } from 'vue';
import { ElMessage } from 'element-plus';
import { msg } from '@/utils/message';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import {
  getMaterialShortageSummary,
  getReplenishmentSuggestions,
  type MaterialShortageSummary,
  type MaterialShortageAlert,
  type ReplenishmentSuggestion,
} from '@/api/material-shortage';
import { useTableApi } from '@/composables/useTableApi';
import { logger } from '@/utils/logger';

/**
 * 物料短缺主业务 composable
 * 集中管理汇总、列表、分页、过滤
 */
export function useMs() {
  // 过滤（取值域见 @/constants/shortage，与后端筛选参数同名）
  const filterLevel = ref('');
  const filterStatus = ref('');

  // 加载状态
  const checking = ref(false);

  // 列表数据接入 useTableApi
  // 后端 list_shortage_alerts 返回 PaginatedResponse（data.items + total），
  // 显式钉 listKey='items'，不依赖 detectList 的 list→items 顺序探测。
  const {
    data: shortageList,
    total,
    loading: tableLoading,
    page: currentPage,
    pageSize,
    queryParams,
    refresh: fetchShortages,
  } = useTableApi<MaterialShortageAlert>({
    url: '/material-shortage/list',
    listKey: 'items',
    defaultPageSize: 10,
    defaultParams: {
      level: '',
      status: '',
    },
    onError: (err: unknown) => {
      logger.error('获取缺料列表失败', err);
      msg.error('loadMaterialShortageFailed');
    },
  });

  // 数据
  const summary = ref<MaterialShortageSummary>({} as MaterialShortageSummary);

  /** 补货建议（后端按实时缺料计算，缺料清单变化后需重新拉取） */
  const suggestions = ref<ReplenishmentSuggestion[]>([]);
  const suggestionsLoading = ref(false);

  const fetchSuggestions = async () => {
    suggestionsLoading.value = true;
    try {
      const res = await getReplenishmentSuggestions();
      suggestions.value = res.data?.suggestions ?? [];
    } catch (error) {
      const errMsg = error instanceof Error ? error.message : '获取补货建议失败';
      logger.error(errMsg);
      ElMessage.error(errMsg);
      suggestions.value = [];
    } finally {
      suggestionsLoading.value = false;
    }
  };

  /**
   * 加载汇总
   */
  const fetchSummary = async () => {
    try {
      const res = await getMaterialShortageSummary();
      summary.value = (res.data || {}) as MaterialShortageSummary;
    } catch (error) {
      const errMsg = error instanceof Error ? error.message : '获取缺料汇总失败';
      logger.error(errMsg);
      ElMessage.error(errMsg);
      summary.value = {} as MaterialShortageSummary;
    }
  };

  /** 同步 filterLevel/filterStatus 到 queryParams（空串表示不筛选该维度） */
  const syncFilterToQuery = () => {
    queryParams.value = {
      ...queryParams.value,
      level: filterLevel.value,
      status: filterStatus.value,
    };
  };

  // 懒加载标记
  const hasLoaded = createLazyLoader();

  // 使用 reactive 包装，父组件可直接访问字段
  return reactive({
    // 分页
    currentPage,
    pageSize,
    total,
    // 过滤
    filterLevel,
    filterStatus,
    queryParams,
    // 加载状态
    tableLoading,
    checking,
    // 数据
    summary,
    shortageList,
    suggestions,
    suggestionsLoading,
    // 加载方法
    fetchSummary,
    fetchShortages,
    fetchSuggestions,
    syncFilterToQuery,
    // 懒加载标记
    hasLoaded,
    // 兼容旧名
    loadIfNot,
  });
}
