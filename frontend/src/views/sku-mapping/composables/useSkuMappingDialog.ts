/**
 * useSkuMappingDialog - SKU 对照表新建/编辑对话框 composable
 *
 * 注意：proc 传给子组件时传 reactive 代理本身，不可对象字面量快照，
 * 否则 dialogVisible 断链（本仓已多次此坑）。
 */
import { reactive, ref } from 'vue';
import { msg } from '@/utils/message';
import {
  createSkuMapping,
  updateSkuMapping,
  getProductColors,
  type SkuMapping,
  type CreateSkuMappingPayload,
} from '@/api/sku-mapping';
import type { ProductColor } from '@/api/product';

export interface SkuMappingFormState {
  id?: number;
  product_id: number | undefined;
  product_color_id: number | null;
  color_no: string;
  supplier_id: number | undefined;
  supplier_product_id: number;
  supplier_product_code: string;
  supplier_product_color_id: number | null;
  supplier_color_no: string;
  supplier_price: string;
  min_order_quantity: string;
  lead_time: number | null;
  is_primary: boolean;
  is_enabled: boolean;
}

const defaultForm = (): SkuMappingFormState => ({
  product_id: undefined,
  product_color_id: null,
  color_no: '',
  supplier_id: undefined,
  supplier_product_id: 0,
  supplier_product_code: '',
  supplier_product_color_id: null,
  supplier_color_no: '',
  supplier_price: '',
  min_order_quantity: '',
  lead_time: null,
  is_primary: false,
  is_enabled: true,
});

export function useSkuMappingDialog(onSuccess: () => void) {
  const dialogVisible = ref(false);
  const isEdit = ref(false);
  const saving = ref(false);

  const form = reactive<SkuMappingFormState>(defaultForm());

  // 色号懒加载（虚拟滚动数据源）
  const colorOptions = ref<ProductColor[]>([]);
  const colorLoading = ref(false);

  const loadColors = async (productId: number, keyword?: string) => {
    colorLoading.value = true;
    try {
      const res = await getProductColors(productId, { keyword, page_size: 50 });
      colorOptions.value = res.data;
    } catch (error: unknown) {
      console.error('[sku-mapping] loadColors error:', error);
      colorOptions.value = [];
    } finally {
      colorLoading.value = false;
    }
  };

  const handleProductChange = (productId: number | undefined) => {
    form.product_color_id = null;
    form.color_no = '';
    colorOptions.value = [];
    if (productId) {
      loadColors(productId);
    }
  };

  const handleColorChange = (colorId: number | null) => {
    const color = colorOptions.value.find(c => c.id === colorId);
    form.color_no = color ? color.color_no : '';
  };

  const openCreate = () => {
    Object.assign(form, defaultForm());
    isEdit.value = false;
    dialogVisible.value = true;
  };

  const openEdit = (row: SkuMapping) => {
    form.id = row.id;
    form.product_id = row.product_id;
    form.product_color_id = row.product_color_id;
    form.color_no = row.color_no ?? '';
    form.supplier_id = row.supplier_id;
    form.supplier_product_id = row.supplier_product_id;
    form.supplier_product_code = row.supplier_product_code;
    form.supplier_product_color_id = row.supplier_product_color_id;
    form.supplier_color_no = row.supplier_color_no ?? '';
    form.supplier_price = row.supplier_price ?? '';
    form.min_order_quantity = row.min_order_quantity ?? '';
    form.lead_time = row.lead_time;
    form.is_primary = row.is_primary;
    form.is_enabled = row.is_enabled;
    isEdit.value = true;
    dialogVisible.value = true;
    if (row.product_id) {
      loadColors(row.product_id);
    }
  };

  const submit = async () => {
    if (!form.product_id || !form.supplier_id) {
      msg.warning('skuMappingFormRequired');
      return;
    }
    saving.value = true;
    try {
      const payload: CreateSkuMappingPayload = {
        product_id: form.product_id,
        product_color_id: form.product_color_id,
        supplier_id: form.supplier_id,
        supplier_product_id: form.supplier_product_id,
        supplier_product_color_id: form.supplier_product_color_id,
        supplier_price: form.supplier_price || null,
        min_order_quantity: form.min_order_quantity || null,
        lead_time: form.lead_time,
        is_primary: form.is_primary,
        is_enabled: form.is_enabled,
      };
      if (isEdit.value && form.id) {
        await updateSkuMapping(form.id, payload);
        msg.saveOk();
      } else {
        await createSkuMapping(payload);
        msg.createOk();
      }
      dialogVisible.value = false;
      onSuccess();
    } catch (error: unknown) {
      msg.saveFail();
      console.error('[sku-mapping] submit error:', error);
    } finally {
      saving.value = false;
    }
  };

  return {
    dialogVisible,
    isEdit,
    saving,
    form,
    colorOptions,
    colorLoading,
    loadColors,
    handleProductChange,
    handleColorChange,
    openCreate,
    openEdit,
    submit,
  };
}
