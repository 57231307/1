<template>
  <div class="bom-form">
    <el-form
      ref="formRef"
      :model="localFormData"
      :rules="formRules"
      label-width="100px"
      :aria-label="$t('bomModule.form.ariaLabel')"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="$t('bomModule.form.productName')" prop="product_id">
            <el-select
              v-model="localFormData.product_id"
              filterable
              :placeholder="$t('bomModule.form.productNamePlaceholder')"
              style="width: 100%"
            >
              <el-option v-for="p in products" :key="p.id" :label="p.product_name" :value="p.id" />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="$t('bomModule.form.version')" prop="version">
            <el-input
              v-model="localFormData.version"
              :placeholder="$t('bomModule.form.versionPlaceholder')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="$t('bomModule.form.isDefault')" prop="is_default">
            <el-switch
              v-model="localFormData.is_default"
              :active-text="$t('bomModule.form.yes')"
              :inactive-text="$t('bomModule.form.no')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="$t('bomModule.form.status')" prop="status">
            <el-select
              v-model="localFormData.status"
              :placeholder="$t('bomModule.form.statusPlaceholder')"
              style="width: 100%"
            >
              <el-option :label="$t('bomModule.status.draft')" value="draft" />
              <el-option :label="$t('bomModule.status.active')" value="active" />
              <el-option :label="$t('bomModule.status.archived')" value="archived" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="$t('bomModule.form.remark')" prop="remark">
        <el-input
          v-model="localFormData.remark"
          type="textarea"
          :rows="2"
          :placeholder="$t('bomModule.form.remarkPlaceholder')"
        />
      </el-form-item>
    </el-form>

    <div class="items-section">
      <div class="items-header">
        <h3 class="items-title">{{ $t('bomModule.form.itemsTitle') }}</h3>
        <el-button type="primary" size="small" @click="handleAddItem">
          <el-icon><Plus /></el-icon>
          {{ $t('bomModule.form.addItem') }}
        </el-button>
      </div>

      <el-table
        :data="localFormData.items"
        border
        size="small"
        class="items-table"
        :aria-label="$t('bomModule.form.itemsAriaLabel')"
      >
        <el-table-column :label="$t('bomModule.form.materialName')" min-width="180">
          <template #default="{ row }">
            <el-select
              v-model="row.material_id"
              filterable
              :placeholder="$t('bomModule.form.materialNamePlaceholder')"
              style="width: 100%"
            >
              <el-option v-for="p in products" :key="p.id" :label="p.product_name" :value="p.id" />
            </el-select>
          </template>
        </el-table-column>
        <el-table-column :label="$t('bomModule.form.quantity')" width="120">
          <template #default="{ row }">
            <el-input-number
              v-model="row.quantity"
              :min="0"
              :precision="2"
              controls-position="right"
              style="width: 100%"
            />
          </template>
        </el-table-column>
        <el-table-column :label="$t('bomModule.form.unit')" width="100">
          <template #default="{ row }">
            <!-- maxlength 对齐后端 DDL bom_items.unit VARCHAR(20)（m0007:40）与
                 DTO 校验 max=20（bom_handler.rs::CreateBomItemPayload） -->
            <el-input
              v-model="row.unit"
              :maxlength="20"
              :placeholder="$t('bomModule.form.unitPlaceholder')"
            />
          </template>
        </el-table-column>
        <el-table-column :label="$t('bomModule.form.lossRate')" width="130">
          <template #default="{ row }">
            <!-- 损耗率 = API 百分比数值口径（10 = 10%），键名与后端 scrap_rate 同名；
                 0–100 + 两位小数（0.01% 粒度）与后端写边界校验同口径，
                 比率换算只发生在后端边界，前端不二次乘除 -->
            <el-input-number
              v-model="row.scrap_rate"
              :min="0"
              :max="100"
              :precision="2"
              controls-position="right"
              style="width: 100%"
            />
          </template>
        </el-table-column>
        <el-table-column :label="$t('bomModule.form.operation')" width="80" fixed="right">
          <template #default="{ $index }">
            <el-button type="danger" link size="small" @click="handleRemoveItem($index)">
              {{ $t('bomModule.form.delete') }}
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </div>

    <div class="form-footer">
      <el-button @click="handleCancel">{{ $t('bomModule.form.cancel') }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
        $t('bomModule.form.save')
      }}</el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import type { Bom } from '@/api/bom';
import { getProductList } from '@/api/product';
import type { Product } from '@/api/product';

const { t } = useI18n({ useScope: 'global' });

/**
 * 编辑回显灌入的行形状：等于后端 BomItemResponse（api/bom.ts::BomItem）。
 * quantity/scrap_rate 后端 rust_decimal 序列化为 JSON 字符串，此处类型如实为串；
 * 判空用 `== null`（合法 0 不可误杀）。material_id 在回显行可缺（新行未选料）。
 */
interface BomItemEcho {
  id?: number;
  bom_id?: number;
  material_id?: number;
  quantity: string;
  unit: string | null;
  scrap_rate: string | null;
  sort_order?: number | null;
}

/**
 * 控件绑定态行形状：el-input-number 需要 number。仅在「回显数据灌入控件」这一点
 * 由 toFormRow 归一得到，属绑定边界，不在数据层（api / index.vue props）伪造类型。
 */
