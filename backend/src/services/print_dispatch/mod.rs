//! 打印派发编排：目标解析、状态机流转、临时文件与传输调用
//!
//! 对外入口 [`dispatch_task`] 由打印受理链路（构建文档字节后）调用，把一个已落库为
//! `pending` 的 `print_tasks` 行驱动到终态：`succeeded` 或 `failed`。派发目标从
//! `print_targets` 表按 `print_tasks.target_name` 解析（含打印机池轮换），不在代码里
//! 出现任何设备地址字面量。
//!
//! 状态机采用与邮件队列 Worker 相同的乐观锁推进：每一次跃迁都是
//! `UPDATE ... WHERE id = ? AND status = ?`，仅当当前状态恰为期望的前置态才生效，
//! 因此并发实例不会把同一任务重复推进；前置态不满足即让位（记 WARN，不静默）。
//! 跃迁次序：`pending → queued → dispatched → succeeded | failed`。
//!
//! 失败可见性硬约束：`print_tasks` 表上有 `chk_print_tasks_failed_has_error`——
//! 状态为 `failed` 时 `error_message` 必非空。因此**任何**通往 failed 的路径（目标缺失/禁用、
//! 不支持的后端、连接字段缺失、临时文件写入失败、传输错误、派发超时、传输 panic）都
//! 显式携带一条非空 `error_message`，不存在让任务静默停留 in-flight 的分支。

pub mod transport;
pub mod transports;

use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use tracing::{info, warn};

use crate::config::settings::PrintConfig;
use crate::models::print_target::Column as PrintTargetColumn;
use crate::models::print_target::{
    self, ActiveModel as PrintTargetActiveModel, Entity as PrintTarget,
};
use crate::models::print_task::{
    ActiveModel as PrintTaskActiveModel, Column as PrintTaskColumn, Entity as PrintTask,
};
use crate::models::status::print::print_task_status as status;
use crate::services::print_dispatch::transport::{
    DispatchError, PrintJob, PrintTransport, ResolvedTarget, build_transport,
    resolve_target_from_row,
};
use crate::utils::error::AppError;

/// 把当前 UTC 时刻转成列类型 `DateTimeWithTimeZone`（`DateTime<FixedOffset>`）。
fn now() -> DateTime<FixedOffset> {
    Utc::now().fixed_offset()
}

/// 派发一个已落库为 pending 的打印任务，驱动其走完整状态机至终态。
///
/// 入参：
/// - `db`：目标数据库连接（与 `print_targets` / `print_tasks` 同一库）。
/// - `job_id`：`print_tasks.id`，须为已创建的 pending 行。
/// - `doc_bytes`：已由打印渲染链路生成的文档字节（写入临时文件后投递）。
/// - `config`：运行期派发参数（超时、失败保留临时文件开关、临时目录根）。
///
/// 目标解析走数据库：以 `print_tasks.target_name` 为 `target_code` 查 `print_targets`，
/// 池行再取成员；缺失/禁用即失败可见。传输由解析出的目标 `kind` 经
/// [`build_transport`] 构造，调用方无需关心后端选择。
///
/// 返回语义（调用方须知）：当任务被成功驱动到任一终态（`succeeded` 或 `failed`）后返回
/// `Ok(())`——成功与失败都以 `print_tasks` 行为事实来源（status / external_job_id /
/// error_message），调用方应回读该行判定结果，而非依赖返回值区分成败。仅当**无法把终态
/// 落库**（数据库错误）时才返回 `Err(AppError)`，使故障同样在调用栈上可见。若进入时该行
/// 已不在 `pending`（被并发实例抢先推进或已是终态），本函数不重复推进并返回 `Ok(())`。
pub async fn dispatch_task(
    db: &DatabaseConnection,
    job_id: i32,
    doc_bytes: &[u8],
    config: &PrintConfig,
) -> Result<(), AppError> {
    let task = claim_pending(db, job_id).await?;
    let task = match task {
        Some(t) => t,
        // 行不存在或未被本实例从 pending 领取（并发/已终态）：不重复推进。
        None => return Ok(()),
    };

    // 解析目标（含池轮换）。失败 → queued → failed。
    let resolved = match resolve_target(db, &task.target_name).await {
        Ok(t) => t,
        Err(err) => {
            return fail_and_settle(db, job_id, status::QUEUED, &err, config, None).await;
        }
    };

    // 构建传输（校验连接字段）。失败 → queued → failed。
    let transport = match build_transport(&resolved) {
        Ok(t) => t,
        Err(err) => {
            return fail_and_settle(db, job_id, status::QUEUED, &err, config, None).await;
        }
    };

    finalize_dispatch(
        db,
        job_id,
        task.copies,
        &task.job_code,
        doc_bytes,
        config,
        transport,
        resolved,
    )
    .await
}

