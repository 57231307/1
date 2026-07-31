/**
 * i18n 国际化测试 mock 数据夹具（V15 P2 B06-P2-4 修复）
 * 规则 6：测试 mock 数据禁止硬编码在测试用例中，统一抽取到 fixtures。
 * 使用 createXxxMock(overrides?) 工厂函数模式，便于通过 overrides 灵活定制。
 *
 * 集中管理 Login.vue 等认证页面使用的 i18n 消息键值，
 * 供需要验证 i18n 集成或覆盖全局 i18n 配置的测试使用。
 */

/** 登录页 i18n 消息 mock（对齐 Login.vue 使用的 i18n 键） */
export function createLoginI18nMessagesMock(
  overrides: Record<string, string> = {}
): Record<string, string> {
  return {
    'login.subtitle': '秉羲 ERP 系统',
    'login.lockedAlert': '账号已锁定，请 {minutes} 分钟后重试',
    'login.failedAttempts': '已失败 {count} 次',
    'login.remainingTime': '剩余 {minutes} 分 {seconds} 秒',
    'login.formLabel': '登录表单',
    'login.username': '用户名',
    'login.password': '密码',
    'login.agreeTo': '我已阅读并同意',
    'login.userAgreement': '《用户协议》',
    'login.and': '和',
    'login.privacyPolicy': '《隐私政策》',
    'login.submit': '登录',
    'login.passwordExpiredTitle': '密码已过期',
    'login.passwordExpiredDesc': '您的密码已超过 90 天未修改，请修改密码',
    ...overrides,
  };
}

/** 通用 i18n 消息 mock（含登录页 + 通用操作消息） */
export function createI18nMessagesMock(
  overrides: Record<string, string> = {}
): Record<string, string> {
  return {
    ...createLoginI18nMessagesMock(),
    'common.confirm': '确认',
    'common.cancel': '取消',
    'common.save': '保存',
    'common.delete': '删除',
    'common.success': '操作成功',
    'common.error': '操作失败',
    ...overrides,
  };
}
