/**
 * useQualityLookups - 质检页共享的主数据（产品 / 检验人）
 *
 * quality_inspection_records 只存 product_id 与 inspector_id，而列表与表单都要显示名称；
 * 记录子 tab 与父页面都要用，故在模块级缓存一份，整页只发一次请求。
 * 主数据里查不到的 id 一律告警并退回显示 id 本身，不猜测名称。
 */
import { computed, ref } from 'vue';
import { getProductList, type Product } from '@/api/product';
import { getUserList, type User } from '@/api/user';
import { logger } from '@/utils/logger';

const products = ref<Product[]>([]);
const inspectors = ref<User[]>([]);
let loading: Promise<void> | null = null;

export function useQualityLookups() {
  const load = async (): Promise<void> => {
    if (!loading) {
      loading = (async () => {
        try {
          const [productRes, userRes] = await Promise.all([
            getProductList({ page: 1, page_size: 1000 }),
            getUserList({ page: 1, page_size: 1000 }),
          ]);
          products.value = productRes.data.items ?? [];
          inspectors.value = userRes.data.items ?? [];
        } catch (err: unknown) {
          // 失败后允许下一次挂载重试，不把失败的 Promise 永久缓存
          loading = null;
          logger.warn('质检主数据（产品/检验人）加载失败', { error: String(err) });
        }
      })();
    }
    return loading;
  };

  const productOptions = computed(() =>
    products.value.map(p => ({ value: p.id, label: p.product_name || p.product_code }))
  );
  const inspectorOptions = computed(() =>
    inspectors.value.map(u => ({ value: u.id, label: u.real_name || u.username }))
  );

  const productName = (id: number): string => {
    const hit = productOptions.value.find(p => p.value === id);
    if (!hit) {
      logger.warn('质检记录引用了主数据中不存在的产品', { product_id: id });
      return String(id);
    }
    return hit.label;
  };

  const inspectorName = (id: number | null): string => {
    if (id === null) return '';
    const hit = inspectorOptions.value.find(u => u.value === id);
    if (!hit) {
      logger.warn('质检记录引用了系统中不存在的检验人', { inspector_id: id });
      return String(id);
    }
    return hit.label;
  };

  return { load, productOptions, inspectorOptions, productName, inspectorName };
}
