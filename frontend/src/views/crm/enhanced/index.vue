<template>
  <div class="crm-enhanced-page">
    <el-card shadow="never">
      <div class="page-header">
        <h2>CRM 高级分析</h2>
      </div>

      <el-tabs v-model="activeTab">
        <!-- 页签一：销售漏斗 -->
        <el-tab-pane label="销售漏斗" name="funnel">
          <div class="toolbar">
            <el-date-picker
              v-model="funnelDateRange"
              type="daterange"
              value-format="YYYY-MM-DD"
              start-placeholder="开始日期"
              end-placeholder="结束日期"
              style="width: 260px"
            />
            <el-button type="primary" :loading="funnelLoading" @click="loadSalesFunnel">
              查询漏斗
            </el-button>
            <el-button :loading="forecastLoading" @click="loadWeightedForecast">加权预测</el-button>
          </div>

          <el-table v-loading="funnelLoading" :data="funnelRows" border>
            <el-table-column prop="stage" label="阶段" min-width="140" />
            <el-table-column prop="count" label="数量" width="120" align="right">
              <template #default="{ row }">{{ row.count ?? '-' }}</template>
            </el-table-column>
            <el-table-column label="金额" width="180" align="right">
              <template #default="{ row }">{{ fmtAmount(row.amount) }}</template>
            </el-table-column>
            <el-table-column label="向下转化率" width="140" align="right">
              <template #default="{ row }">{{ fmtRate(row.rate) }}</template>
            </el-table-column>
          </el-table>

          <h3 class="section-title">加权销售预测</h3>
          <el-descriptions v-if="forecast" :column="3" border class="block-gap">
            <el-descriptions-item label="商机总数">
              {{ forecast.total_opportunities }}
            </el-descriptions-item>
            <el-descriptions-item label="预估总额">
              {{ fmtAmount(forecast.total_estimated_amount) }}
            </el-descriptions-item>
            <el-descriptions-item label="加权总额">
              {{ fmtAmount(forecast.total_weighted_amount) }}
            </el-descriptions-item>
          </el-descriptions>
          <el-table v-loading="forecastLoading" :data="forecastRows" border>
            <el-table-column prop="opportunity_no" label="商机编号" min-width="140" />
            <el-table-column
              prop="opportunity_name"
              label="商机名称"
              min-width="160"
              show-overflow-tooltip
            />
            <el-table-column prop="stage" label="阶段" width="150" />
            <el-table-column label="预估金额" width="140" align="right">
              <template #default="{ row }">{{ fmtAmount(row.estimated_amount) }}</template>
            </el-table-column>
            <el-table-column label="赢率" width="100" align="right">
              <template #default="{ row }">{{ fmtRate(row.win_probability) }}</template>
            </el-table-column>
            <el-table-column label="加权金额" width="140" align="right">
              <template #default="{ row }">{{ fmtAmount(row.weighted_amount) }}</template>
            </el-table-column>
            <el-table-column label="预计成交日期" width="130">
              <template #default="{ row }">{{ row.expected_close_date || '-' }}</template>
            </el-table-column>
          </el-table>
        </el-tab-pane>

        <!-- 页签二：线索工具 -->
        <el-tab-pane label="线索工具" name="tools">
          <h3 class="section-title">重复线索检测</h3>
          <div class="toolbar">
            <el-input
              v-model="dupMobile"
              placeholder="手机号（选填）"
              clearable
              style="width: 200px"
            />
            <el-input
              v-model="dupCompany"
              placeholder="公司名称（选填）"
              clearable
              style="width: 200px"
            />
            <el-button type="primary" :loading="dupLoading" @click="handleDetectDuplicates">
              查重
            </el-button>
          </div>
          <el-table v-loading="dupLoading" :data="dupGroups" border>
            <el-table-column prop="match_key" label="匹配键" min-width="140" />
            <el-table-column prop="match_type" label="匹配类型" width="130" />
            <el-table-column prop="count" label="重复数" width="90" align="center" />
            <el-table-column label="线索编号" min-width="160">
              <template #default="{ row }">{{ (row.lead_nos ?? []).join('、') || '-' }}</template>
            </el-table-column>
            <el-table-column label="公司名称" min-width="160">
              <template #default="{ row }">
                {{ (row.company_names ?? []).join('、') || '-' }}
              </template>
            </el-table-column>
            <el-table-column label="操作" width="100" fixed="right">
              <template #default="{ row }">
                <el-button size="small" link type="primary" @click="openMergeDialog(row)">
                  合并
                </el-button>
              </template>
            </el-table-column>
          </el-table>

          <h3 class="section-title">线索转化漏斗报告</h3>
          <div class="toolbar">
            <el-date-picker
              v-model="reportDateRange"
              type="daterange"
              value-format="YYYY-MM-DD"
              start-placeholder="开始日期"
              end-placeholder="结束日期"
              style="width: 260px"
            />
            <el-button type="primary" :loading="reportLoading" @click="loadLeadFunnelReport">
              漏斗报告
            </el-button>
          </div>
          <el-descriptions v-if="leadFunnelReport" :column="2" border class="block-gap">
            <el-descriptions-item label="线索总数">{{
              leadFunnelReport.total_leads
            }}</el-descriptions-item>
            <el-descriptions-item label="已转化线索">
              {{ leadFunnelReport.converted_leads }}
            </el-descriptions-item>
            <el-descriptions-item label="商机总数">
              {{ leadFunnelReport.total_opportunities }}
            </el-descriptions-item>
            <el-descriptions-item label="赢单商机">
              {{ leadFunnelReport.won_opportunities }}
            </el-descriptions-item>
            <el-descriptions-item label="客户总数">
              {{ leadFunnelReport.total_customers }}
            </el-descriptions-item>
            <el-descriptions-item label="订单总数">{{
              leadFunnelReport.total_orders
            }}</el-descriptions-item>
            <el-descriptions-item label="线索→商机">
              {{ fmtRate(leadFunnelReport.lead_to_opp_rate) }}
            </el-descriptions-item>
            <el-descriptions-item label="商机→客户">
              {{ fmtRate(leadFunnelReport.opp_to_customer_rate) }}
            </el-descriptions-item>
            <el-descriptions-item label="商机→订单">
              {{ fmtRate(leadFunnelReport.opp_to_order_rate) }}
            </el-descriptions-item>
            <el-descriptions-item label="整体转化率">
              {{ fmtRate(leadFunnelReport.overall_conversion_rate) }}
            </el-descriptions-item>
          </el-descriptions>
        </el-tab-pane>

        <!-- 页签三：客户价值 -->
        <el-tab-pane label="客户价值" name="clv">
          <div class="toolbar">
            <el-input-number
              v-model="clvCustomerId"
              :min="1"
              :precision="0"
              placeholder="客户 ID"
              style="width: 180px"
            />
            <el-select
              v-model="auditOperation"
              placeholder="全部操作"
              clearable
              style="width: 140px"
              @change="handleAuditOperationChange"
            >
              <el-option v-for="op in operationOptions" :key="op" :label="op" :value="op" />
            </el-select>
            <el-button type="primary" :loading="clvLoading" @click="loadCustomerValue">
              查询
            </el-button>
          </div>

          <h3 class="section-title">客户全生命周期价值（CLV）</h3>
          <el-empty v-if="!clv" description="暂无 CLV 数据" :image-size="60" />
          <el-descriptions v-else :column="3" border>
            <el-descriptions-item label="订单总数">{{ clv.total_orders }}</el-descriptions-item>
            <el-descriptions-item label="累计收入">{{
              fmtAmount(clv.total_revenue)
            }}</el-descriptions-item>
            <el-descriptions-item label="平均订单价值">
              {{ fmtAmount(clv.avg_order_value) }}
            </el-descriptions-item>
            <el-descriptions-item label="首单日期">{{
              clv.first_order_date || '-'
            }}</el-descriptions-item>
            <el-descriptions-item label="最近订单日期">
              {{ clv.last_order_date || '-' }}
            </el-descriptions-item>
            <el-descriptions-item label="生命周期(天)">
              {{ clv.customer_lifespan_days }}
            </el-descriptions-item>
            <el-descriptions-item label="购买频率">{{
              clv.purchase_frequency
            }}</el-descriptions-item>
            <el-descriptions-item label="CLV 得分">{{ clv.clv_score }}</el-descriptions-item>
            <el-descriptions-item label="客户分层">
              <el-tag>{{ clv.segment || '-' }}</el-tag>
            </el-descriptions-item>
          </el-descriptions>

          <h3 class="section-title">地址簿</h3>
          <el-table v-loading="clvLoading" :data="addresses" border>
            <el-table-column prop="contact_name" label="收货人" width="110" />
            <el-table-column prop="contact_phone" label="联系电话" width="130" />
            <el-table-column label="省市区" min-width="150">
              <template #default="{ row }">
                {{ [row.province, row.city, row.district].filter(Boolean).join(' / ') || '-' }}
              </template>
            </el-table-column>
            <el-table-column
              prop="address"
              label="详细地址"
              min-width="180"
              show-overflow-tooltip
            />
            <el-table-column prop="postal_code" label="邮编" width="100">
              <template #default="{ row }">{{ row.postal_code || '-' }}</template>
            </el-table-column>
            <el-table-column label="默认地址" width="100" align="center">
              <template #default="{ row }">
                <el-tag v-if="row.is_default" type="success" size="small">默认</el-tag>
                <span v-else>-</span>
              </template>
            </el-table-column>
            <el-table-column prop="remark" label="备注" min-width="140" show-overflow-tooltip>
              <template #default="{ row }">{{ row.remark || '-' }}</template>
            </el-table-column>
          </el-table>

          <h3 class="section-title">审计日志</h3>
          <el-table v-loading="clvLoading" :data="auditLogs" border>
            <el-table-column prop="operation" label="操作类型" width="110" />
            <el-table-column prop="field_name" label="字段" width="130">
              <template #default="{ row }">{{ row.field_name || '-' }}</template>
            </el-table-column>
            <el-table-column prop="old_value" label="旧值" min-width="130" show-overflow-tooltip>
              <template #default="{ row }">{{ row.old_value || '-' }}</template>
            </el-table-column>
            <el-table-column prop="new_value" label="新值" min-width="130" show-overflow-tooltip>
              <template #default="{ row }">{{ row.new_value || '-' }}</template>
            </el-table-column>
            <el-table-column prop="user_name" label="操作人" width="110" />
            <el-table-column prop="ip_address" label="IP 地址" width="130">
              <template #default="{ row }">{{ row.ip_address || '-' }}</template>
            </el-table-column>
            <el-table-column prop="created_at" label="操作时间" min-width="160">
              <template #default="{ row }">{{ row.created_at || '-' }}</template>
            </el-table-column>
          </el-table>
        </el-tab-pane>
      </el-tabs>
    </el-card>

    <!-- 合并重复线索弹窗 -->
    <el-dialog v-model="mergeVisible" title="合并重复线索" width="440px">
      <el-form label-width="100px">
        <el-form-item label="主线索">
          <el-select v-model="mergePrimaryId" placeholder="选择保留的主线索" style="width: 100%">
            <el-option
              v-for="item in mergeOptions"
              :key="item.id"
              :label="item.label"
              :value="item.id"
            />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="mergeVisible = false">取消</el-button>
        <el-button type="primary" :loading="mergeLoading" @click="submitMerge">确定合并</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { ElMessage } from 'element-plus';
