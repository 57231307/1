<!--
  PurchaseReceiptForm.vue - 采购入库新增/编辑对话框
  拆分自 purchaseReceipt/index.vue（P14 批 2 I-3 第 4 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  价税分离口径：含税单价为主输入（录入事实），净额单价、税额预览与金额小计为派生只读列，
  反算算式全部走 usePrcProc 与 money.ts 的唯一实现，本组件只接线不复算
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="title"
    width="1280px"
    :aria-label="title"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form
      ref="formRef"
      :model="localForm"
      :rules="rules"
      label-width="100px"
      :aria-label="t('purchaseReceipt.form.aria.form')"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('purchaseReceipt.form.label.receiptNo')" prop="receipt_no">
            <el-input v-model="localForm.receipt_no" readonly />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('purchaseReceipt.form.label.receiptDate')" prop="receipt_date">
            <el-date-picker v-model="localForm.receipt_date" type="date" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('purchaseReceipt.form.label.supplier')" prop="supplier_id">
            <el-select
              :model-value="localForm.supplier_id"
              :placeholder="t('purchaseReceipt.form.placeholder.supplier')"
              @update:model-value="(v: number | undefined) => (localForm.supplier_id = v)"
            >
              <el-option v-for="s in suppliers" :key="s.value" :label="s.label" :value="s.value" />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('purchaseReceipt.form.label.warehouse')" prop="warehouse_id">
            <el-select
              :model-value="localForm.warehouse_id"
              :placeholder="t('purchaseReceipt.form.placeholder.warehouse')"
              @update:model-value="(v: number | undefined) => (localForm.warehouse_id = v)"
            >
              <el-option v-for="w in warehouses" :key="w.value" :label="w.label" :value="w.value" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('purchaseReceipt.form.label.items')">
        <div class="items-table">
          <div class="items-header">
            <span class="col-product">{{ t('purchaseReceipt.form.itemsHeader.product') }}</span>
            <span class="col-qty">{{ t('purchaseReceipt.form.itemsHeader.quantity') }}</span>
            <span class="col-qty">
              {{ t('purchaseReceipt.form.itemsHeader.quantityAlt') }}
            </span>
            <span class="col-unit">{{ t('purchaseReceipt.form.itemsHeader.unit') }}</span>
            <span class="col-price">
              {{ t('purchaseReceipt.form.itemsHeader.priceWithTax') }}
            </span>
            <span class="col-tax">{{ t('purchaseReceipt.form.itemsHeader.taxRate') }}</span>
            <span class="col-net-price">{{ t('purchaseReceipt.form.itemsHeader.netPrice') }}</span>
            <span class="col-tax-amount">
              {{ t('purchaseReceipt.form.itemsHeader.taxAmount') }}
            </span>
            <span class="col-amount">{{ t('purchaseReceipt.form.itemsHeader.amount') }}</span>
            <span class="col-action">{{ t('purchaseReceipt.form.itemsHeader.action') }}</span>
          </div>
          <div v-for="(item, index) in localForm.items || []" :key="index" class="items-block">
            <div class="items-row">
              <el-select
                :model-value="item.product_id"
                :placeholder="t('purchaseReceipt.form.placeholder.product')"
                class="col-product"
                @update:model-value="(v: number) => onProductChange(item, v)"
              >
                <el-option v-for="p in products" :key="p.value" :label="p.label" :value="p.value" />
              </el-select>
              <el-input-number
                :model-value="item.quantity"
                class="col-qty"
                @update:model-value="
                  (v: number | undefined) => {
                    item.quantity = v ?? 0;
                    emit('calc-amount', item);
                  }
                "
              />
              <!--
                辅量（quantity_alt）：面料行业按米点/公斤双计量收货的实测辅量。
                DB 可空列（models/purchase_receipt_item 的 quantity_alt: Option<Decimal>）——留空=未采集
                （undefined，不得 ?? 0 伪装成实测量）；更新契约行留空按三态处理（usePrcProc）。
              -->
              <el-input-number
                :model-value="item.quantity_alt"
                class="col-qty"
                :placeholder="t('purchaseReceipt.form.placeholder.quantityAlt')"
                @update:model-value="
                  (v: number | undefined) => {
                    item.quantity_alt = v ?? undefined;
                  }
                "
              />
              <!-- 主单位取自产品档案，不可手工录入（后端 unit_master 为必填契约字段） -->
              <span class="col-unit">{{ item.unit_master || '-' }}</span>
              <!--
                含税单价是本表价格的主输入（录入事实），el-input 承载输入框原文：
                带加减号的步进控件是全项目禁项；空串=未采集，'0' 是人工显式声明的 0 价。
                后端入库明细没有含税单价列，本值不上送，只经 usePrcProc 除法反算净额与税额。
                任何编辑（含清空）都置 price_edited，让"主动清空"与"没碰"在提交腿可分辨。
              -->
              <el-input
                :model-value="item.unit_price_with_tax"
                class="col-price"
                inputmode="decimal"
                :placeholder="t('purchaseReceipt.form.placeholder.unitPriceWithTax')"
                @update:model-value="
                  (v: string) => {
                    item.unit_price_with_tax = v;
                    item.price_edited = true;
                    emit('calc-amount', item);
                  }
                "
              />
              <!-- 税率与含税单价成对录入（百分数，13 表示 13%），后端无税率列、不上送，只进除法反算；同样置编辑标志 -->
              <NumericInput
                v-model="item.tax_rate"
                class="col-tax"
                optional
                :min="0"
                :max="999.9999"
                :precision="4"
                @change="
                  () => {
                    item.price_edited = true;
                    emit('calc-amount', item);
                  }
                "
              />
              <!-- 以下三列均为派生只读：净额由除法反算、金额为数量乘净额（与后端落库同式）、税额为含税减净额取差额 -->
              <span class="col-net-price">{{ formatDerived(item.unit_price) }}</span>
              <span class="col-tax-amount">{{ formatDerived(item.tax_amount) }}</span>
              <span class="col-amount">{{ formatDerived(item.amount) }}</span>
              <el-button
                v-if="(localForm.items || []).length > 1"
                size="small"
                type="danger"
                @click="emit('remove-item', index)"
                >{{ t('purchaseReceipt.form.button.delete') }}</el-button
              >
            </div>
            <!--
              收货实测维度行：批次号（后端建单期强校验的四维之一，逐行必填）+ 染色布追溯维度
              （色号/缸号/等级，透传至 CreateReceiptItemRequest 的 batch_no/color_code/lot_no/grade）。
              无自动来源（本页不关联采购订单行、产品档案也不含这些维度），由操作人录入，禁止造假值。
            -->
            <div class="items-trace">
              <label class="trace-field">
                <span class="trace-label required">
                  {{ t('purchaseReceipt.form.itemsHeader.batch') }}
                </span>
                <el-input
                  v-model="item.batch_no"
                  size="small"
                  :placeholder="t('purchaseReceipt.form.placeholder.batch')"
                />
              </label>
              <label class="trace-field">
                <span class="trace-label">
                  {{ t('purchaseReceipt.form.itemsHeader.colorNo') }}
                </span>
                <el-input
                  v-model="item.color_code"
                  size="small"
                  :placeholder="t('purchaseReceipt.form.placeholder.colorNo')"
                />
              </label>
              <label class="trace-field">
                <span class="trace-label">
                  {{ t('purchaseReceipt.form.itemsHeader.lotNo') }}
                </span>
                <el-input
                  v-model="item.lot_no"
                  size="small"
                  :placeholder="t('purchaseReceipt.form.placeholder.lotNo')"
                />
              </label>
              <label class="trace-field">
                <span class="trace-label">
                  {{ t('purchaseReceipt.form.itemsHeader.grade') }}
                </span>
                <el-input
                  v-model="item.grade"
                  size="small"
                  :placeholder="t('purchaseReceipt.form.placeholder.grade')"
                />
              </label>
            </div>
          </div>
          <el-button type="text" @click="emit('add-item')">{{
            t('purchaseReceipt.form.button.addItem')
          }}</el-button>
        </div>
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">{{
        t('purchaseReceipt.form.button.cancel')
      }}</el-button>
      <el-button type="primary" :loading="submitting" :disabled="submitting" @click="onSubmit">{{
        t('purchaseReceipt.form.button.submit')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { deepClone } from '@/utils';
import { ref, watch, nextTick } from 'vue';
import { useI18n } from 'vue-i18n';
import type { FormInstance, FormRules } from 'element-plus';
import type { ReceiptItem } from '@/api/purchase-receipt';
import NumericInput from '@/components/NumericInput.vue';
import { formatDecimalAmount } from '@/utils/money';

const { t } = useI18n({ useScope: 'global' });

/**
 * 派生只读列的统一展示（净额单价、税额预览、金额小计三列共用）：
 * 无值（价格未采集或未录税率）显示占位符而不是 0，避免把"没算"看成"算得 0"；
 * 有值走 money.ts 的 formatDecimalAmount 统一千分位与两位小数。
 * 入参是 usePrcProc 的 calculateItemAmount 除法反算产物，只读展示给收货人核对，
 * 本组件不对它们做任何乘除税率运算，也不作为权威值上送（金额由后端按数量乘净额重算）。
 */
const formatDerived = (value: number | undefined): string =>
  value === undefined ? '--' : formatDecimalAmount(value);

// 选项类型（产品选项额外带产品档案主数据：编码/名称/主单位）
interface OptItem {
  label: string;
  value: number;
  code?: string;
  name?: string;
  unit?: string;
}

// 表单类型
// 编辑态行类型：契约行 ReceiptItem 之上加价格录入腿编辑标志 price_edited。
// 本标志是纯 UI 态（不上送、不回显、也不进 api 契约类型——api-keys 门禁口径要求
// 契约类型里的每个键都在后端有同名字段）：含税单价或税率录入框被操作人编辑过即置真，
// usePrcProc 的 calculateItemAmount 回退分支与 handleSubmit 清空点名消费它，
// 用以分辨"主动清空价格"与"没碰价格"。
type PrcFormRow = ReceiptItem & { price_edited?: boolean };

interface PurchaseReceiptFormModel {
  id?: number;
  receipt_no?: string;
  receipt_date?: string;
  supplier_id?: number;
  warehouse_id?: number;
  status?: string;
  items?: PrcFormRow[];
  [key: string]: unknown;
}

const props = defineProps<{
  // 对话框可见性
  visible: boolean;
  // 对话框标题
  title: string;
  // 表单数据（由父组件管理，子组件通过 emit('update:form') 回写）
  form: PurchaseReceiptFormModel;
  // 校验规则
  rules: FormRules;
  // 供应商选项
  suppliers: OptItem[];
  // 仓库选项
  warehouses: OptItem[];
  // 产品选项
  products: OptItem[];
  // 提交在途（父组件管理）：为真时提交按钮转圈并禁用，避免重复建单
  submitting: boolean;
}>();

const emit = defineEmits<{
  (e: 'update:visible', v: boolean): void;
  (e: 'add-item'): void;
  (e: 'remove-item', index: number): void;
  (e: 'calc-amount', item: ReceiptItem): void;
  (e: 'submit'): void;
  // 整体回写表单（父组件监听此事件并回写到自己的 form）
  (e: 'update:form', form: PurchaseReceiptFormModel): void;
}>();

// 表单 ref
const formRef = ref<FormInstance>();

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
// 注意：表单内有 items 数组，需要深拷贝以保证本地修改与父组件解耦
const localForm = ref<PurchaseReceiptFormModel>(deepClone(props.form));

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false;

// 外部 prop 变化时同步到 local（如父组件打开新增/编辑时填充数据）
watch(
  () => props.form,
  newForm => {
    if (syncing) return;
    syncing = true;
    localForm.value = deepClone(newForm);
    nextTick(() => {
      syncing = false;
    });
  },
  { deep: true }
);

// 本地变化时通知父组件（用户输入）
watch(
  localForm,
  newForm => {
    if (syncing) return;
    syncing = true;
    emit('update:form', deepClone(newForm));
    nextTick(() => {
      syncing = false;
    });
  },
  { deep: true }
);

/**
 * 选择产品：把产品档案的编码/名称/主单位带入明细行
 * （后端 CreateReceiptItemRequest 的 material_code/material_name/unit_master 为必填，
 * 只能来源于产品主数据，不允许提交时再拼假值）
 */
const onProductChange = (item: ReceiptItem, productId: number) => {
  item.product_id = productId;
  const opt = props.products.find(p => p.value === productId);
  if (!opt) return;
  item.material_code = opt.code;
  item.material_name = opt.name;
  item.unit_master = opt.unit;
};

/** 点击确定：先校验再发 submit */
const onSubmit = async () => {
  if (props.submitting) return;
  if (!formRef.value) return;
  await formRef.value.validate(async (valid: boolean) => {
    if (!valid) return;
    emit('submit');
  });
};
</script>

<style scoped>
.items-table {
  border: 1px solid #ebeef5;
  border-radius: 4px;
}
.items-header {
  display: flex;
  background: #f5f7fa;
  padding: 10px;
  font-weight: bold;
}
.items-row {
  display: flex;
  padding: 10px 10px 4px;
}
.items-block {
  border-top: 1px solid #ebeef5;
}
.items-trace {
  display: flex;
  flex-wrap: wrap;
  gap: 12px 16px;
  padding: 4px 10px 10px;
}
.trace-field {
  display: flex;
  align-items: center;
  gap: 6px;
}
.trace-label {
  font-size: 12px;
  color: #606266;
  white-space: nowrap;
}
.trace-label.required::before {
  content: '*';
  color: #f56c6c;
  margin-right: 2px;
}
.trace-field :deep(.el-input) {
  width: 150px;
}
.col-product {
  flex: 2;
  min-width: 160px;
  margin-right: 10px;
}
.col-qty,
.col-price,
.col-net-price,
.col-amount {
  width: 100px;
  margin-right: 10px;
}
.col-tax,
.col-tax-amount {
  width: 84px;
  margin-right: 10px;
}
.col-net-price,
.col-tax-amount,
.col-amount {
  line-height: 32px;
  white-space: nowrap;
}
.col-unit {
  width: 56px;
  margin-right: 10px;
  line-height: 32px;
}
.col-action {
  width: 60px;
}

@media (max-width: 768px) {
  /* 明细行与追溯维度在竖屏逐项堆叠，固定列宽会撑破弹窗内容区 */
  .items-header {
    display: none;
  }

  .items-row {
    flex-wrap: wrap;
    gap: 8px;
    padding: 10px 0 4px;
  }

  .items-row > * {
    margin-right: 0;
  }

  .col-product,
  .col-qty,
  .col-price,
  .col-tax,
  .col-net-price,
  .col-tax-amount,
  .col-amount,
  .col-unit {
    flex: 0 0 100%;
    width: 100%;
    min-width: 0;
  }

  .col-unit,
  .col-net-price,
  .col-tax-amount,
  .col-amount {
    line-height: 1.5;
  }

  .items-row > .el-button {
    width: 100%;
    margin-left: 0;
  }

  .items-trace {
    gap: 8px;
    padding: 4px 0 10px;
  }

  .trace-field {
    width: 100%;
  }

  .trace-field :deep(.el-input) {
    flex: 1;
    width: 100%;
  }
}
</style>
