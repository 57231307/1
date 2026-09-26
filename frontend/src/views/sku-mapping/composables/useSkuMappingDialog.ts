/**
 * useSkuMappingDialog - SKU 对照表新建/编辑对话框 composable
 *
 * 注意：proc 传给子组件时传 reactive 代理本身，不可对象字面量快照，
 * 否则 dialogVisible 断链（本仓已多次此坑）。
 *
 * 供应商侧为三级级联：供应商 → 供应商商品 → 供应商色号。
 * 后两级用 el-select-v2（虚拟滚动）+ 远程搜索 + 分页，禁止一次性拉全量
 * （单商品可上千色号）。切换上级会清空下级。编辑态按 id 各发一次单条 GET 回显 label。
 */
import { computed, reactive, ref } from 'vue';
import { msg } from '@/utils/message';
import {
  createSkuMapping,
  updateSkuMapping,
  getProductColors,
  type SkuMapping,
  type CreateSkuMappingPayload,
} from '@/api/sku-mapping';
import {
  getSupplierProductList,
  getSupplierProduct,
  getSupplierProductColorList,
  getSupplierProductColor,
  type SupplierProduct,
  type SupplierProductColor,
} from '@/api/supplier-product';
import type { ProductColor } from '@/api/product';

export interface SkuMappingFormState {
  id?: number;
  product_id: number | undefined;
  product_color_id: number | null;
  color_no: string;
  supplier_id: number | undefined;
  supplier_product_id: number | undefined;
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
  supplier_product_id: undefined,
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

  // 我方色号懒加载（虚拟滚动数据源）
  const colorOptions = ref<ProductColor[]>([]);
  const colorLoading = ref(false);

  // 供应商商品（第二级）
  const supplierProductOptions = ref<SupplierProduct[]>([]);
  const supplierProductLoading = ref(false);
  // 已选供应商商品（远程搜索换页后仍需回显 label，钉住它）
  const pinnedSupplierProduct = ref<SupplierProduct | null>(null);

  // 供应商色号（第三级，高基数 → 虚拟滚动 + 远程分页）
  const supplierColorOptions = ref<SupplierProductColor[]>([]);
  const supplierColorLoading = ref(false);
  const pinnedSupplierColor = ref<SupplierProductColor | null>(null);

  // ============ 我方色号 ============
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

  // ============ 供应商商品（第二级） ============
  const loadSupplierProducts = async (supplierId: number, keyword?: string) => {
    supplierProductLoading.value = true;
    try {
      const res = await getSupplierProductList({
        supplier_id: supplierId,
        keyword,
        page: 1,
        page_size: 50,
      });
      supplierProductOptions.value = res.data.items;
    } catch (error: unknown) {
      console.error('[sku-mapping] loadSupplierProducts error:', error);
      supplierProductOptions.value = [];
    } finally {
      supplierProductLoading.value = false;
    }
  };

  const handleSupplierChange = (supplierId: number | undefined) => {
    // 切换供应商：清空第二、三级及其数据源
    form.supplier_product_id = undefined;
    form.supplier_product_code = '';
    form.supplier_product_color_id = null;
    form.supplier_color_no = '';
    supplierProductOptions.value = [];
    supplierColorOptions.value = [];
    pinnedSupplierProduct.value = null;
    pinnedSupplierColor.value = null;
    if (supplierId) {
      loadSupplierProducts(supplierId);
    }
  };

  const handleSupplierProductChange = (productId: number | undefined) => {
    const picked = supplierProductOptions.value.find(p => p.id === productId) ?? null;
    pinnedSupplierProduct.value = picked;
    form.supplier_product_code = picked ? picked.product_code : '';
    // 切换商品：清空第三级
    form.supplier_product_color_id = null;
    form.supplier_color_no = '';
    supplierColorOptions.value = [];
    pinnedSupplierColor.value = null;
    if (productId) {
      loadSupplierColors(productId);
    }
  };