import {
  detectDuplicateLeads,
  getLeadFunnelReport,
  getSalesFunnel,
  getWeightedForecast,
  mergeLeads,
  type DuplicateLeadGroup,
  type LeadFunnelReport,
  type SalesFunnelReport,
  type WeightedForecastResult,
} from '@/api/crm';
import {
  getCustomerAddressList,
  getCustomerAuditLogs,
  getCustomerClv,
  type CustomerAddress,
  type CustomerAuditLog,
  type CustomerClv,
} from '@/api/customer';

const activeTab = ref('funnel');

/** 响应解包防御：兼容数组 / { items } / { history } 包装，避免 el-table "r is not iterable" */
const unwrapList = <T,>(payload: unknown): T[] => {
  if (Array.isArray(payload)) return payload as T[];
  const paged = payload as { items?: T[]; history?: T[] } | null;
  return paged?.items ?? paged?.history ?? [];
};

const fmtAmount = (value: number | null | undefined): string =>
  value == null ? '-' : Number(value).toLocaleString('zh-CN', { minimumFractionDigits: 2 });

const fmtRate = (value: number | null | undefined): string =>
  value == null ? '-' : `${Number(value).toFixed(1)}%`;

// ============== 页签一：销售漏斗 ==============

const funnelDateRange = ref<[string, string] | null>(null);
const funnelLoading = ref(false);
const funnelReport = ref<SalesFunnelReport | null>(null);

