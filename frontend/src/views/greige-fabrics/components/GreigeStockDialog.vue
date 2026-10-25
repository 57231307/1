<template>
  <el-dialog
    :model-value="visible"
    :title="dialogTitle"
    width="560px"
    :close-on-click-modal="false"
    :aria-label="dialogTitle"
    data-testid="greige-stock-dialog"
    @update:model-value="emit('update:visible', $event)"
  >
    <!-- 出库前把当前库存原样摊出来：数量是否超量由后端判定，界面先把依据显示出来 -->
    <el-descriptions
      v-if="row"
      :column="2"
      size="small"
      border
      :aria-label="t('greigeFabrics.stock.ariaCurrentStock')"
      class="current-stock"
    >
      <el-descriptions-item :label="t('greigeFabrics.index.colFabricCode')">
        {{ row.fabric_no ?? '-' }}
      </el-descriptions-item>
      <el-descriptions-item :label="t('greigeFabrics.index.colStatus')">
        {{ row.status ?? '-' }}
      </el-descriptions-item>
      <el-descriptions-item :label="t('greigeFabrics.index.colWeightKg')">
        {{ formatQuantity(row.weight_kg) }}
      </el-descriptions-item>
      <el-descriptions-item :label="t('greigeFabrics.index.colLengthM')">
        {{ formatQuantity(row.length_m) }}
      </el-descriptions-item>
    </el-descriptions>

    <el-form
      ref="formRef"
      :model="form"
      :rules="rules"
      label-width="120px"
      :aria-label="dialogTitle"
    >
      <el-form-item
        v-if="type === 'in'"
        :label="t('greigeFabrics.index.labelWarehouse')"
        prop="warehouse_id"
      >
        <el-select
          v-model="form.warehouse_id"
          filterable
          remote
          :loading="warehouseLoading"
          :remote-method="searchWarehouses"
          :remote-show-suffix="true"
          :placeholder="t('greigeFabrics.form.placeholderRemoteSearch')"
          :no-data-text="t('greigeFabrics.picker.noData')"
          style="width: 100%"
          @change="form.warehouse_id = typeof $event === 'number' ? $event : null"
        >
          <el-option
            v-for="item in warehouseOptions"
            :key="item.id"
            :label="item.label"
            :value="item.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item v-if="type === 'in'" :label="t('greigeFabrics.form.labelLocation')">
        <el-input v-model="form.location" clearable />
      </el-form-item>
      <el-form-item :label="t('greigeFabrics.index.labelWeightKg')" prop="weight_kg">
        <el-input v-model="form.weight_kg" :placeholder="unitKg" />
      </el-form-item>
      <el-form-item :label="t('greigeFabrics.index.labelLengthM')" prop="length_m">
        <el-input v-model="form.length_m" :placeholder="unitM" />
      </el-form-item>
      <el-form-item v-if="type === 'in'" :label="t('greigeFabrics.form.labelQualityGrade')">
        <el-select
          v-model="form.quality_grade"
          clearable
          :placeholder="t('greigeFabrics.form.placeholderSelectQualityGrade')"
          style="width: 100%"
        >
          <el-option v-for="g in QUALITY_GRADES" :key="g" :label="g" :value="g" />
        </el-select>
      </el-form-item>
      <el-form-item v-if="type === 'in'" :label="t('greigeFabrics.form.labelPurchaseReceipt')">
        <!-- 三单匹配的入库单候选按坯布所属供应商收窄；档案没有供应商时无法判定归属，
             选择器保持禁用并说明原因，不给勾不出的空下拉 -->
        <el-select
          v-model="form.purchase_receipt_id"
          filterable
          clearable
          :disabled="!row?.supplier_id"
          :loading="receiptLoading"
          :placeholder="receiptPlaceholder"
          :no-data-text="t('greigeFabrics.picker.noData')"
          style="width: 100%"
          @change="form.purchase_receipt_id = typeof $event === 'number' ? $event : null"
        >
          <el-option
            v-for="item in receiptOptions"
            :key="item.id"
            :label="item.label"
            :value="item.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('greigeFabrics.form.labelRemarks')">
        <el-input v-model="form.remarks" type="textarea" :rows="2" clearable />
      </el-form-item>
    </el-form>

    <template #footer>
      <el-button @click="emit('update:visible', false)">
        {{ t('greigeFabrics.index.buttonCancel') }}
      </el-button>
      <el-button type="primary" :loading="submitting" @click="submit">
        {{ t('greigeFabrics.index.buttonConfirm') }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';

import {
  stockInGreigeFabric,
  stockOutGreigeFabric,
  type GreigeFabric,
  type GreigeStockInPayload,
  type GreigeStockOutPayload,
} from '@/api/greige-fabric';
import { getWarehouseById, getWarehouseList } from '@/api/warehouse';
import { getPurchaseReceiptList } from '@/api/purchase-receipt';
import { decimalWireToNumber } from '@/utils/money';

import { useGreigePicker, PICKER_PAGE_SIZE, type PickerOption } from '../useGreigePicker';
import { requiredPositiveDecimal, toPayloadNumber } from '../greige-rules';

/**
 * 坯布入库与出库对话框。
 * 功能：按后端 StockInRequest 与 StockOutRequest 的完整入参采集，入库采仓库、库位、重量、
 * 长度、等级、备注与关联采购入库单七项，出库采重量、长度与备注三项并在提交前挡住全空提交。
 * 调用方：views/greige-fabrics/index.vue 的行内入库与出库入口。
 * 入参：visible 控制显隐，type 区分入库与出库，row 是被操作的坯布行（含当前库存出参）。
 * 传给谁：stockInGreigeFabric / stockOutGreigeFabric，即 POST /production/greige-fabrics/{id}/
 *      stock-in 与 stock-out；数量经校验后以 number 提交（后端 f64 入参转 DECIMAL(12,2)）。
 * 存什么：只存本对话框的录入值与两个选择器的候选。
 * 存哪里：组件内响应式状态，不落 store 与浏览器存储。
 */
const props = defineProps<{
  visible: boolean;
  type: 'in' | 'out';
  row: GreigeFabric | null;
}>();

const emit = defineEmits<{
  (e: 'update:visible', value: boolean): void;
  (e: 'done'): void;
}>();

const { t } = useI18n({ useScope: 'global' });

/** 与建档对话框同一取值域（建表列注释声明 A/B/C，见 system/m0002:56） */
const QUALITY_GRADES = ['A', 'B', 'C'] as const;
/** 入库单候选单页条数，取后端 clamp 上限以内（handlers/purchase_receipt_handler.rs:38,102） */
const RECEIPT_PAGE_SIZE = 100;

const formRef = ref<FormInstance>();
const submitting = ref(false);

const dialogTitle = computed(() =>
  props.type === 'in'
    ? t('greigeFabrics.index.stockInTitle')
    : t('greigeFabrics.index.stockOutTitle')
);
const unitKg = computed(() => t('greigeFabrics.form.placeholderUnit', { unit: 'kg' }));
const unitM = computed(() => t('greigeFabrics.form.placeholderUnit', { unit: 'm' }));
const receiptPlaceholder = computed(() =>
  props.row?.supplier_id
    ? t('greigeFabrics.form.placeholderSelectReceipt')
    : t('greigeFabrics.form.receiptNeedsSupplier')
);

const form = reactive({
  warehouse_id: null as number | null,
  location: '',
  weight_kg: '',
  length_m: '',
  quality_grade: '',
  remarks: '',
  purchase_receipt_id: null as number | null,
});

/** 库存出参是 Decimal 字符串，展示前 Number 归一，NULL 显 '-' 不兜 0 */
const formatQuantity = (value: string | null | undefined): string =>
  value === null || value === undefined ? '-' : String(decimalWireToNumber(value));

/**
 * 校验规则：入库三项全必填（对齐 StockInRequest 的必填键），出库两项均可选但由
 * submit 前置保证至少一项为正数（后端 StockOutRequest 两项都是 Option）。
 */
const rules = computed<FormRules>(() =>
  props.type === 'in'
    ? {
        warehouse_id: [
          {
            required: true,
            message: t('greigeFabrics.index.messageWarehouseRequired'),
            trigger: 'change',
          },
        ],
        weight_kg: [
          { validator: requiredPositiveDecimal('greigeFabrics.index.labelWeightKg'), trigger: 'blur' },
        ],
        length_m: [
          { validator: requiredPositiveDecimal('greigeFabrics.index.labelLengthM'), trigger: 'blur' },
        ],
      }
    : {
        weight_kg: [
          {
            validator: (rule, value, callback) => {
              if (String(form.length_m ?? '').trim() === '' && toPayloadNumber(value) === undefined) {
                callback(
                  new Error(t('greigeFabrics.index.messageAtLeastOneOfWeightOrLength'))
                );
                return;
              }
              callback();
            },
            trigger: 'blur',
          },
        ],
        length_m: [
          {
            validator: (rule, value, callback) => {
              if (String(form.weight_kg ?? '').trim() === '' && toPayloadNumber(value) === undefined) {
                callback(
                  new Error(t('greigeFabrics.index.messageAtLeastOneOfWeightOrLength'))
                );
                return;
              }
              callback();
            },
            trigger: 'blur',
          },
        ],
      }
);

const warehousePicker = useGreigePicker(
  async (keyword): Promise<PickerOption[]> => {
    const res = await getWarehouseList({
      search: keyword,
      page: 1,
      page_size: PICKER_PAGE_SIZE,
    });
    return res.data.items.map(item => ({
      id: item.id,
      label: `${item.warehouse_name}（${item.warehouse_code}）`,
    }));
  },
  async (id): Promise<PickerOption | null> => {
    const res = await getWarehouseById(id);
    const row = res.data;
    if (!row) return null;
    return { id, label: `${row.warehouse_name}（${row.warehouse_code}）` };
  }
);
const {
  options: warehouseOptions,
  loading: warehouseLoading,
  search: searchWarehouses,
} = warehousePicker;

/** 入库单候选：按坯布档案上的供应商做服务端收窄，控件本地过滤只在这批结果内继续缩 */
const receiptPicker = useGreigePicker(
  async (): Promise<PickerOption[]> => {
    const supplierId = props.row?.supplier_id;
    if (!supplierId) return [];
    const res = await getPurchaseReceiptList({
      supplier_id: supplierId,
      page: 1,
      page_size: RECEIPT_PAGE_SIZE,
    });
    return (res.data?.items ?? [])
      .filter(item => typeof item.id === 'number')
      .map(item => ({ id: item.id as number, label: item.receipt_no }));
  },
  async (): Promise<PickerOption | null> => null
);
const { options: receiptOptions, loading: receiptLoading, search: searchReceipts } = receiptPicker;

watch(
  () => props.visible,
  async visible => {
    if (!visible) return;
    Object.assign(form, {
      warehouse_id: null,
      location: '',
      weight_kg: '',
      length_m: '',
      quality_grade: '',
      remarks: '',
      purchase_receipt_id: null,
    });
    formRef.value?.clearValidate();
    // 入库默认落回坯布档案上的现仓库，用户可改；出库不动仓库
    if (props.type === 'in') {
      await warehousePicker.restore(props.row?.warehouse_id ?? null);
      form.warehouse_id = props.row?.warehouse_id ?? null;
      await searchReceipts('');
    }
  }
);

const submit = async (): Promise<void> => {
  if (!props.row || !formRef.value) return;
  const valid = await formRef.value.validate().catch(() => false);
  if (!valid) return;

  const weight = toPayloadNumber(form.weight_kg);
  const length = toPayloadNumber(form.length_m);
  if (props.type === 'out' && !(typeof weight === 'number') && !(typeof length === 'number')) {
    ElMessage.error(t('greigeFabrics.index.messageAtLeastOneOfWeightOrLength'));
    return;
  }

  submitting.value = true;
  try {
    if (props.type === 'in') {
      const payload: GreigeStockInPayload = {
        warehouse_id: form.warehouse_id as number,
        weight_kg: weight as number,
        length_m: length as number,
        location: form.location.trim() === '' ? undefined : form.location.trim(),
        quality_grade: form.quality_grade || undefined,
        remarks: form.remarks.trim() === '' ? undefined : form.remarks.trim(),
        purchase_receipt_id: form.purchase_receipt_id ?? undefined,
      };
      await stockInGreigeFabric(props.row.id, payload);
      ElMessage.success(t('greigeFabrics.index.messageStockInSuccess'));
    } else {
      const payload: GreigeStockOutPayload = {
        remarks: form.remarks.trim() === '' ? undefined : form.remarks.trim(),
      };
      if (typeof weight === 'number') payload.weight_kg = weight;
      if (typeof length === 'number') payload.length_m = length;
      await stockOutGreigeFabric(props.row.id, payload);
      ElMessage.success(t('greigeFabrics.index.messageStockOutSuccess'));
    }
    emit('update:visible', false);
    emit('done');
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('greigeFabrics.index.messageStockFailed'));
  } finally {
    submitting.value = false;
  }
};
</script>

<style scoped>
.current-stock {
  margin-bottom: 16px;
}
</style>
