/**
 * useSkuMappingList - SKU 对照表列表 composable
 */
import { reactive, ref, onMounted } from 'vue';
import { msg } from '@/utils/message';
import {
  getSkuMappingList,
  deleteSkuMapping,
  type SkuMapping,
  type SkuMappingQueryParams,
} from '@/api/sku-mapping';
import { getProductList } from '@/api/product';
import { getSupplierList } from '@/api/supplier';
import type { Product } from '@/api/product';
import type { Supplier } from '@/api/supplier';

export function useSkuMappingList() {
  const loading = ref(false);
  const list = ref<SkuMapping[]>([]);
  const total = ref(0);
  const products = ref<Product[]>([]);
  const suppliers = ref<Supplier[]>([]);

  const queryParams = reactive<SkuMappingQueryParams>({
    page: 1,
    page_size: 20,
    product_id: undefined,
    supplier_id: undefined,
    keyword: '',
  });

  const loadData = async () => {
    loading.value = true;
    try {
      const res = await getSkuMappingList(queryParams);
      list.value = res.data.items;
      total.value = res.data.total;
    } catch (error: unknown) {
      msg.loadFail();
      console.error('[sku-mapping] loadData error:', error);
    } finally {
      loading.value = false;
    }
  };

  const loadProducts = async () => {
    try {
      const res = await getProductList({ page_size: 200 });
      products.value = res.data.items;
    } catch (error: unknown) {
      console.error('[sku-mapping] loadProducts error:', error);
    }
  };

  const loadSuppliers = async () => {
    try {
      const res = await getSupplierList({ page_size: 200 });
      suppliers.value = res.data.items;
    } catch (error: unknown) {
      console.error('[sku-mapping] loadSuppliers error:', error);
    }
  };

  const handleQuery = () => {
    queryParams.page = 1;
    loadData();
  };

  const handleReset = () => {
    queryParams.product_id = undefined;
    queryParams.supplier_id = undefined;
    queryParams.keyword = '';
    queryParams.page = 1;
    loadData();
  };

  const handlePageChange = (page: number) => {
    queryParams.page = page;
    loadData();
  };

  const handleSizeChange = (size: number) => {
    queryParams.page_size = size;
    queryParams.page = 1;
    loadData();
  };

  const handleDelete = async (id: number) => {
    try {
      await deleteSkuMapping(id);
      msg.deleteOk();
      loadData();
    } catch (error: unknown) {
      msg.deleteFail();
      console.error('[sku-mapping] handleDelete error:', error);
    }
  };

  onMounted(() => {
    loadData();
    loadProducts();
    loadSuppliers();
  });

  return {
    loading,
    list,
    total,
    products,
    suppliers,
    queryParams,
    loadData,
    handleQuery,
    handleReset,
    handlePageChange,
    handleSizeChange,
    handleDelete,
  };
}