interface FunnelRow {
  stage: string;
  count: number | null;
  amount: number | null;
  rate: number | null;
}

const funnelRows = computed<FunnelRow[]>(() => {
  const r = funnelReport.value;
  if (!r) return [];
  return [
    { stage: '线索', count: r.lead_count, amount: null, rate: r.lead_to_opp_rate },
    {
      stage: '商机',
      count: r.opportunity_count,
      amount: r.opportunity_amount,
      rate: r.opp_to_quotation_rate,
    },
    { stage: '报价单', count: r.quotation_count, amount: null, rate: r.opp_to_order_rate },
    { stage: '赢单商机', count: r.won_count, amount: r.won_amount, rate: null },
    {
      stage: '销售订单',
      count: r.order_count,
      amount: r.order_amount,
      rate: r.order_to_collection_rate,
    },
    { stage: '回款', count: null, amount: r.collected_amount, rate: null },
  ];
});

const loadSalesFunnel = async () => {
  funnelLoading.value = true;
  try {
    const res = await getSalesFunnel({
      start_date: funnelDateRange.value?.[0],
      end_date: funnelDateRange.value?.[1],
    });
    funnelReport.value = res.data ?? null;
  } catch {
    ElMessage.error('加载销售漏斗失败');
  } finally {
    funnelLoading.value = false;
  }
};

