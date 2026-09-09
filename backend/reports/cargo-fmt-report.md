# Rust 格式检查报告

**生成时间**: 2026-09-09T19:39:06Z  
**首次检查退出码**: 1  
**模式**: 自动修正（失败时执行 cargo fmt --all 并提交）

## ✅ 已自动修正（cargo fmt --all）

### 修复前 Diff 内容（前 100 行）

```diff
Diff in /home/runner/work/1/1/backend/tests/handlers_system_update_authz_test.rs:119:
 /// 场景 2：非 admin（role_id=999，mock fail-closed）调用 download_and_update → 403
 #[tokio::test]
 async fn test_system_update_non_admin_403_download() {
-    let app = build_app(
-        AppState::default(),
-        make_auth(2, "e2e_cashier", Some(999)),
-    );
+    let app = build_app(AppState::default(), make_auth(2, "e2e_cashier", Some(999)));
     let resp = app
         .oneshot(
             Request::builder()
Diff in /home/runner/work/1/1/backend/tests/handlers_system_update_authz_test.rs:151:
 /// （axum extractor 顺序执行：auth 在 Json 前，auth 403 先于 body 解析；带合法 JSON 确保 422 不先触发）
 #[tokio::test]
 async fn test_system_update_non_admin_403_rollback() {
-    let app = build_app(
-        AppState::default(),
-        make_auth(2, "e2e_cashier", Some(999)),
-    );
+    let app = build_app(AppState::default(), make_auth(2, "e2e_cashier", Some(999)));
     let resp = app
         .oneshot(
             Request::builder()
Diff in /home/runner/work/1/1/backend/tests/handlers_system_update_authz_test.rs:199:
 /// 场景 5：只读端点 current-version 无权限校验，登录态可达 → 200
 #[tokio::test]
 async fn test_system_update_version_readonly_200() {
-    let app = build_app(
-        AppState::default(),
-        make_auth(1, "e2e_admin", Some(1)),
-    );
+    let app = build_app(AppState::default(), make_auth(1, "e2e_admin", Some(1)));
     let resp = app
         .oneshot(
             Request::builder()
```

*自动修正已提交到本分支，CI 将基于修正后代码继续。*
