<!--
  OrderFormDialog.vue - 销售订单表单对话框
  来源：原 sales/index.vue 中 订单表单 dialog
  拆分日期：2026-06-15 B3-1
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="title"
    width="900px"
    destroy-on-close
    :aria-label="title"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form
      ref="formRef"
      :model="localData"
      :rules="formRules"
      label-width="100px"
      :aria-label="t('sales.orderForm.formAriaLabel')"
    >
      <el-divider content-position="left">{{ t('sales.orderForm.basicInfo') }}</el-divider>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('sales.orderForm.customer')" prop="customer_id">
            <el-select
              v-model="localData.customer_id"
              :placeholder="t('sales.orderForm.customerPlaceholder')"
              style="width: 100%"
              @change="handleCustomerChange"
            >
              <el-option
                v-for="c in customers"
                :key="c.id"
                :label="c.customer_name"
                :value="c.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('sales.orderForm.orderDate')" prop="order_date">
            <el-date-picker
              v-model="localData.order_date"
              type="date"
              value-format="YYYY-MM-DD"
              :placeholder="t('sales.orderForm.datePlaceholder')"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('sales.orderForm.requiredDate')" prop="required_date">
            <el-date-picker
              v-model="localData.required_date"
              type="date"
              value-format="YYYY-MM-DD"
              :placeholder="t('sales.orderForm.datePlaceholder')"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('sales.orderForm.contactPerson')" prop="contact_person">
            <el-input
              v-model="localData.contact_person"
              :placeholder="t('sales.orderForm.contactPersonPlaceholder')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('sales.orderForm.contactPhone')" prop="contact_phone">
        <el-input
          v-model="localData.contact_phone"
          :placeholder="t('sales.orderForm.contactPhonePlaceholder')"
          style="width: 50%"
        />
      </el-form-item>
      <el-form-item :label="t('sales.orderForm.deliveryAddress')" prop="shipping_address">
        <el-input
          v-model="localData.shipping_address"
          type="textarea"
          :rows="2"
          :placeholder="t('sales.orderForm.deliveryAddressPlaceholder')"
        />
      </el-form-item>

      <el-divider content-position="left">{{ t('sales.orderForm.orderItems') }}</el-divider>
      <div class="order-items">
        <el-table
          :data="localData.items"
          border
          style="width: 100%"
          :aria-label="t('sales.orderForm.itemsTableAriaLabel')"
        >
          <el-table-column :label="t('sales.orderForm.product')" width="200">
            <template #default="{ row, $index }">
              <el-select
                v-model="row.product_id"
                :placeholder="t('sales.orderForm.productPlaceholder')"
                @change="(v: number) => handleProductSelect($index, v)"
              >
                <el-option
                  v-for="p in products"
                  :key="p.id"
                  :label="p.product_name"
                  :value="p.id"
                />
              </el-select>
            </template>
          </el-table-column>
          <el-table-column :label="t('sales.orderForm.colorNo')" width="160">
            <template #default="{ row }">
              <el-select
                v-model="row.color_no"
                clearable
                filterable
                :disabled="!row.product_id"
                :placeholder="t('sales.orderForm.colorNoPlaceholder')"
                style="width: 100%"
              >
                <el-option
                  v-for="c in colorOptionsByProduct[row.product_id] || []"
                  :key="c.id"
                  :label="c.color_name ? `${c.color_no} · ${c.color_name}` : c.color_no"
                  :value="c.color_no"
                />
              </el-select>
            </template>
          </el-table-column>
          <el-table-column prop="quantity" :label="t('sales.orderForm.quantity')" width="120">
            <template #default="{ row }">
              <el-input-number
                v-model="row.quantity"
                :min="1"
                size="small"
                @change="() => calculateSubtotal(row)"
              />
            </template>
          </el-table-column>
          <el-table-column prop="unit" :label="t('sales.orderForm.unit')" width="80" />
          <el-table-column
            prop="unit_price_with_tax"
            :label="t('sales.orderForm.unitPriceWithTax')"
            width="130"
          >
            <template #default="{ row }">
              <!--
                含税单价是本表唯一的价格主输入（录入事实），用 el-input 承载、靠校验器管范围：
                带加减号的步进控件是全项目禁项。不含税单价与税额由它按税率除法反算，只读不可手输。
              -->
              <el-input
                v-model="row.unit_price_with_tax"
                size="small"
                :placeholder="t('sales.orderForm.unitPriceWithTaxPlaceholder')"
                @change="() => calculateSubtotal(row)"
              />
            </template>
          </el-table-column>
          <el-table-column prop="tax_percent" :label="t('sales.orderForm.taxPercent')" width="110">
            <template #default="{ row }">
              <el-input
                v-model.number="row.tax_percent"
                size="small"
                :placeholder="t('sales.orderForm.taxPercentPlaceholder')"
                @change="() => calculateSubtotal(row)"
              />
            </template>
          </el-table-column>
          <!-- 不含税单价是派生只读列：含税单价按本行税率除法反算，前端绝不乘 (1 + 税率) 造含税 -->
          <el-table-column prop="unit_price" :label="t('sales.orderForm.unitPrice')" width="120">
            <template #default="{ row }">
              <span class="amount">{{ formatDerived(row.unit_price) }}</span>
            </template>
          </el-table-column>
          <el-table-column prop="subtotal" :label="t('sales.orderForm.subtotal')" width="120">
            <template #default="{ row }">
              <span class="amount">{{ formatDerived(row.subtotal) }}</span>
            </template>
          </el-table-column>
          <!-- 税额是派生只读列：含税成交额减不含税小计取差额，与后端 utils tax 同一口径 -->
          <el-table-column :label="t('sales.orderForm.taxAmount')" width="110">
            <template #default="{ row }">
              <span class="amount">{{ formatDerived(row.tax_amount) }}</span>
            </template>
          </el-table-column>
          <el-table-column :label="t('sales.orderForm.tolerance')" width="140">
            <template #default="{ row }">
              <el-input-number
                v-model="row.quantity_tolerance_pct"
                :min="0"
                :max="100"
                :precision="2"
                size="small"
                :placeholder="t('sales.orderForm.tolerancePlaceholder')"
                style="width: 100%"
              />
            </template>
          </el-table-column>
          <el-table-column :label="t('sales.orderForm.operation')" width="80">
            <template #default="{ $index }">
              <el-button type="danger" link size="small" @click="removeItem($index)">{{
                t('sales.orderForm.delete')
              }}</el-button>
            </template>
          </el-table-column>
        </el-table>
        <el-button type="primary" plain size="small" style="margin-top: 10px" @click="addItem">
          <el-icon><Plus /></el-icon> {{ t('sales.orderForm.addItem') }}
        </el-button>
      </div>

      <el-divider content-position="left">{{ t('sales.orderForm.otherInfo') }}</el-divider>
      <el-form-item :label="t('sales.orderForm.remark')">
        <el-input
          v-model="localData.notes"
          type="textarea"
          :rows="3"
          :placeholder="t('sales.orderForm.remarkPlaceholder')"
        />
      </el-form-item>
      <!--
        本表单的价税方向与后端 utils/tax.rs 与前端 money.ts 单源一致：含税单价是录入事实（主输入），
        不含税单价、不含税小计与税额都是逐行派生只读列（除法反算 + 取差额），
        表单内不存在任何「× 税率」造税额或「× (1 + 税率)」造含税的算式。
        上面的合计是**不含税净额合计**；成交含税额与税额的权威值仍由后端按真实 tax_rate
        反算回传，展示见 OrderViewDialog 的详情。
      -->
      <el-row :gutter="20">
        <el-col :span="8">
          <el-form-item :label="t('sales.orderForm.orderTotal')">
            <div class="total-amount">¥{{ formatDerived(calculateTotal()) }}</div>
          </el-form-item>
        </el-col>
      </el-row>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">{{
        t('sales.orderForm.cancel')
      }}</el-button>
      <el-button type="primary" :loading="submitting" @click="handleSubmit">{{
        t('sales.orderForm.confirm')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { deepClone, toLocalDateString } from '@/utils';
import { ref, reactive, watch, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import type { FormInstance, FormRules } from 'element-plus';
import type { Customer } from '@/api/customer';
import { getProductColorList, type Product } from '@/api/product';
import type { ProductColor } from '@/api/product';
import { logger } from '@/utils/logger';
import {
  formatDecimalAmount,
  inclusiveAmountFromLine,
  netFromInclusive,
  parseDecimalInput,
  taxFromInclusive,
} from '@/utils/money';

const { t } = useI18n({ useScope: 'global' });

interface OrderItemForm {
  id: number;
  product_id: number | undefined;
  product_name: string;
  product_code: string;
  /** 色号：空串=白坯布（合法），非空=染色布；下拉选项来自该行产品的色号列表 */
  color_no: string;
  quantity: number;
  unit: string;
  /**
   * 含税单价（录入事实，本表唯一价格主输入）：输入框原文，空串=未填，'0'=显式 0 价行。
   * 税率非 0 时后端要求本值（仅可还原的历史行能免提交），系统不按税率代造含税价。
   */
  unit_price_with_tax: string;
  /** 税率（百分数口径，13 表示 13%）：undefined 或非法文本按未填处理，后端视为 0 = 不含税/免税 */
  tax_percent: number | undefined;
  /** 不含税单价（派生只读）：含税单价按税率除法反算 */
  unit_price: number;
  /** 不含税小计（派生只读）：含税成交额按税率除法反算 */
  subtotal: number;
  /** 税额（派生只读）：含税成交额 − 不含税小计 取差额 */
  tax_amount: number | undefined;
  /** 交货容差百分比（undefined=未填，提交时转为 null） */
  quantity_tolerance_pct: number | undefined;
}

interface OrderForm {
  id?: number;
  customer_id: number | undefined;
  customer_name: string;
  order_date: string;
  required_date: string;
  contact_person: string;
  contact_phone: string;
  shipping_address: string;
  notes: string;
  items: OrderItemForm[];
  total_amount?: number;
}

const props = defineProps<{
  visible: boolean;
  title: string;
  formData: OrderForm;
  customers: Customer[];
  products: Product[];
  submitting?: boolean;
}>();

const emit = defineEmits<{
  'update:visible': [value: boolean];
  submit: [data: OrderForm];
}>();

const formRef = ref<FormInstance>();

// 本地数据副本：避免 vue/no-mutating-props 警告；
// watch 监听 props.formData 变化以同步父组件重置
const localData = reactive<OrderForm>({
  id: 0,
  customer_id: undefined,
  customer_name: '',
  order_date: toLocalDateString(),
  required_date: '',
  contact_person: '',
  contact_phone: '',
  shipping_address: '',
  notes: '',
  items: [],
  total_amount: 0,
});

watch(
  () => props.formData,
  newData => {
    Object.assign(localData, deepClone(newData));
  },
  { deep: true, immediate: true }
);

// 编辑回显：对话框打开时按各行已选产品预加载色号选项，使已存 color_no 命中下拉项正常显示
watch(
  () => props.visible,
  visible => {
    if (!visible) return;
    props.formData.items.forEach(item => ensureColorOptions(item.product_id));
  }
);

const formRules = computed<FormRules>(() => ({
  customer_id: [
    { required: true, message: t('sales.orderForm.customerRequired'), trigger: 'change' },
  ],
  order_date: [
    { required: true, message: t('sales.orderForm.orderDateRequired'), trigger: 'change' },
  ],
  required_date: [
    { required: true, message: t('sales.orderForm.requiredDateRequired'), trigger: 'change' },
  ],
  contact_person: [
    { required: true, message: t('sales.orderForm.contactPersonRequired'), trigger: 'blur' },
  ],
  contact_phone: [
    { required: true, message: t('sales.orderForm.contactPhoneRequired'), trigger: 'blur' },
    {
      pattern: /^1[3-9]\d{9}$/,
      message: t('sales.orderForm.contactPhoneInvalid'),
      trigger: 'blur',
    },
  ],
  shipping_address: [
    { required: true, message: t('sales.orderForm.deliveryAddressRequired'), trigger: 'blur' },
  ],
}));

const handleCustomerChange = (customerId: number) => {
  const customer = props.customers.find(c => c.id === customerId);
  if (customer) {
    localData.customer_name = customer.customer_name;
  }
};

/** 色号下拉选项缓存：key=product_id，来源为后端该产品色号列表 GET /products/{id}/colors */
const colorOptionsByProduct = reactive<Record<number, ProductColor[]>>({});

/** 按产品加载色号选项，已加载则复用；失败经 logger 暴露后不再重复请求 */
const ensureColorOptions = async (productId: number | undefined) => {
  if (!productId || colorOptionsByProduct[productId]) return;
  try {
    const res = await getProductColorList(productId);
    colorOptionsByProduct[productId] = res.data ?? [];
  } catch (error) {
    logger.error(t('sales.orderForm.colorLoadFailed'), error);
  }
};

const handleProductSelect = async (index: number, productId: number) => {
  const row = localData.items[index];
  // 换产品清空按旧产品选定的色号，避免残留不属于新产品的色号
  row.color_no = '';
  const product = props.products.find(p => p.id === productId);
  if (product) {
    row.product_name = product.product_name;
    row.product_code = product.product_code;
    // 产品参考价带进**含税单价**主输入列（可现谈覆盖）；无参考价就留空，
    // 不补 0、不乘税率：带出只是建议值，含税口径由制单人确认
    row.unit_price_with_tax = product.price == null ? '' : String(product.price);
    calculateSubtotal(row);
  }
  await ensureColorOptions(row.product_id);
};

/**
 * 派生列重算（方向口径与后端 utils/tax.rs 与前端 money.ts 单源完全一致）：
 * - 含税成交额 = 含税单价 × 数量（含税事实的聚合，税率不进此乘法）
 * - 不含税单价与不含税小计 = 含税值 ÷ (1 + 税率% ÷ 100) 四舍五入到分（除法反算）
 * - 税额 = 含税成交额 − 不含税小计 取差额（不做第二次独立舍入，恒等式逐分成立）
 * 入参 item 是本表一行；出参写回 item.unit_price、item.subtotal、item.tax_amount 三个只读列。
 * 未填含税单价或税率非法（parseDecimalInput 出 NaN/undefined）时派生列回到无值：
 * 不猜税率档位、不补 0 冒充算得，缺值由 handleSubmit 的行有效性判定拦下。
 */
const calculateSubtotal = (item: OrderItemForm) => {
  const inclusiveUnit = parseDecimalInput(item.unit_price_with_tax);
  const taxPercent = item.tax_percent;
  const usableRate =
    taxPercent === undefined || !Number.isFinite(taxPercent) || taxPercent < 0
      ? undefined
      : taxPercent;
  if (
    inclusiveUnit === undefined ||
    !Number.isFinite(inclusiveUnit) ||
    inclusiveUnit < 0 ||
    usableRate === undefined
  ) {
    item.unit_price = 0;
    item.subtotal = 0;
    item.tax_amount = undefined;
    return;
  }
  const lineInclusive = inclusiveAmountFromLine(inclusiveUnit, item.quantity);
  item.unit_price = netFromInclusive(inclusiveUnit, usableRate);
  item.subtotal = netFromInclusive(lineInclusive, usableRate);
  item.tax_amount = taxFromInclusive(lineInclusive, usableRate);
};

/** 派生只读列展示：无值显示占位符而不是 0，避免把"没算"看成"算得 0" */
const formatDerived = (value: number | undefined): string =>
  value === undefined ? '--' : formatDecimalAmount(value);

/**
 * 合计（不含税净额口径）：只累加已算得的派生小计，缺值行按 0 计入合计，
 * 与后端表头汇总一致；含税合计与税额合计不在这里乘税率造出来。
 */
const calculateTotal = () => {
  return localData.items.reduce((sum, item) => sum + (item.subtotal || 0), 0);
};

const addItem = () => {
  localData.items.push({
    id: Date.now(),
    product_id: undefined,
    product_name: '',
    product_code: '',
    color_no: '',
    quantity: 1,
    unit: t('sales.orderForm.defaultUnit'),
    // 含税单价留空由人工填写（0 只能由人工显式敲出），税率不预填任何档位
    unit_price_with_tax: '',
    tax_percent: undefined,
    // 三个派生列在无录入值时为无值状态，不预置 0 冒充算得
    unit_price: 0,
    subtotal: 0,
    tax_amount: undefined,
    quantity_tolerance_pct: undefined,
  });
};

const removeItem = (index: number) => {
  if (localData.items.length > 1) {
    localData.items.splice(index, 1);
  } else {
    ElMessage.warning(t('sales.orderForm.atLeastOneItem'));
  }
};

const handleSubmit = async () => {
  if (!formRef.value) return;
  try {
    await formRef.value.validate();
    // 有效行判据：已选产品且有数量。含税单价与税率的缺失/非法不再"静默丢行"，
    // 而是点名拦下——丢行会让制单人以为下单成功而少行，属金额台账的实缺陷形态。
    const touched = localData.items.filter(item => item.product_id && item.quantity > 0);
    const incomplete = touched.find(
      item => parseDecimalInput(item.unit_price_with_tax) === undefined
    );
    if (incomplete) {
      ElMessage.warning(t('sales.orderForm.unitPriceRequired'));
      return;
    }
    const invalidPrice = touched.find(item => {
      const parsed = parseDecimalInput(item.unit_price_with_tax);
      return parsed !== undefined && (!Number.isFinite(parsed) || parsed < 0);
    });
    if (invalidPrice) {
      ElMessage.warning(t('sales.orderForm.unitPriceInvalid'));
      return;
    }
    const validItems = touched.filter(item => (item.subtotal || 0) > 0);
    if (validItems.length === 0) {
      ElMessage.warning(t('sales.orderForm.addValidItem'));
      return;
    }
    // 空白明细行（未选产品）的 product_id 为 undefined，序列化后该键缺失，而后端
    // SalesOrderItemRequest.product_id 是非 Option 的 i32（backend/src/services/so/mod.rs:170），
    // 缺键会导致反序列化失败并报参数错误。判空依据与提交内容必须同为 validItems。
    // 判空已用 validItems，提交内容必须同为过滤后的有效行（与判空依据一致），杜绝空行入参。
    localData.total_amount = calculateTotal();
    emit('submit', { ...localData, items: validItems });
  } catch (error) {
    const err = error as { message?: string };
    if (err.message) {
      ElMessage.error(err.message || t('sales.orderForm.operationFailed'));
    }
  }
};
</script>

<style scoped>
.order-items {
  margin-bottom: 20px;
}
.amount {
  font-weight: 600;
  color: #f56c6c;
}
.total-amount {
  font-size: 20px;
  font-weight: 700;
  color: #303133;
}
.total-amount.highlight {
  color: #f56c6c;
}
</style>