const forecastLoading = ref(false);
const forecast = ref<WeightedForecastResult | null>(null);
const forecastRows = computed(() => forecast.value?.details ?? []);

const loadWeightedForecast = async () => {
  forecastLoading.value = true;
  try {
    const res = await getWeightedForecast();
    forecast.value = res.data ?? null;
  } catch {
    ElMessage.error('加载加权预测失败');
  } finally {
    forecastLoading.value = false;
  }
};

// ============== 页签二：线索工具 ==============

const dupMobile = ref('');
const dupCompany = ref('');
const dupLoading = ref(false);
const dupGroups = ref<DuplicateLeadGroup[]>([]);

const handleDetectDuplicates = async () => {
  if (!dupMobile.value && !dupCompany.value) {
    ElMessage.warning('请至少填写手机号或公司名称');
    return;
  }
  dupLoading.value = true;
  try {
    const res = await detectDuplicateLeads({
      mobile_phone: dupMobile.value || undefined,
      company_name: dupCompany.value || undefined,
    });
    dupGroups.value = unwrapList<DuplicateLeadGroup>(res.data);
  } catch {
    ElMessage.error('重复线索检测失败');
  } finally {
    dupLoading.value = false;
  }
};

const mergeVisible = ref(false);
const mergeLoading = ref(false);
const mergeGroup = ref<DuplicateLeadGroup | null>(null);
const mergePrimaryId = ref<number | undefined>(undefined);