/// 与 [`dispatch_task`] 共享同一状态机/临时文件/超时/panic 处理，但由调用方直接提供
/// 已构建的传输与已解析的目标（跳过 `print_targets` 解析与 [`build_transport`]）。
///
/// 这是给集成测试注入 `MockTransport` 以确定性验证编排层行为的窄接缝：生产路径经
/// [`dispatch_task`] 内部构造传输，不经此入口；测试可据此断言成功/失败/超时/panic 各
/// 终态与 `error_message` 落库、乐观锁不被重复推进，而无需真实硬件或网络。
pub async fn dispatch_task_with_transport(
    db: &DatabaseConnection,
    job_id: i32,
    doc_bytes: &[u8],
    config: &PrintConfig,
    transport: Box<dyn PrintTransport>,
    target: ResolvedTarget,
) -> Result<(), AppError> {
    let task = claim_pending(db, job_id).await?;
    let task = match task {
        Some(t) => t,
        None => return Ok(()),
    };
    finalize_dispatch(
        db,
        job_id,
        task.copies,
        &task.job_code,
        doc_bytes,
        config,
        transport,
        target,
    )
    .await
}

/// 领取一个 pending 任务并推进到 queued。
///
/// 返回 `Ok(Some(task))` 表示本实例成功从 pending 领取（行存在且已置 queued）；
/// `Ok(None)` 表示行不存在、或前置态非 pending（并发抢先/已终态），调用方应据 `AppError`
/// 与 `None` 区分“真的不存在”与“让位”。为简化调用，行不存在在此直接返回 `Err(not_found)`，
/// 让位返回 `Ok(None)`。
async fn claim_pending(
    db: &DatabaseConnection,
    job_id: i32,
) -> Result<Option<crate::models::print_task::Model>, AppError> {
    let task = PrintTask::find_by_id(job_id)
        .one(db)
        .await
        .map_err(|e| AppError::DatabaseError(format!("读取打印任务失败（id={job_id}）: {e}")))?
        .ok_or_else(|| AppError::not_found(format!("打印任务不存在（id={job_id}）")))?;

    // pending → queued：前置态不符说明已被并发推进或已是终态，让位且不重复处理。
    let advanced = advance(db, job_id, status::PENDING, status::QUEUED, |_| {}).await?;
    if !advanced {
        info!(
            job_id,
            "打印派发跳过：任务当前状态非 pending（已被并发推进或已终态），不重复派发"
        );
        return Ok(None);
    }
    Ok(Some(task))
}

