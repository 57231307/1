<template>
  <div class="seasonal-rules-tab">
    <el-card>
      <div class="toolbar">
        <el-button type="primary" @click="handleCreate">{{
          $t('colorPrices.seasonalRules.create')
        }}</el-button>
        <el-button plain @click="loadRules">{{
          $t('colorPrices.seasonalRules.refresh')
        }}</el-button>
      </div>

      <el-table v-loading="loading" :data="rules" border stripe>
        <el-table-column prop="id" label="ID" width="70" />
        <el-table-column
          prop="rule_name"
          :label="$t('colorPrices.seasonalRules.table.ruleName')"
          min-width="140"
          show-overflow-tooltip
        />
        <el-table-column
          prop="season"
          :label="$t('colorPrices.seasonalRules.table.season')"
          width="130"
        >
          <template #default="{ row }">
            <el-tag>{{ getSeasonLabel(row.season) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="product_category_id"
          :label="$t('colorPrices.seasonalRules.table.categoryId')"
          width="110"
        />
        <el-table-column
          prop="adjustment_type"
          :label="$t('colorPrices.seasonalRules.table.adjustType')"
          width="110"
        >
          <template #default="{ row }">
            {{
              row.adjustment_type === 'percentage'
                ? $t('colorPrices.seasonalRules.dialog.percentage')
                : $t('colorPrices.seasonalRules.dialog.fixed')
            }}
          </template>
        </el-table-column>
        <el-table-column
          prop="adjustment_value"
          :label="$t('colorPrices.seasonalRules.table.adjustValue')"
          width="100"
        />
        <el-table-column
          prop="valid_from"
          :label="$t('colorPrices.seasonalRules.table.validFrom')"
          width="110"
        />
        <el-table-column
          prop="valid_until"
          :label="$t('colorPrices.seasonalRules.table.validUntil')"
          width="110"
        />
        <el-table-column
          prop="is_active"
          :label="$t('colorPrices.seasonalRules.table.isActive')"
          width="90"
        >
          <template #default="{ row }">
            <el-tag :type="row.is_active ? 'success' : 'info'">
              {{
                row.is_active ? $t('colorPrices.common.enable') : $t('colorPrices.common.disable')
              }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="description"
          :label="$t('colorPrices.seasonalRules.table.description')"
          min-width="140"
          show-overflow-tooltip
        />
        <el-table-column
          :label="$t('colorPrices.seasonalRules.table.operation')"
          fixed="right"
          width="150"
        >
          <template #default="{ row }">
            <el-button link type="primary" @click="handleEdit(row)">{{
              $t('colorPrices.detail.edit')
            }}</el-button>
            <el-button link type="danger" @click="handleDelete(row)">{{
              $t('colorPrices.common.delete')
            }}</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog
      v-model="dialogVisible"
      :title="
        editingId
          ? $t('colorPrices.seasonalRules.dialog.editTitle')
          : $t('colorPrices.seasonalRules.dialog.createTitle')
      "
      width="520"
    >
      <el-form ref="formRef" :model="form" :rules="formRules" label-width="110px">
        <el-form-item :label="$t('colorPrices.seasonalRules.dialog.ruleName')" required>
          <el-input v-model="form.rule_name" />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.seasonalRules.dialog.season')" required>
          <el-select v-model="form.season" style="width: 100%">
            <el-option label="SS" value="SS" />
            <el-option label="AW" value="AW" />
            <el-option label="HOLIDAY" value="HOLIDAY" />
          </el-select>
        </el-form-item>
        <el-form-item
          :label="$t('colorPrices.seasonalRules.dialog.categoryId')"
          prop="product_category_id"
        >
          <el-input v-model="form.product_category_id" inputmode="numeric" />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.seasonalRules.dialog.adjustType')" required>
          <el-radio-group v-model="form.adjustment_type">
            <el-radio value="percentage">{{
              $t('colorPrices.seasonalRules.dialog.percentage')
            }}</el-radio>
            <el-radio value="fixed">{{ $t('colorPrices.seasonalRules.dialog.fixed') }}</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item
          :label="$t('colorPrices.seasonalRules.dialog.adjustValue')"
          prop="adjustment_value"
          required
        >
          <el-input v-model="form.adjustment_value" inputmode="decimal" />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.seasonalRules.dialog.validFrom')" required>
          <el-input v-model="form.valid_from" placeholder="2026-01-01" />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.seasonalRules.dialog.validUntil')">
          <el-input v-model="form.valid_until" placeholder="2026-12-31" />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.seasonalRules.dialog.description')">
          <el-input v-model="form.description" type="textarea" :rows="2" />
        </el-form-item>
        <el-form-item v-if="editingId" :label="$t('colorPrices.seasonalRules.table.isActive')">
          <el-switch v-model="form.is_active" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ $t('colorPrices.common.cancel') }}</el-button>
        <el-button type="primary" :loading="saving" @click="handleSave">{{
          $t('colorPrices.common.confirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus';
import { isDialogDismissal, rethrowNonDismissal } from '@/utils/monitor';
import {
  getSeasonalRuleList,
  createSeasonalRule,
  getSeasonalRule,
  updateSeasonalRule,
  deleteSeasonalRule,
  type SeasonalPriceRule,
} from '@/api/color-price';
import { numberInputRules, numericView } from '@/utils/number-input';

const { t } = useI18n({ useScope: 'global' });

const getSeasonLabel = (season: string | null | undefined) =>
  t(`colorPrices.season.${season || 'default'}`);

const loading = ref(false);
const saving = ref(false);
const rules = ref<SeasonalPriceRule[]>([]);
const dialogVisible = ref(false);
const editingId = ref<number | null>(null);

// 手写录入文本模型（数字约束收进 numberInputRules 校验器，不靠步进按钮）
// 类目 ID 为可选项初值空串（提交回 null 三态）；调整值原控件无下界，负数调价为合法语义
const form = reactive({
  rule_name: '',
  season: 'SS' as 'SS' | 'AW' | 'HOLIDAY',
  product_category_id: '',
  adjustment_type: 'percentage' as 'percentage' | 'fixed',
  adjustment_value: '0',
  valid_from: '',
  valid_until: '',
  description: '',
  is_active: true,
});

// 弹窗数值录入校验（范围与精度靠校验器，不靠步进按钮）
const formRef = ref<FormInstance>();
const formRules = computed<FormRules>(() => ({
  product_category_id: numberInputRules({
    label: t('colorPrices.seasonalRules.dialog.categoryId'),
    min: 1,
    integer: true,
  }),
  adjustment_value: numberInputRules({
    label: t('colorPrices.seasonalRules.dialog.adjustValue'),
    required: true,
    precision: 2,
  }),
}));

const loadRules = async () => {
  loading.value = true;
  try {
    const res = await getSeasonalRuleList({});
    rules.value = res.items || [];
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : String(e));
  } finally {
    loading.value = false;
  }
};

const handleCreate = () => {
  editingId.value = null;
  Object.assign(form, {
    rule_name: '',
    season: 'SS',
    product_category_id: '',
    adjustment_type: 'percentage',
    adjustment_value: '0',
    valid_from: '',
    valid_until: '',
    description: '',
    is_active: true,
  });
  dialogVisible.value = true;
};

const handleEdit = async (row: SeasonalPriceRule) => {
  editingId.value = row.id;
  dialogVisible.value = true;
  try {
    const detail = await getSeasonalRule(row.id);
    Object.assign(form, {
      rule_name: detail.rule_name,
      season: detail.season as 'SS' | 'AW' | 'HOLIDAY',
      product_category_id:
        detail.product_category_id === null || detail.product_category_id === undefined
          ? ''
          : String(detail.product_category_id),
      adjustment_type: detail.adjustment_type as 'percentage' | 'fixed',
      adjustment_value: numericView(Number(detail.adjustment_value), 2),
      valid_from: detail.valid_from,
      valid_until: detail.valid_until || '',
      description: detail.description || '',
      is_active: detail.is_active,
    });
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : String(e));
  }
};

const handleSave = async () => {
  if (!form.rule_name || !form.valid_from) {
    ElMessage.warning(
      `${t('colorPrices.seasonalRules.dialog.ruleName')} / ${t('colorPrices.seasonalRules.dialog.validFrom')}`
    );
    return;
  }
  let valid = false;
  await formRef.value?.validate(ok => {
    valid = ok;
  });
  if (!valid) return;
  saving.value = true;
  try {
    if (editingId.value) {
      await updateSeasonalRule(editingId.value, {
        rule_name: form.rule_name,
        season: form.season,
        product_category_id:
          form.product_category_id.trim() === '' ? null : Number(form.product_category_id),
        adjustment_type: form.adjustment_type,
        adjustment_value: Number(form.adjustment_value),
        valid_from: form.valid_from,
        valid_until: form.valid_until || null,
        is_active: form.is_active,
        description: form.description || null,
      });
    } else {
      await createSeasonalRule({
        rule_name: form.rule_name,
        season: form.season,
        product_category_id:
          form.product_category_id.trim() === '' ? null : Number(form.product_category_id),
        adjustment_type: form.adjustment_type,
        adjustment_value: Number(form.adjustment_value),
        valid_from: form.valid_from,
        valid_until: form.valid_until || null,
        description: form.description || null,
      });
    }
    ElMessage.success(t('colorPrices.message.createSuccess'));
    dialogVisible.value = false;
    await loadRules();
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : String(e));
  } finally {
    saving.value = false;
  }
};

const handleDelete = async (row: SeasonalPriceRule) => {
  try {
    await ElMessageBox.confirm(
      `${t('colorPrices.common.delete')} #${row.id}?`,
      t('colorPrices.common.confirm'),
      { type: 'warning' }
    );
  } catch (error: unknown) {
    if (isDialogDismissal(error)) return;
    rethrowNonDismissal('colorPrices.SeasonalRulesTab.handleDelete', error);
  }
  try {
    await deleteSeasonalRule(row.id);
    ElMessage.success(t('colorPrices.message.deleteSuccess'));
    await loadRules();
  } catch (e: unknown) {
    if (isDialogDismissal(e)) return;
    ElMessage.error(e instanceof Error ? e.message : String(e));
  }
};

onMounted(() => {
  loadRules();
});
</script>

<style scoped>
.seasonal-rules-tab {
  padding: 20px;
}
.toolbar {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
}
</style>