const mergeOptions = computed(() => {
  const group = mergeGroup.value;
  if (!group) return [];
  return group.lead_ids.map((id, idx) => ({
    id,
    label: `${group.lead_nos[idx] ?? `#${id}`} ${group.company_names[idx] ?? ''}`.trim(),
  }));
});

const openMergeDialog = (group: DuplicateLeadGroup) => {
  mergeGroup.value = group;
  mergePrimaryId.value = group.lead_ids[0];
  mergeVisible.value = true;
};

const submitMerge = async () => {
  const group = mergeGroup.value;
  const primaryId = mergePrimaryId.value;
  if (!group || !primaryId) {
    ElMessage.warning('请选择主线索');
    return;
  }
  const duplicateIds = group.lead_ids.filter(id => id !== primaryId);
  if (duplicateIds.length === 0) {
    ElMessage.warning('该组仅有一条线索，无需合并');
    return;
  }
  mergeLoading.value = true;
  try {
    const res = await mergeLeads({ primary_id: primaryId, duplicate_ids: duplicateIds });
    ElMessage.success(
      `合并成功：${res.data?.merged_count ?? 0} 条线索并入 ${res.data?.master_lead_no ?? ''}`
    );
    mergeVisible.value = false;
    await handleDetectDuplicates();
  } catch {
    ElMessage.error('合并线索失败');
  } finally {
    mergeLoading.value = false;
  }
};

const reportDateRange = ref<[string, string] | null>(null);
const reportLoading = ref(false);
const leadFunnelReport = ref<LeadFunnelReport | null>(null);

const loadLeadFunnelReport = async () => {
  reportLoading.value = true;
  try {
    const res = await getLeadFunnelReport({
      start_date: reportDateRange.value?.[0],
      end_date: reportDateRange.value?.[1],
    });
    leadFunnelReport.value = res.data ?? null;
  } catch {
    ElMessage.error('加载漏斗报告失败');
  } finally {
    reportLoading.value = false;
  }
};

// ============== 页签三：客户价值 ==============

const operationOptions = ['create', 'update', 'delete', 'view', 'export'];
const clvCustomerId = ref<number | undefined>(undefined);
const auditOperation = ref('');
const clvLoading = ref(false);
const clv = ref<CustomerClv | null>(null);
const addresses = ref<CustomerAddress[]>([]);
const auditLogs = ref<CustomerAuditLog[]>([]);

const loadCustomerValue = async () => {
  const customerId = clvCustomerId.value;
  if (!customerId) {
    ElMessage.warning('请输入客户 ID');
    return;
  }
  clvLoading.value = true;
  const tasks = [
    getCustomerClv(customerId)
      .then(res => {
        clv.value = res.data ?? null;
      })
      .catch(() => {
        ElMessage.error('加载客户 CLV 失败');
      }),
    getCustomerAddressList(customerId)
      .then(res => {
        addresses.value = unwrapList<CustomerAddress>(res.data);
      })
      .catch(() => {
        ElMessage.error('加载地址簿失败');
      }),
    getCustomerAuditLogs(customerId, { operation: auditOperation.value || undefined })
      .then(res => {
        auditLogs.value = unwrapList<CustomerAuditLog>(res.data);
      })
      .catch(() => {
        ElMessage.error('加载审计日志失败');
      }),
  ];
  await Promise.all(tasks);
  clvLoading.value = false;
};

const handleAuditOperationChange = () => {
  if (clvCustomerId.value) loadCustomerValue();
};

onMounted(() => {
  loadSalesFunnel();
  loadWeightedForecast();
});
</script>

<style scoped>
.crm-enhanced-page {
  padding: 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.page-header h2 {
  margin: 0;
  font-size: 18px;
}

.section-title {
  margin: 20px 0 12px;
  font-size: 15px;
  font-weight: 600;
}

.toolbar {
  display: flex;
  gap: 12px;
  align-items: center;
  margin-bottom: 12px;
}

.block-gap {
  margin-bottom: 12px;
}
</style>
