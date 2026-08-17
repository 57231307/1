# Rust 格式检查报告

**生成时间**: 2026-08-17T09:26:56Z  
**首次检查退出码**: 1  
**模式**: 自动修正（失败时执行 cargo fmt --all 并提交）

## ✅ 已自动修正（cargo fmt --all）

### 修复前 Diff 内容（前 100 行）

```diff
info: syncing channel updates for 1.94-x86_64-unknown-linux-gnu
info: latest update on 2026-03-26 for version 1.94.1 (e408947bf 2026-03-25)
info: downloading 5 components
Diff in /home/runner/work/1/1/backend/tests/services_quotation_service_test.rs:297:
     assert!(result.is_err());
     let err = result.unwrap_err();
     let msg = format!("{}", err);
-    assert!(msg.contains("报价单不存在") || msg.contains("not found") || msg.contains("不存在") || msg.contains("未找到"));
+    assert!(
+        msg.contains("报价单不存在")
+            || msg.contains("not found")
+            || msg.contains("不存在")
+            || msg.contains("未找到")
+    );
 }
 
 // ============ update 状态机校验测试 ============
```

*自动修正已提交到本分支，CI 将基于修正后代码继续。*