/// 派发主循环的后半段：写临时文件 → queued→dispatched → 受保护地投递 → 落终态 → 清理临时文件。
///
/// 假定任务已处于 `queued`（由 [`claim_pending`] 保证）。投递包裹在 `tokio::time::timeout`
/// 与 `tokio::spawn` 之下：超时判 failed（「派发超时」），panic 被 JoinError 捕获判 failed，
/// 传输 Err 判 failed——三条失败路径均经 [`fail_and_settle`] 写入非空 `error_message`，
/// 满足 `chk_print_tasks_failed_has_error`，绝不让任务静默停留 in-flight。
async fn finalize_dispatch(
    db: &DatabaseConnection,
    job_id: i32,
    copies: i32,
    job_code: &str,
    doc_bytes: &[u8],
    config: &PrintConfig,
    transport: Box<dyn PrintTransport>,
    target: ResolvedTarget,
) -> Result<(), AppError> {
    // 写入临时文件：失败 → queued → failed。
    let temp_path = match write_temp_file(config, job_code, doc_bytes).await {
        Ok(p) => p,
        Err(err) => {
            return fail_and_settle(db, job_id, status::QUEUED, &err, config, None).await;
        }
    };

    // queued → dispatched（置 dispatched_at）。若被并发抢先 → 清理临时文件后让位。
    let dispatched = advance(db, job_id, status::QUEUED, status::DISPATCHED, |am| {
        am.dispatched_at = Set(Some(now()));
    })
    .await?;
    if !dispatched {
        warn!(
            job_id,
            "打印派发让位：queued→dispatched 未命中前置态（并发推进），清理临时文件后返回"
        );
        settle_temp(config, &temp_path, false).await;
        return Ok(());
    }

    let job = PrintJob {
        target,
        copies,
        file_path: temp_path.clone(),
        job_code: job_code.to_string(),
    };

    // 在超时与 panic 双重防护下执行投递。transport 与作业数据移入受监控任务。
    let bytes = doc_bytes.to_vec();
    let timeout_dur = Duration::from_secs(config.dispatch_timeout_secs);
    let handle = tokio::spawn(async move { transport.send(&job, &bytes).await });
    let outcome = tokio::time::timeout(timeout_dur, handle).await;

    match outcome {
        // 超时：到点未完成 → dispatched → failed，文案「派发超时」。
        Err(_elapsed) => {
            let err = DispatchError::Timeout;
            fail_and_settle(
                db,
                job_id,
                status::DISPATCHED,
                &err,
                config,
                Some(&temp_path),
            )
            .await
        }
        // 受监控任务结束（正常返回或 panic）。
        Ok(join_res) => match join_res {
            Err(je) => {
                let err = DispatchError::Panic {
                    detail: je.to_string(),
                };
                fail_and_settle(
                    db,
                    job_id,
                    status::DISPATCHED,
                    &err,
                    config,
                    Some(&temp_path),
                )
                .await
            }
            Ok(Ok(external_job_id)) => {
                // 成功 → dispatched → succeeded，落外部作业号与完成时间。
                let ok = advance(db, job_id, status::DISPATCHED, status::SUCCEEDED, |am| {
                    am.external_job_id = Set(Some(external_job_id.clone()));
                    am.completed_at = Set(Some(now()));
                })
                .await?;
                if ok {
                    info!(
                        job_id,
                        external_job_id = %external_job_id,
                        "打印派发成功，任务进入 succeeded"
                    );
                } else {
                    warn!(
                        job_id,
                        "打印结果回写让位：dispatched→succeeded 未命中前置态（并发），不回退已投递作业"
                    );
                }
                settle_temp(config, &temp_path, false).await;
                Ok(())
            }
            Ok(Err(dispatch_err)) => {
                fail_and_settle(
                    db,
                    job_id,
                    status::DISPATCHED,
                    &dispatch_err,
                    config,
                    Some(&temp_path),
                )
                .await
            }
        },
    }
}

