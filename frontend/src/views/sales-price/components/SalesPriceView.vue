<!--
  SalesPriceView.vue - 销售价格查看详情对话框
  拆分自 sales-price/index.vue（P14 批 2 I-3 第 3 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('salesPrice.view.dialogTitle')"
    width="600px"
    :aria-label="t('salesPrice.view.dialogAriaLabel')"
    @update:model-value="onVisibleChange"
  >
    <el-descriptions :column="2" border>
      <!--
        产品名称/客户两项的数据源 = 详情行对象（列表富化行 SalesPriceView，后端 LEFT JOIN 产出）。
        两表无外键 ⇒ 孤儿引用/无客户标准价行的名称为 NULL（后端 Option<String>），
        el-descriptions 将 null 渲染为空白——如实呈现，禁止 '未知'/'-' 造名或本地缓存回填。
        "备注"项仍不存在：sales_price::Model 与 SalesPriceView 均无 remark/remarks 列
        （建表 DDL m0011 sales_prices 逐列核对无该列；审批入参 ApprovePriceRequest.remark 是入参
        且仅入 tracing 日志不落库，不是出参）。
      -->
      <el-descriptions-item :label="t('salesPrice.view.labelProductName')">{{
        viewData.product_name
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('salesPrice.view.labelCustomer')">{{
        viewData.customer_name
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('salesPrice.view.labelPrice')">{{
        formatCurrency(viewData.price)
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('salesPrice.view.labelCurrency')">{{
        viewData.currency
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('salesPrice.view.labelUnit')">{{
        viewData.unit
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('salesPrice.view.labelMinOrderQty')">{{
        viewData.min_order_qty || '-'
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('salesPrice.view.labelPriceType')">{{
        viewData.price_type ? getPriceTypeLabel(viewData.price_type) : ''
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('salesPrice.view.labelPriceLevel')">{{
        viewData.price_level || '-'
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('salesPrice.view.labelEffectiveDate')">{{
        viewData.effective_date || '-'
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('salesPrice.view.labelExpiryDate')">{{
        viewData.expiry_date || '-'
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('salesPrice.view.labelStatus')">
        <el-tag :type="getStatusType(viewData.status)">{{
          getStatusLabel(viewData.status)
        }}</el-tag>
      </el-descriptions-item>
    </el-descriptions>
  </el-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
// 类型/状态文案与货币格式化统一走 spFmts（词表权威 models/status/sales.rs::price_approval），删除组件局部 map；
// price/min_order_qty 按后端 Decimal→JSON 字符串如实声明
import {
  formatCurrency,
  getStatusType,
  getStatusLabel,
  getPriceTypeLabel,
} from '../composables/spFmts';

const { t } = useI18n({ useScope: 'global' });

// 查看详情数据类型（本局部接口按传入行对象收窄：详情与列表共用同一行
// （useSpProc.handleView 直接把列表行交给本对话框，不单独调 get_price），
// 名列 product_name/customer_name 与 api/sales-price.ts SalesPriceRow 的可空性对齐——
// 后端 SalesPriceView 为 Option<String> ⇒ string | null 必键可空值；
// remarks 不声明：后端任何读路径都无该列（详见模板内注释）。
interface SpViewData {
  product_name: string | null;
  customer_name: string | null;
  price?: string;
  currency?: string;
  unit?: string;
  min_order_qty?: string;
  price_type?: string;
  price_level?: string;
  effective_date?: string;
  // 后端为 Option<NaiveDate>（models/sales_price.rs::Model.expiry_date）⇒ 响应可为 null，本局部类型如实放宽为
  // `string | null`，与 api/sales-price.ts 的 SalesPrice.expiry_date: string | null 声明一致；
  // null 在本组件渲染为 '-'，
  // 与 el-table 各 prop="expiry_date" 字符串列把 null 渲染为空白同为既有空态呈现。
  expiry_date?: string | null;
  status?: string;
}

/**
 * 销售价格查看详情对话框组件
 */
defineProps<{
  // 对话框可见性
  visible: boolean;
  // 详情数据
  viewData: SpViewData;
}>();

const emit = defineEmits<{
  'update:visible': [v: boolean];
}>();

/** 关闭对话框 */
const onVisibleChange = (v: boolean) => {
  emit('update:visible', v);
};
</script>
