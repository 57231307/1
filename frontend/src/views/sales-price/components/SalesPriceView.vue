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
        此处原有"产品名称/客户名称/备注"三项，因后端读链路从不输出这三个键而删除：
        后端 sales_price::Model（models/sales_price.rs）既无名列（product_name/customer_name），
        也无 remark/remarks 列（建表 DDL migration/src/domain/business/
        m0011_add_sales_and_logistics_extensions.rs 的 sales_prices 表定义逐列核对，全列无备注）；
        sales_price_handler.rs::get_price 直接序列化 Model（find_by_id 单表查询，keyword 类
        LeftJoin 谓词也不追加 SELECT 列），信封（utils/response.rs::ApiResponse）不补键
        ⇒ 三项自始空白。其中"备注"因本组件局部 SpViewData 曾声明
        remarks? 而传入对象实为 SalesPrice（index.vue 直传 spProc.viewData，该类型从未有 remarks 键），
        配合 `|| '-'` 把不存在的键伪装成"无备注"的正常空态（掩盖型渲染，正解是删掉不存在的东西）。
        `remark` 在本域只存在于审批请求体 sales_price_handler::ApprovePriceRequest（入参不是出参，
        且仅入 tracing 日志、无落库列）。
      -->
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

// 查看详情数据类型（本局部接口按后端 sales_price::Model 逐列收窄，
// 删除幽灵键 product_name/customer_name/remarks——models/sales_price.rs::Model 无名列亦无备注列，
// 详见模板内注释；index.vue 直传的 SalesPrice 也不含这些键）
interface SpViewData {
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
