/**
 * 角色凭证 setup 独立入口（CI role-permission-matrix job 专用）
 *
 * `npx playwright test --list` 不触发 globalSetup（list 模式仅 load tests），
 * 因此矩阵 job 需要显式调用 ensureRoleUsers 生成 role-credentials.json。
 * 用 esbuild 转译后 node 执行（与项目构建链一致的转译方式，免加 tsx 依赖）。
 */
import { ensureRoleUsers } from './global-setup';

ensureRoleUsers()
  .then(() => {
    console.log('[ensure-role-users] 完成');
    process.exit(0);
  })
  .catch((err: unknown) => {
    console.error('[ensure-role-users] 失败:', err);
    process.exit(1);
  });