interface BomFormRow {
  id?: number;
  bom_id?: number;
  material_id?: number;
  quantity: number;
  unit: string | null;
  scrap_rate: number | null;
  sort_order?: number | null;
}

const props = defineProps<{
  formData: {
    id?: number;
    product_id?: number;
    product_name: string;
    version: string;
    is_default: boolean;
    status: 'draft' | 'active' | 'archived';
    remark: string;
    // 行对象键名对齐后端 BomItemResponse（见 api/bom.ts BomItem 注释）：
    // 编辑回显直接灌 GET /boms/:id 的 items，同名键才接得住；
    // quantity/scrap_rate 类型如实为 string/string|null（Decimal 序列化是字符串），
    // 喂 el-input-number 的 number 归一只发生在下方 toFormRow 控件绑定边界。
    items: BomItemEcho[];
  };
  mode: 'create' | 'edit';
}>();

// v11 批次 169 P2-1 修复：emit submit data: any 改为 Partial<Bom>
const emit = defineEmits<{
  submit: [data: Partial<Bom>];
  cancel: [];
}>();

const formRef = ref<FormInstance>();
const submitLoading = ref(false);

// 后端 CreateBomRequest 必填 product_id + items[].material_id（i32），
// 表单由名称输入改为产品下拉，直接绑定 ID
const products = ref<Product[]>([]);

const loadProducts = async () => {
  try {
    const res = await getProductList({ page: 1, page_size: 1000 });
    const data = res.data as { items?: Product[]; list?: Product[] } | undefined;
    products.value = data?.items || data?.list || [];
  } catch (error) {
    ElMessage.warning(t('bomModule.form.productNamePlaceholder'));
  }
};

onMounted(loadProducts);

/**
 * 归一灌入点（回显 → 控件）：后端 rust_decimal 序列化为 JSON 字符串，el-input-number 需 number，
 * 只在数据进入控件的这一点显式 Number()，不在数据层（api/index.vue props）伪造类型。
 * 判空一律 `== null`：scrap_rate 合法 0（"0"/"0.00"）须保留为数字 0，
 * 不可被 `!value` / `Number('')` 误杀成空（本仓已多次因 `!value` 误杀合法 0 返工）。
 */
const toFormRow = (item: BomItemEcho): BomFormRow => ({
  ...item,
  quantity: Number(item.quantity),
  scrap_rate: item.scrap_rate == null ? null : Number(item.scrap_rate),
});

const localFormData = ref({
  product_id: props.formData.product_id,
  product_name: props.formData.product_name,
  version: props.formData.version,
  is_default: props.formData.is_default,
  status: props.formData.status,
  remark: props.formData.remark,
  items: [...props.formData.items.map(toFormRow)],
});

watch(
  () => props.formData,
  newVal => {
    localFormData.value = {
      product_id: newVal.product_id,
      product_name: newVal.product_name,
      version: newVal.version,
      is_default: newVal.is_default,
      status: newVal.status,
      remark: newVal.remark,
      items: [...newVal.items.map(toFormRow)],
    };
  },
  { deep: true }
);

const formRules: FormRules = {
  product_id: [
    { required: true, message: t('bomModule.form.productNameRequired'), trigger: 'change' },
  ],
  version: [{ required: true, message: t('bomModule.form.versionRequired'), trigger: 'blur' }],
  status: [{ required: true, message: t('bomModule.form.statusRequired'), trigger: 'change' }],
};

const handleAddItem = () => {
  localFormData.value.items.push({
    material_id: undefined,
    quantity: 1,
    unit: '',
    scrap_rate: 0,
  });
};

const handleRemoveItem = (index: number) => {
  localFormData.value.items.splice(index, 1);
};

const handleSubmit = async () => {
  if (!formRef.value) return;

  await formRef.value.validate(async valid => {
    if (!valid) return;

    const hasEmptyItems = localFormData.value.items.some(item => !item.material_id || !item.unit);
    if (hasEmptyItems) {
      ElMessage.warning(t('bomModule.form.itemsIncomplete'));
      return;
    }

    submitLoading.value = true;
    try {
      emit('submit', {
        product_id: localFormData.value.product_id,
        product_name: localFormData.value.product_name,
        // 后端 CreateBomPayload.version 为 Option<i32>，version 输入框是 el-input（字符串），
        // 提交时转数字，否则 "1" → invalid type: string, expected i32 → 422
        version: Number(localFormData.value.version),
        is_default: localFormData.value.is_default,
        status: localFormData.value.status,
        remark: localFormData.value.remark,
        items: localFormData.value.items,
      } as unknown as Partial<Bom>);
    } finally {
      submitLoading.value = false;
    }
  });
};

const handleCancel = () => {
  emit('cancel');
};
</script>

<style scoped>
.bom-form {
  padding: 10px 0;
}
.items-section {
  margin-top: 24px;
}
.items-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}
.items-title {
  font-size: 16px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}
.items-table {
  margin-bottom: 20px;
}
.form-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  margin-top: 24px;
  padding-top: 20px;
  border-top: 1px solid #ebeef5;
}
</style>
