/**
 * 多角色接力地基自证
 *
 * 功能：用一条三棒接力链证明接力地基可用，不做任何具体业务流程。
 * 一，两个以上不同角色各自真实登录成功，且身份互异、每个身份都等于该角色码的凭证账号。
 * 二，第一棒建一个最小业务对象并把它的 id 与名称交给下一棒，第二棒按 id 直读与列表筛选都能看到这一行，
 * 且在功能页表格里真的渲染出来。
 * 三，第三种角色对同一批端点得到权限门拒绝（403 加权限机器码），侧栏里没有该入口，
 * 功能页被路由守卫挡在无权限页且不泄露业务数据行。
 * 调用方：Playwright E2E 套件，由 CI 的 e2e flow 分片按文件哈希收集执行；
 * 本文件同时是后续多角色业务流程接力用例的写法样板（接力夹具用法与判据口径）。
 * 入参：BASE_URL 与 API_BASE 取环境变量；角色凭证取 env 优先、回退 e2e/.auth/role-credentials.json；
 * 不依赖 ensureTestEntities 的共享实体，造数只有一条自建客户。
 * 传给谁：棒次会话与身份钉由 fixtures/relay 建与查，端点拒绝归因走 flow/helpers 的 expectDenied，
 * 列表信封形状判定走 flow/ui-helpers 的 pickListArray，不另立第二套判据。
 * 存什么：只在数据库里留一条自建客户行，用例结束由接力收尾登记删除。
 * 存哪里：客户行落业务表，删除登记进各棒的延迟清理队列并由接力句柄在 finally 里 flush；
 * 会话 cookie 只存在于各棒 BrowserContext 内，随 context 关闭消失，不落会话态文件。
 *
 * 为什么整条链写成单个 test 且放在 serial 描述块里：CI 分片按文件哈希分配，
 * 接力棒次不能跨文件；跨 test 会把接力句柄与会话拆到不同 worker，棒间传标识随之失真。
 *
 * 三个角色码的权限面来源（不是推测）：
 * sales_rep 由后端初始化角色种子授予 customers:read 与 customers:create，行级范围为 self；
 * purchase_clerk 在同一用例里被证明越权读客户列表 403 且客户页被守卫挡住；
 * admin 由初始化与角色种子证明持有通配权限与全部数据范围，能读到他人建的行。
 * 侧栏菜单项的可见性判据取路由元权限，与守卫同源，因此菜单差异与端点差异是同一张权限表的两个投影。
 *
 * 建单棒为什么用 self 档而不选部门级角色（如 sales_manager）：建单棒要按 id 回读自建行，
 * 而部门级读门只认资源部门，e2e 角色账号由 global-setup 建号时不带部门、既有数据范围
 * 可达性用例又明文禁止给角色账号补部门，资源部门为空即 fail-closed 403；换用 self 档角色后
 * 这条腿判的是接力本身，不再取决于夹具是否给账号分了部门。
 */
import { test, expect } from '../diagnose-fixture';
import { apiCallRaw, genCode, getRoleCredential } from './helpers';
import { pickListArray, safeGoto } from './ui-helpers';
import {
  assertUiIdentity,
  createRelay,
  expectEndpointDenied,
  expectMenuItemAbsent,
  expectMenuItemPresent,
  expectPageGuardDenied,
  expectPageRendersRow,
} from '../fixtures/relay';

type Row = Record<string, unknown>;

/**
 * 客户管理入口的中文文案，来源 src/locales/zh-CN.ts 的 layout.menu.customer。
 * 套件把浏览器语言固定为 zh-CN（playwright 配置的 locale），因此菜单文本可按该文案判定；
 * 这是既有入口回归用例的同款选法，不新增文案键、不引入 testid。
 */
const CUSTOMER_MENU_LABEL = '客户管理';

/** 列表筛选用的取页尺寸：只看第一页，配合唯一名判定，不做多页归并 */
const LIST_PAGE_QUERY = '?page=1&page_size=200';