/// 以乐观锁把任务从 `from` 推进到 `to`，`patch` 负责补充本跃迁要写的列。
///
/// 返回是否命中前置态（`rows_affected > 0`）。不吞掉数据库错误：写失败以 `AppError` 上抛。
async fn advance(
    db: &DatabaseConnection,
    job_id: i32,
    from: &str,
    to: &str,
    patch: impl FnOnce(&mut PrintTaskActiveModel),
) -> Result<bool, AppError> {
    let mut am = PrintTaskActiveModel {
        status: Set(to.to_string()),
        updated_at: Set(now()),
        ..Default::default()
    };
    patch(&mut am);

    let result = PrintTask::update_many()
        .set(am)
        .filter(PrintTaskColumn::Id.eq(job_id))
        .filter(PrintTaskColumn::Status.eq(from))
        .exec(db)
        .await
        .map_err(|e| {
            AppError::DatabaseError(format!(
                "更新打印任务状态失败（{from}→{to}, id={job_id}）: {e}"
            ))
        })?;

    Ok(result.rows_affected > 0)
}

/// 通往 failed 的统一落点：乐观锁把 `prior` → `failed` 并写非空 `error_message`，随后清理临时文件。
///
/// 失败文案取自 `err`（[`DispatchError::TargetUnresolved`] 的 Display 即逐字的
/// 「打印目标未配置或未启用: <name>」），保证 `chk_print_tasks_failed_has_error` 成立。
async fn fail_and_settle(
    db: &DatabaseConnection,
    job_id: i32,
    prior: &str,
    err: &DispatchError,
    config: &PrintConfig,
    temp_path: Option<&Path>,
) -> Result<(), AppError> {
    let message = err.to_string();
    warn!(job_id, error = %message, "打印派发失败，任务转入 failed");

    let ok = advance(db, job_id, prior, status::FAILED, |am| {
        am.error_message = Set(Some(message.clone()));
        am.completed_at = Set(Some(now()));
    })
    .await?;
    if !ok {
        warn!(
            job_id,
            prior, "转入 failed 未命中前置态：任务已被并发推进到其它状态，遵循乐观锁不重复改写"
        );
    }

    if let Some(path) = temp_path {
        // 失败：仅在 keep_failed_temp 打开时保留临时文件，否则删除。
        settle_temp(config, path, true).await;
    }
    Ok(())
}

/// 依据终态与配置处置临时文件：成功一律删除；失败按 `keep_failed_temp` 决定保留或删除。
async fn settle_temp(config: &PrintConfig, path: &Path, failed: bool) {
    let keep = failed && config.keep_failed_temp;
    if keep {
        // 保留时把路径显式记入日志（不静默），便于运维事后取回排查。
        warn!(
            path = %path.display(),
            "打印失败且开启 keep_failed_temp，保留临时文件于上述路径"
        );
        return;
    }
    match tokio::fs::remove_file(path).await {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            warn!(path = %path.display(), error = %e, "删除打印临时文件失败（不影响任务终态判定）");
        }
    }
}

/// 在配置的临时根目录下按任务编码写入文档字节，返回落盘绝对路径。
///
/// 文件名取 `<job_code>.docx`；对任务编码做白名单净化，避免任何路径分隔或异常字符
/// 进入文件系统。目录不存在时先创建。
async fn write_temp_file(
    config: &PrintConfig,
    job_code: &str,
    doc_bytes: &[u8],
) -> Result<PathBuf, DispatchError> {
    let root = temp_root(config);
    tokio::fs::create_dir_all(&root)
        .await
        .map_err(|e| DispatchError::TempIo {
            detail: format!("创建临时目录 {} 失败: {e}", root.display()),
        })?;

    let path = root.join(format!("{}.docx", sanitize_file_name(job_code)));
    tokio::fs::write(&path, doc_bytes)
        .await
        .map_err(|e| DispatchError::TempIo {
            detail: format!("写入临时文件 {} 失败: {e}", path.display()),
        })?;

    Ok(path)
}

/// 解析临时根目录：配置给定则用之，否则回退到进程系统的临时目录下的 `bingxi_print` 子目录。
fn temp_root(config: &PrintConfig) -> PathBuf {
    match config.temp_dir_root.as_deref().map(str::trim) {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => {
            let mut p = std::env::temp_dir();
            p.push("bingxi_print");
            p
        }
    }
}

