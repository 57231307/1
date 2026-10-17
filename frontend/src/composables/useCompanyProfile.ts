/**
 * 公司档案共享 composable（服务端化·前端）
 *
 * 设计要点：
 * - 模块级共享 `ref`：整个应用只持有一份对外硬投影副本，头部品牌（MainLayout）、
 *   document.title（router）等多个消费点读同一实例，避免每个组件各拉一次 `/company-profile`。
 * - `load()`：登录态拉一次并缓存（MainLayout 挂载于已鉴权外壳内，天然保证已登录）。
 *   并发/重复调用经 `loadingPromise` 去重，命中缓存直接返回。
 * - `refresh()`：保存后回填共享副本，强制重拉。
 * - 空态（后端无档案/载荷缺键）返回 `null`，由展示层决定回退文案，本层不做 `?? 默认` 掩盖。
 * - 失败显式 `logger.error`（不静默吞错）；不采用 `t(key) || '中文'` 兜底
 *   （vue-i18n 取不到键会返回键名本身，兜底分支不可达，只会把缺键降级成裸字面量）。
 */
import { ref } from 'vue';
import { getCompanyProfilePublic, type CompanyProfilePublic } from '@/api/company-profile';
import { logger } from '@/utils/logger';
import { i18n } from '@/i18n';

// 模块级共享实例（非每组件各一份）：null 表示未加载 / 无档案。
const company = ref<CompanyProfilePublic | null>(null);
// 在途 / 已加载标记：保证登录态只拉一次并缓存。
let loaded = false;
let loadingPromise: Promise<void> | null = null;

async function doLoad(): Promise<void> {
  try {
    const res = await getCompanyProfilePublic();
    company.value = res.data;
    loaded = true;
  } catch (error) {
    // 显式留痕：档案读失败会让品牌回退到 i18n 字面量，需在控制台可追溯，不静默。
    logger.error(i18n.global.t('system.company.message.loadFailed'), error);
    company.value = null;
  }
}

export function useCompanyProfile() {
  /** 登录态拉取一次并缓存；命中缓存或已有在途请求则复用，不重复发请求。 */
  const load = (): Promise<void> => {
    if (loaded) return Promise.resolve();
    if (loadingPromise) return loadingPromise;
    loadingPromise = doLoad().finally(() => {
      loadingPromise = null;
    });
    return loadingPromise;
  };

  /** 强制重拉，供 CompanyTab 保存成功后回填共享副本（含品牌名/简称/标志）。 */
  const refresh = async (): Promise<void> => {
    loaded = false;
    loadingPromise = null;
    await doLoad();
  };

  return { company, load, refresh };
}