test.describe.serial('角色接力地基：建单棒、可见棒、越权棒', () => {
  test('三棒接力在同一用例内完成，身份互异且每棒产出可追', async ({ browser }) => {
    const relay = await createRelay(browser);
    try {
      await relay.runSteps([
        {
          role: 'sales_rep',
          label: '建一条客户',
          run: async ({ session }) => {
            const uniqueName = genCode('E2E-RELAY-CUS');
            const created = await apiCallRaw<Row | null>(session.page, 'POST', '/crm/customers', {
              customer_name: uniqueName,
              contact_phone: '13800000000',
            });
            const createdId = Number(created?.id);
            if (!Number.isFinite(createdId) || createdId <= 0) {
              throw new Error(
                `第一棒建客户未返回有效 id，后续棒次的可见性断言将失去对象，实际出参片段=` +
                  `${JSON.stringify(created)?.slice(0, 200)}`
              );
            }
            // 自建自读回钉一行真落库：只信创建接口的返回值会把"接口给了 id 但行没落"读成成功
            const back = await apiCallRaw<Row>(session.page, 'GET', `/crm/customers/${createdId}`);
            expect(
              String(back?.customer_name ?? ''),
              `第一棒按 id 回读的客户名应等于自建的唯一名 ${uniqueName}`
            ).toBe(uniqueName);

            return {
              artifacts: [
                { key: 'relayCustomerId', value: createdId },
                { key: 'relayCustomerName', value: uniqueName },
              ],
              cleanup: [
                {
                  method: 'DELETE' as const,
                  path: `/crm/customers/${createdId}`,
                  label: `接力客户 ${uniqueName}`,
                },
              ],
            };
          },
        },
        {
          role: 'admin',
          label: '看见上一棒那一单',
          run: async ({ session, require }) => {
            const customerId = Number(require('relayCustomerId'));
            const customerName = String(require('relayCustomerName'));

            // 端点面一：按上一棒给的 id 直读，读到的就是那一行
            const detail = await apiCallRaw<Row>(
              session.page,
              'GET',
              `/crm/customers/${customerId}`
            );
            expect(
              String(detail?.customer_name ?? ''),
              `第二棒按 id ${customerId} 直读到的客户名应等于第一棒产出 ${customerName}`
            ).toBe(customerName);

            // 端点面二：客户列表里能筛出唯一名，且只有一行（同名噪声与重复行都不算看见）
            const list = await apiCallRaw<unknown>(
              session.page,
              'GET',
              `/crm/customers${LIST_PAGE_QUERY}`
            );
            const rows = pickListArray<Row>(list, 'items', '第二棒客户列表');
            const hits = rows.filter(row => row.customer_name === customerName);
            expect(
              hits.length,
              `第二棒的客户列表应恰好筛出唯一名 ${customerName} 一行，实际 ${hits.length} 行，` +
                `本页共 ${rows.length} 行`
            ).toBe(1);

            // UI 面：外壳头部认到的是第二棒自己的账号，侧栏有该入口，功能页表格渲出这一行
            await safeGoto(session.page, '/dashboard');
            await assertUiIdentity(session.page, session.username, '第二棒');
            await expectMenuItemPresent(session.page, CUSTOMER_MENU_LABEL, '第二棒');
            await expectPageRendersRow(session.page, '/customer', customerName, '第二棒客户功能页');

            return { artifacts: [{ key: 'relaySeenCustomerName', value: customerName }] };
          },
        },
        {
          role: 'purchase_clerk',
          label: '同一端点被权限门拒绝',
          run: async ({ session, require }) => {
            const customerId = Number(require('relayCustomerId'));

            // 端点面：列表与详情两口径都必须被权限门拒；只钉列表会把"详情门缺失"读成地基可用
            await expectEndpointDenied(
              session.page,
              'GET',
              `/crm/customers${LIST_PAGE_QUERY}`,
              '第三棒越权读客户列表'
            );
            await expectEndpointDenied(
              session.page,
              'GET',
              `/crm/customers/${customerId}`,
              '第三棒越权读客户详情'
            );

            // UI 面：头部身份是第三棒自己；侧栏没有该入口；功能页被守卫挡下且无业务行
            await safeGoto(session.page, '/dashboard');
            await assertUiIdentity(session.page, session.username, '第三棒');
            await expectMenuItemAbsent(session.page, CUSTOMER_MENU_LABEL, '第三棒');
            await expectPageGuardDenied(session.page, '/customer', '第三棒越权进客户功能页');

            // 负向棒同样要交出可追产出：记下被拒的端点段，缺产出说明这一步根本没发起请求
            return { artifacts: [{ key: 'relayDeniedEndpoint', value: '/crm/customers' }] };
          },
        },
      ]);

      // 链尾总钉一：三棒各自登录成功，账号名逐棒等于该角色码凭证里的账号
      const expectedUsernames = ['sales_rep', 'admin', 'purchase_clerk'].map(role => {
        const cred = getRoleCredential(role);
        if (!cred) {
          throw new Error(
            `链尾对账取凭证失败：角色码 ${role} 在 env 与凭证文件两处都不存在，` +
              '无法判定接力是否真的用了三个身份'
          );
        }
        return cred.username;
      });
      const actualUsernames = relay.sessions.map(session => session.username);
      expect(
        actualUsernames,
        `三棒登录账号应与角色凭证账号逐一相符，期望 ${expectedUsernames.join(', ')}，` +
          `实际 ${actualUsernames.join(', ')}`
      ).toEqual(expectedUsernames);

      // 链尾总钉二：三个身份互不相同（同账号自问自答的地基假绿由接力夹具先拦，这里再对外层钉一次）
      expect(relay.sessions.length, '接力应真实开启三棒会话').toBe(3);
      const userIds = relay.sessions.map(session => session.userId);
      expect(new Set(userIds).size, `三棒必须是三个不同用户 id，实际=${userIds.join(', ')}`).toBe(
        3
      );

      // 链尾总钉三：跨棒传递的业务标识确实登记在链上
      expect(
        Number(relay.artifacts.relayCustomerId),
        '链上应登记第一棒产出的客户 id 供后续棒次读取'
      ).toBeGreaterThan(0);
      expect(
        String(relay.artifacts.relayCustomerName),
        '链上应登记第一棒产出的客户唯一名，且与第二棒读到的一致'
      ).toBe(String(relay.artifacts.relaySeenCustomerName));
    } finally {
      await relay.close();
    }
  });
});
