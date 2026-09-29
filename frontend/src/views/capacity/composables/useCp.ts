// capacity 主业务 composable
// 拆分自 capacity/index.vue（P14 批 2 I-3 第 6 批）
// 业务领域：产能管理（summary/trend/workCenters/bottlenecks + 分页 + 过滤）
// 行为完全保持一致（仅结构重构）
// 批次 288：workCenters 接入 useTableApi，移除手写分页逻辑
import { reactive, ref, computed } from 'vue';
import { ElMessage } from 'element-plus';
import { msg } from '@/utils/message';
import {
  getCapacitySummary,
  getCapacityTrend,
  getCapacityBottlenecks,
  getLoadAnalysis,
} from '@/api/capacity';
import type {
  CapacitySummary,
  WorkCenter,
  CapacityTrend,
  CapacityLoadItem,
  CapacityRow,
} from '@/api/capacity';
import { useTableApi } from '@/composables/useTableApi';
import { logger } from '@/utils/logger';

const toDateInput = (d: Date): string => d.toISOString().slice(0, 10);

/** capacity 主业务 composable（返回 reactive 包装的字段，父组件可直接 .字段 解包） */
export const useCp = () => {
  // 日期范围 + 趋势天数 + 工作中心筛选
  const dateRange = ref<[Date, Date] | null>(null);
  const trendDays = ref(7);
  const selectedWorkCenter = ref<number | undefined>(undefined);

  // 列表数据接入 useTableApi
  const {
    data: workCenters,
    total,
    loading: tableLoading,
    page: currentPage,
    pageSize,
    queryParams,
    refresh: fetchWorkCenters,
  } = useTableApi<WorkCenter>({
    url: '/production/capacity/work-centers',
    // 后端返回 data 为裸数组；useTableApi 会把裸数组归一为 {data: [...]}，故显式钉 listKey='data'。
    listKey: 'data',
    defaultPageSize: 10,
    onError: (err: unknown) => {
      logger.error('获取工作中心列表失败:', err);
      msg.error('loadWorkCenterFailed');
    },
  });

  // 业务数据
  const summary = ref<CapacitySummary>({} as CapacitySummary);
  const bottlenecks = ref<CapacityLoadItem[]>([]);
  const bottleneckLoading = ref(false);
  // 负荷分析数据（与 work-centers 列表按 id 关联合并进产能表）
  const loadItems = ref<CapacityLoadItem[]>([]);

  // 产能表合并行：/capacity/work-centers（产能定义）+ /capacity/load-analysis（负荷度量）
  // 无对应负荷记录的启用中工作中心其负荷列为 null（真实缺省，非缺键）。
  const capacityRows = computed<CapacityRow[]>(() => {
    const byId = new Map<number, CapacityLoadItem>(
      loadItems.value.map(item => [item.work_center_id, item])
    );
    return workCenters.value.map(wc => {
      const load = byId.get(wc.id);
      return {
        ...wc,
        total_demand: load ? load.total_demand : null,
        load_rate: load ? load.load_rate : null,
        load_status: load ? load.status : null,
        bottleneck: load ? load.status === 'OVERLOADED' : false,
      };
    });
  });

  // 趋势数据（传给 CapacityTrend 用于渲染 ECharts）
  const trendData = ref<CapacityTrend[]>([]);

  // 获取概览
  const fetchSummary = async () => {
    try {
      const res = await getCapacitySummary();
      // 安全检查：防止后端返回 data 为 null 时崩溃
      if (res.data) summary.value = res.data;
    } catch (error: unknown) {
      ElMessage.error(
        (error instanceof Error ? error.message : '') || msg.translate('loadCapacityOverviewFailed')
      );
      summary.value = {} as CapacitySummary;
    }
  };

  // 获取趋势数据
  const fetchTrendData = async () => {
    try {
      const res = await getCapacityTrend({
        days: trendDays.value,
        work_center_id: selectedWorkCenter.value,
      });
      // 安全检查：防止后端返回 data 为 null 时崩溃
      if (res.data) trendData.value = res.data;
    } catch (error: unknown) {
      ElMessage.error(
        (error instanceof Error ? error.message : '') || msg.translate('loadCapacityTrendFailed')
      );
    }
  };

  // 获取瓶颈分析
  const fetchBottlenecks = async () => {
    bottleneckLoading.value = true;
    try {
      const res = await getCapacityBottlenecks();
      // 安全检查：防止后端返回 data 为 null 时崩溃
      if (res.data) bottlenecks.value = res.data;
    } catch (error: unknown) {
      ElMessage.error(
        (error instanceof Error ? error.message : '') ||
          msg.translate('loadBottleneckAnalysisFailed')
      );
      bottlenecks.value = [];
    } finally {
      bottleneckLoading.value = false;
    }
  };

  // 获取负荷分析（供产能表负荷列使用；随日期区间过滤，未选日期则取全量）
  const fetchLoadAnalysis = async () => {
    try {
      const params: { date_from?: string; date_to?: string } = {};
      if (dateRange.value) {
        params.date_from = toDateInput(dateRange.value[0]);
        params.date_to = toDateInput(dateRange.value[1]);
      }
      const res = await getLoadAnalysis(params);
      // 安全检查：防止后端返回 data 为 null 时崩溃
      if (res.data) loadItems.value = res.data;
      else loadItems.value = [];
    } catch (error: unknown) {
      logger.error('获取产能负荷分析失败:', error);
      msg.error('loadCapacityOverviewFailed');
    }
  };

  // 日期变化 → 重新拉取趋势 + 负荷分析
  const handleDateChange = () => {
    fetchTrendData();
    fetchLoadAnalysis();
  };

  // 工作中心变化 → 重新拉取趋势
  const handleWorkCenterChange = () => {
    fetchTrendData();
  };

  // 趋势天数变化 → 重新拉取趋势
  const handleTrendDaysChange = () => {
    fetchTrendData();
  };

  // 初始化挂载：工作中心列表由 useTableApi setup 自动加载，此处仅拉取辅助数据
  const initOnMount = async () => {
    await Promise.all([fetchSummary(), fetchTrendData(), fetchBottlenecks(), fetchLoadAnalysis()]);
  };

  return reactive({
    dateRange,
    trendDays,
    selectedWorkCenter,
    currentPage,
    pageSize,
    total,
    queryParams,
    summary,
    workCenters,
    capacityRows,
    loadItems,
    bottlenecks,
    tableLoading,
    bottleneckLoading,
    trendData,
    fetchSummary,
    fetchTrendData,
    fetchWorkCenters,
    fetchBottlenecks,
    fetchLoadAnalysis,
    handleDateChange,
    handleWorkCenterChange,
    handleTrendDaysChange,
    initOnMount,
  });
};