/// 把任务编码净化为可用作单段文件名的安全串（仅保留 `[A-Za-z0-9._-]`，其余转 `_`）。
fn sanitize_file_name(job_code: &str) -> String {
    let mapped: String = job_code
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = mapped.trim_matches('.');
    if trimmed.is_empty() {
        "print_job".to_string()
    } else {
        trimmed.to_string()
    }
}

/// 按 `target_code` 解析派发目标：单台目标直接返回其连接上下文；打印机池取首个成员、
/// 轮换取下一个并把更新写回，再解析该成员行。目标不存在/禁用/池成员缺失一律以
/// [`DispatchError::TargetUnresolved`] 失败可见（文案「打印目标未配置或未启用: <name>」）。
async fn resolve_target(
    db: &DatabaseConnection,
    name: &str,
) -> Result<ResolvedTarget, DispatchError> {
    let row = find_enabled_target_by_code(db, name)
        .await?
        .ok_or_else(|| DispatchError::TargetUnresolved {
            name: name.to_string(),
        })?;

    if !row.is_pool {
        return resolve_target_from_row(&row);
    }

    // 池：解析成员编码，取首个作为本轮目标，并把数组左旋一格写回以实现跨派发轮询。
    let members = pool_member_codes(&row.pool_members);
    if members.is_empty() {
        return Err(DispatchError::TargetUnresolved {
            name: format!("{name}（打印机池成员为空）"),
        });
    }

    let head = members[0].clone();
    let mut rotated = members[1..].to_vec();
    rotated.push(head.clone());
    // 轮换游标写回是尽力而为的均衡状态：写库失败不影响本次已确定的成员选择，记 WARN 不静默。
    if let Err(e) = persist_pool_rotation(db, row.id, &rotated).await {
        warn!(
            target_id = row.id,
            error = %e,
            "打印机池轮换游标写回失败（本次仍按已选成员派发，均衡次序下次重算）"
        );
    }

    let member = find_enabled_target_by_code(db, &head)
        .await?
        .ok_or_else(|| DispatchError::TargetUnresolved { name: head.clone() })?;
    resolve_target_from_row(&member)
}

/// 读取 `pool_members` 里的字符串成员编码（非字符串或越界元素按缺失对待，不伪造成员）。
fn pool_member_codes(pool_members: &serde_json::Value) -> Vec<String> {
    pool_members
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// 按 `target_code` 取「存在且启用」的目标行；不存在、或存在但禁用，返回 `None`
/// （由调用方转成「打印目标未配置或未启用」失败）。数据库错误以 [`DispatchError`] 上抛，
/// 与目标解析同错误域，供编排层统一转 failed 落 error_message。
async fn find_enabled_target_by_code(
    db: &DatabaseConnection,
    code: &str,
) -> Result<Option<print_target::Model>, DispatchError> {
    let row = PrintTarget::find()
        .filter(PrintTargetColumn::TargetCode.eq(code))
        .one(db)
        .await
        .map_err(|e| DispatchError::Transport {
            detail: format!("查询打印目标失败（target_code={code}）: {e}"),
        })?;
    Ok(row.filter(|r| r.enabled))
}

/// 把轮换后的成员数组写回目标行的 `pool_members`。
async fn persist_pool_rotation(
    db: &DatabaseConnection,
    target_id: i32,
    rotated: &[String],
) -> Result<(), DispatchError> {
    let am = PrintTargetActiveModel {
        id: Set(target_id),
        pool_members: Set(serde_json::Value::Array(
            rotated
                .iter()
                .map(|s| serde_json::Value::String(s.clone()))
                .collect(),
        )),
        updated_at: Set(now()),
        ..Default::default()
    };
    am.update(db).await.map_err(|e| DispatchError::Transport {
        detail: format!("写回打印机池轮换次序失败（id={target_id}）: {e}"),
    })?;
    Ok(())
}
