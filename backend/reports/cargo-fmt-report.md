# Rust 格式检查报告

**生成时间**: 2026-08-16T05:02:47Z  
**首次检查退出码**: 1  
**模式**: 自动修正（失败时执行 cargo fmt --all 并提交）

## ✅ 已自动修正（cargo fmt --all）

### 修复前 Diff 内容（前 100 行）

```diff
info: syncing channel updates for 1.94-x86_64-unknown-linux-gnu
info: latest update on 2026-03-26 for version 1.94.1 (e408947bf 2026-03-25)
info: downloading 5 components
Diff in /home/runner/work/1/1/backend/tests/handlers_auth_handler_test.rs:112:
         .expect("LoginResponse 应序列化为 JSON 对象");
 
     let actual_fields: std::collections::HashSet<&String> = obj.keys().collect();
-    let expected_fields: std::collections::HashSet<&str> = [
-        "csrf_token",
-        "user",
-        "permissions",
-        "password_expired",
-    ]
-    .into_iter()
-    .collect();
+    let expected_fields: std::collections::HashSet<&str> =
+        ["csrf_token", "user", "permissions", "password_expired"]
+            .into_iter()
+            .collect();
 
     let extra: Vec<&&String> = actual_fields
         .iter()
```

*自动修正已提交到本分支，CI 将基于修正后代码继续。*
