// sales-analysis 主业务 composable
// 拆分自 sales-analysis/index.vue（P14 批 2 I-3 第 6 批）
// 业务领域：销售分析（stats + 4 个排行榜 + 趋势周期 + 排名类型 + 销售目标）
// 行为完全保持一致（仅结构重构）
import { reactive, ref, watch } from 'vue';
import {
  getSalesAnalysisStats,
  getProductRanking as fetchProductRanking,
  getCustomerRanking as fetchCustomerRanking,
  getSalesTargetList as fetchSalesTargetList,
  getSalesTrendData as fetchSalesTrendData,
  type ProductRanking,
  type CustomerRanking,
  type SalesTrendGranularity,
  type SalesTarget,
  type SalesTrendResult,
} from '@/api/sales-analysis';
import { logger } from '@/utils/logger';

/** 销售分析主业务 composable（返回 reactive 包装的字段，父组件可直接 .字段 解包） */
export const useSa = () => {
  // 统计数据
  const stats = reactive({
    monthOrders: 0,
    monthAmount: 0,
    grossProfitRate: 0,
    activeCustomers: 0,
    orderTrend: 0,
    amountTrend: 0,
    profitTrend: 0,
    customerTrend: 0,
  });

  // 趋势分桶粒度（后端按粒度对 sales_orders 现算分桶；键名与查询词表同为 granularity）
  const trendGranularity = ref<SalesTrendGranularity>('month');

  // 排名类型
  const productRankType = ref('amount');
  const customerRankType = ref('amount');

  // 产品排名
  const productRanking = ref<ProductRanking[]>([]);

  // 客户排名
  const customerRanking = ref<CustomerRanking[]>([]);

  // 销售目标
  const salesTargets = ref<SalesTarget[]>([]);

  // 销售趋势数据（批次 95 P3-20 修复：供 SalesAnalysisTrend 折线图渲染）
  const trendData = ref<SalesTrendResult[]>([]);

  // 获取统计数据
  const getStats = async () => {
    try {
      const res = await getSalesAnalysisStats();
      if (res.data) {
        Object.assign(stats, res.data);
      }
    } catch (error) {
      logger.error('获取统计数据失败:', error);
    }
  };

  // 获取产品排名
  // 后端 `type` 查询键 = dimension_type（统计维度，默认 product），不是「按金额/数量」排序开关；
  // 此前把 'amount'/'quantity' 当 type 下发 ⇒ 按不存在的维度过滤 ⇒ 榜单恒空（静默假绿形态）。
  // 排序指标为前端侧展示排序：先按后端默认（金额降序 top-N）取数，再对返回行 Number() 归一后本地排序；
  // 「按数量全局 top-N」需后端新增排序参数（已登记后端串行清单）。
  const getProductRanking = async () => {
    try {
      const res = await fetchProductRanking({ limit: 10 });
      const rows = res.data || [];
      const key = productRankType.value === 'quantity' ? 'quantity' : 'amount';
      productRanking.value = [...rows].sort((a, b) => Number(b[key]) - Number(a[key]));
    } catch (error) {
      logger.error('获取产品排名失败:', error);
    }
  };

  // 获取客户排名（同产品排名：type 是维度不是排序键；orders 即按 order_count 本地排序）
  const getCustomerRanking = async () => {
    try {
      const res = await fetchCustomerRanking({ limit: 10 });
      const rows = res.data || [];
      const key = customerRankType.value === 'orders' ? 'order_count' : 'amount';
      customerRanking.value = [...rows].sort((a, b) => Number(b[key]) - Number(a[key]));
    } catch (error) {
      logger.error('获取客户排名失败:', error);
    }
  };

  // 获取销售目标
  const getSalesTargets = async () => {
    try {
      const res = await fetchSalesTargetList();
      salesTargets.value = res.data || [];
    } catch (error) {
      logger.error('获取销售目标失败:', error);
    }
  };

  // 获取销售趋势数据：下发分桶粒度；出参契约保证 data 恒为数组，直读、不再 `|| []` 兜底
  // （兜底会把"后端缺 data 键"的契约破坏伪装成正常空态，属禁用形态）
  const getTrendData = async () => {
    try {
      const res = await fetchSalesTrendData({ granularity: trendGranularity.value });
      trendData.value = res.data;
    } catch (error) {
      logger.error('获取销售趋势数据失败:', error);
    }
  };

  // 趋势粒度变化时重新拉取趋势数据
  watch(trendGranularity, () => {
    getTrendData();
  });

  return reactive({
    stats,
    trendGranularity,
    productRankType,
    customerRankType,
    productRanking,
    customerRanking,
    salesTargets,
    trendData,
    getStats,
    getProductRanking,
    getCustomerRanking,
    getSalesTargets,
    getTrendData,
  });
};