  // ============ 供应商色号（第三级，虚拟滚动 + 远程分页） ============
  const loadSupplierColors = async (supplierProductId: number, keyword?: string) => {
    supplierColorLoading.value = true;
    try {
      const res = await getSupplierProductColorList({
        supplier_product_id: supplierProductId,
        keyword,
        page: 1,
        page_size: 50,
      });
      supplierColorOptions.value = res.data.items;
    } catch (error: unknown) {
      console.error('[sku-mapping] loadSupplierColors error:', error);
      supplierColorOptions.value = [];
    } finally {
      supplierColorLoading.value = false;
    }
  };

  const handleSupplierColorChange = (colorId: number | null) => {
    const picked = supplierColorOptions.value.find(c => c.id === colorId) ?? null;
    pinnedSupplierColor.value = picked;
    form.supplier_color_no = picked ? picked.color_no : '';
  };

  // 组装 el-select-v2 需要的 {value,label}[]，钉住项前置以保证换页后仍能回显
  const supplierProductV2Options = computed(() => {
    const merged: SupplierProduct[] = [];
    if (pinnedSupplierProduct.value) merged.push(pinnedSupplierProduct.value);
    for (const p of supplierProductOptions.value) {
      if (!merged.some(m => m.id === p.id)) merged.push(p);
    }
    return merged.map(p => ({ value: p.id, label: `${p.product_code} - ${p.product_name}` }));
  });

  const supplierColorV2Options = computed(() => {
    const merged: SupplierProductColor[] = [];
    if (pinnedSupplierColor.value) merged.push(pinnedSupplierColor.value);
    for (const c of supplierColorOptions.value) {
      if (!merged.some(m => m.id === c.id)) merged.push(c);
    }
    return merged.map(c => ({ value: c.id, label: `${c.color_no} - ${c.color_name}` }));
  });

  const openCreate = () => {
    Object.assign(form, defaultForm());
    supplierProductOptions.value = [];
    supplierColorOptions.value = [];
    pinnedSupplierProduct.value = null;
    pinnedSupplierColor.value = null;
    colorOptions.value = [];
    isEdit.value = false;
    dialogVisible.value = true;
  };

  const openEdit = async (row: SkuMapping) => {
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
    supplierProductOptions.value = [];
    supplierColorOptions.value = [];
    pinnedSupplierProduct.value = null;
    pinnedSupplierColor.value = null;
    isEdit.value = true;
    dialogVisible.value = true;
    if (row.product_id) {
      loadColors(row.product_id);
    }
    // 预载下拉首页，使编辑态打开即可直接改选（选中项由单条 GET 钉住回显）
    if (row.supplier_id) {
      loadSupplierProducts(row.supplier_id);
    }
    if (row.supplier_product_id) {
      loadSupplierColors(row.supplier_product_id);
    }
    // 回显 label：各发一次单条 GET，不拉全量
    if (row.supplier_product_id) {
      try {
        const res = await getSupplierProduct(row.supplier_product_id);
        pinnedSupplierProduct.value = res.data;
      } catch (error: unknown) {
        console.error('[sku-mapping] echo supplier product error:', error);
      }
    }
    if (row.supplier_product_color_id) {
      try {
        const res = await getSupplierProductColor(row.supplier_product_color_id);
        pinnedSupplierColor.value = res.data;
      } catch (error: unknown) {
        console.error('[sku-mapping] echo supplier color error:', error);
      }
    }
  };

  const submit = async () => {
    if (!form.product_id || !form.supplier_id || !form.supplier_product_id) {
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
    // 我方色号
    colorOptions,
    colorLoading,
    loadColors,
    handleProductChange,
    handleColorChange,
    // 供应商商品
    supplierProductLoading,
    supplierProductV2Options,
    loadSupplierProducts,
    handleSupplierChange,
    handleSupplierProductChange,
    // 供应商色号
    supplierColorLoading,
    supplierColorV2Options,
    loadSupplierColors,
    handleSupplierColorChange,
    openCreate,
    openEdit,
    submit,
  };
}
