/**
 * 遍历列表 API 探针——42a/42b/42c/42d 四 spec 唯一实现（原四处各自复刻同一逻辑）。
 *
 * 根因（此前三重假绿，全部在本文件闭环）：
 * 1. 判据过宽：旧实现断 `resp.status() < 500`。探针跑在 admin 全权上下文
 *    （loginViaUI 无参 ⇒ TEST_USERNAME=e2e_admin，flow/helpers.ts:32-33、
 *    e2e/global-setup.ts:11-15），admin 下 401/403/404/429/400 全部 <500，
 *    一条写错的 listApi 无论落 403 还是 404 都判绿——14 条路径漂移得以长期存活。
 * 2. 覆盖面/死配置：旧条件 `mod.tier === 'A' &&` 使挂在 Tier B 模块上的
 *    listApi 根本不发请求（配置存在但零断言）。现改为**凡配置必真探**。
 *    依据 route-snapshot.txt 逐条核对：配置中全部 listApi（含原 Tier B/C 档位）
 *    都有已注册的集合 GET，故无需删除任何字段（不存在"无集合契约"档位）。
 * 3. 逻辑复刻漂移：四个 spec 各抄一份判据，改一处漏三处。现单一来源。
 *
 * 判据口径：复用 flow/helpers.ts:3029 verifyEndpointHealthy（strict）——
 * 2xx 健康；404=端点未注册/路由漂移判红；403 在 admin 上下文判红（不传
 * allowForbidden）；其它 4xx=请求契约破坏判红；5xx 判红。该 helper 自带
 * CSRF 头与信封解析，失败消息含 status 与后端 message，无需另起裸
 * page.request.get + resp.ok() 形态。禁止以任何 <500 / "非403且非404"
 * 中间档放宽（helpers.ts:3026-3027 明文"禁止以任何宽松探测把 404/403
 * 伪装成健康"）。
 */
import type { Page } from '@playwright/test';
import { verifyEndpointHealthy } from '../flow/helpers';
import type { TraversalModule } from './modules.config';

export async function probeListApiIfConfigured(page: Page, mod: TraversalModule): Promise<void> {
  // 未配置 listApi = 该模块不在本探针职责内（其覆盖由显式 spec 承担），
  // 这是配置语义的分支，不是对失败结果的兜底。
  if (!mod.listApi) return;
  // fail-closed 归一：路径必须写成 /api/v1/erp 之后的绝对路径（helpers API_PREFIX 直拼）；
  // 不合规直接点名，而非静默补 '/'。
  if (!mod.listApi.startsWith('/')) {
    throw new Error(
      `[${mod.id}] listApi=${mod.listApi} 未以 '/' 开头，不是 /api/v1/erp 之后的合法绝对路径（route-snapshot 口径）`
    );
  }
  // 分页键 page/page_size 与后端 Query 同名（utils/response.rs 分页契约）；
  // verifyEndpointHealthy 的 apiCall 直接拼接 path，query 串随行。
  await verifyEndpointHealthy(page, `${mod.listApi}?page=1&page_size=1`);
}
