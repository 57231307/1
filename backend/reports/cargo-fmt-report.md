# Rust 格式检查报告

**生成时间**: 2026-09-09T18:27:08Z  
**首次检查退出码**: 1  
**模式**: 自动修正（失败时执行 cargo fmt --all 并提交）

## ✅ 已自动修正（cargo fmt --all）

### 修复前 Diff 内容（前 100 行）

```diff
Diff in /home/runner/work/1/1/backend/src/cli/util/upgrade.rs:1242:
             check_version_downgrade("2026.9.10.1000", "unknown"),
             "目标版本非标格式应 fail-open 放行"
         );
-        assert!(
-            check_version_downgrade("", ""),
-            "空版本号应 fail-open 放行"
-        );
+        assert!(check_version_downgrade("", ""), "空版本号应 fail-open 放行");
     }
 
     #[test]
Diff in /home/runner/work/1/1/backend/src/handlers/audit_enhanced_handler.rs:135:
     Query(_query): Query<AuditLogQuery>,
 ) -> Result<Json<ApiResponse<ExportResult>>, AppError> {
     // 敏感导出 fail-closed：校验审批令牌
-    let approval = crate::services::export_approval_service::ExportApprovalService::new(
-        state.db.clone(),
-    )
-    .enforce_export_download(_query.download_token.as_deref(), "audit_log")
-    .await?;
+    let approval =
+        crate::services::export_approval_service::ExportApprovalService::new(state.db.clone())
+            .enforce_export_download(_query.download_token.as_deref(), "audit_log")
+            .await?;
 
     use sea_orm::{EntityTrait, QueryOrder, QuerySelect};
 
Diff in /home/runner/work/1/1/backend/src/handlers/audit_enhanced_handler.rs:180:
     svc.record_async(event, None);
 
     // 敏感导出 fail-closed：记录令牌消费
-    let _ = crate::services::export_approval_service::ExportApprovalService::new(
-        state.db.clone(),
-    )
-    .record_download(approval.id, file_name.clone(), count as i64, String::new())
-    .await;
+    let _ = crate::services::export_approval_service::ExportApprovalService::new(state.db.clone())
+        .record_download(approval.id, file_name.clone(), count as i64, String::new())
+        .await;
 
     Ok(Json(ApiResponse::success(ExportResult {
         download_url: format!("/api/v1/erp/downloads/{}", file_name),
Diff in /home/runner/work/1/1/backend/src/handlers/audit_log_handler.rs:364:
     require_admin_role(&state, &auth).await?;
 
     // 敏感导出 fail-closed：校验审批令牌
-    let approval = crate::services::export_approval_service::ExportApprovalService::new(
-        state.db.clone(),
-    )
-    .enforce_export_download(query.download_token.as_deref(), "audit_log")
-    .await?;
+    let approval =
+        crate::services::export_approval_service::ExportApprovalService::new(state.db.clone())
+            .enforce_export_download(query.download_token.as_deref(), "audit_log")
+            .await?;
 
     const EXPORT_LIMIT: u64 = 10000;
     let cond = build_audit_log_condition(&query);
Diff in /home/runner/work/1/1/backend/src/handlers/audit_log_handler.rs:410:
     .await;
 
     // 敏感导出 fail-closed：记录令牌消费（用真实 file_hash + file_size）
-    let _ = crate::services::export_approval_service::ExportApprovalService::new(
-        state.db.clone(),
-    )
-    .record_download(approval.id, filename.clone(), file_size, file_hash)
-    .await;
+    let _ = crate::services::export_approval_service::ExportApprovalService::new(state.db.clone())
+        .record_download(approval.id, filename.clone(), file_size, file_hash)
+        .await;
 
     // 规则 3：导出统一使用 xlsx 格式，错误用 AppError 表达，成功返回 200 + xlsx 响应体
     Ok(xlsx_response(xlsx_bytes, &filename))
Diff in /home/runner/work/1/1/backend/src/handlers/custom_order_handler.rs:274:
     })
     .await
     .map_err(|_| {
-        AppError::InternalError("查询定制订单详情超时（数据库连接池 acquire 死锁防御触发）".to_string())
+        AppError::InternalError(
+            "查询定制订单详情超时（数据库连接池 acquire 死锁防御触发）".to_string(),
+        )
     })??;
 
     Ok(Json(ApiResponse::success(detail)))
Diff in /home/runner/work/1/1/backend/src/handlers/custom_order_handler.rs:281:
-
 }
 
 /// 转换流程节点列表为响应 DTO
Diff in /home/runner/work/1/1/backend/src/handlers/customer_handler.rs:561:
     auth: AuthContext,
 ) -> Result<axum::response::Response, AppError> {
     // 敏感导出 fail-closed：校验审批令牌
-    let approval = crate::services::export_approval_service::ExportApprovalService::new(
-        state.db.clone(),
-    )
-    .enforce_export_download(query.download_token.as_deref(), "customer")
-    .await?;
+    let approval =
```

*自动修正已提交到本分支，CI 将基于修正后代码继续。*
