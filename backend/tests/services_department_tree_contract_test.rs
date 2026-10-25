//! 功能：部门树与部门列表出参契约锁，钉住五件事——父级不可达的部门必须回到树出参里（不再
//! 静默丢弃）、成环不得造成节点重复或丢失、树节点出参必须带出 code 与 sort_order 与 is_active
//! 三列、树出参的根级与层内先后必须构成确定全序、列表分页排序必须构成全序使同名部门跨页
//! 不重复取也不漏取。
//! 调用方：集成测试层（真已迁移 PostgreSQL，TEST_DATABASE_URL；本文件不调 Migrator，
//! 不依赖迁移台账内容）。
//! 入参：test_common 的 setup_test_db 连接并清空业务表后，自建一段自有主键的部门行。
//! departments 属迁移种子参照表、不参与清空，同一 job 内上一轮跑过的行仍在，故播种前先按
//! 自有主键删除再重新写入，保证用例可重复执行。
//! 传给谁：DepartmentService 的 get_department_tree、经 Router 真装配并注入 AuthContext 的
//! department_handler 的 get_department_tree、以及 DepartmentService 的 list。
//! 存什么：树出参的节点集合与每个节点的列值、告警日志文本、分页逐页的 id 序列。
//! 存哪里：部门行存 departments 表；断言只读出参与日志，不写新表。
//!
//! 判据（逐条，判红才算锁有效）：
//! 一、父级部门行不存在的部门以顶层节点出现，parent_id 保持数据库原值不被改写，其子节点
//! 随它一起带出且整棵树里该 id 只出现一次；告警按"悬挂外键"类别点名该 id 与缺失的父级主键。
//! 二、自引用部门以顶层节点出现且只出现一次，不会因递归展开而自己套自己；双节点成环、
//! 三节点成环的每个成员以及挂在环下的子节点，在整棵出参里各出现一次，且每个环只有一个
//! 顶层入口；告警把"自引用成环""父级指向环内其他部门""父级同样从根不可达"分成三类如实描述。
//! 三、树节点出参含 code 与 sort_order 与 is_active 三键，值与落库值一致（含 is_active 落成为
//! 假的那个部门，证明不是写死真值），键集合精确等于九键且恒存在，叶子节点的 children 恒为
//! 空数组而不是缺席；端点出参载荷仍是裸数组，没有多出分页信封。
//! 四、两条同名部门的分页按名称升序再以主键升序破平：逐页翻完得到的 id 序列等于全部同名
//! id 的升序序列，既不重复也不遗漏。
//! 五、树出参的根级与每一层 children 都按 sort_order 升序、主键升序排列：顺序夹具把主键先后
//! 与期望先后做成相反，两条并列排序号靠主键破平，并连取两次比对序列一致，用来拦"靠 HashMap
//! 迭代序碰巧稳定"这种非确定性顺序（前端一次取树直渲，顺序跳变即用户可见缺陷）。
//!
//! 悬挂外键这一形态在真库里怎么来的：departments.parent_id 上有自引用外键约束，正常写入
//! 路径插不进"父级行不存在"的行，只有遗留库或导入链绕过约束才会出现，因此播种该形态时在
//! 单个事务内以 SET LOCAL session_replication_role 临时关闭触发器。SET LOCAL 只作用于本事务
//! 本连接，不污染同库其他用例；该设置需要超级用户权限，CI 的 postgres service 以 initdb 首个
//! 用户启动即为超级用户，权限不足时播种直接 panic 点名，不会静默跳过判据。
//! 自引用与成环不需要任何绕过：自引用可单行写入，成环可由部门更新端点（只校验父级存在、
//! 不校验是否为后代）两段写入，属应用自身可达的数据形态。

mod test_common;

use std::io::Write;
use std::sync::{Arc, Mutex};

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::get,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    QueryFilter, Set, Statement, TransactionTrait,
};
use serde_json::Value;
use tower::ServiceExt;
use tracing::Level;
use tracing::subscriber::{DefaultGuard, set_default};
use tracing_subscriber::fmt;

use bingxi_backend::container::AppState;
use bingxi_backend::handlers::department_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::department;
use bingxi_backend::services::department_service::{DepartmentService, DepartmentTreeNode};
use chrono::Utc;

// 自有主键段 9730 至 9752，避开迁移种子部门（id 1 至 5）与其他集成测试占用的部门 id，
// 也避开 departments 自增序列在当前 job 内可能走到的取值。
const DEPT_NORMAL_PARENT: i32 = 9730;
const DEPT_NORMAL_CHILD: i32 = 9731;
const DEPT_DANGLING_CHILD: i32 = 9732;
const DEPT_DANGLING_GRANDCHILD: i32 = 9733;
const DEPT_SELF_REF: i32 = 9734;
const DEPT_MUTUAL_A: i32 = 9735;
const DEPT_MUTUAL_B: i32 = 9736;
const DEPT_RING_A: i32 = 9737;
const DEPT_RING_B: i32 = 9738;
const DEPT_RING_C: i32 = 9739;
const DEPT_RING_LEAF: i32 = 9740;
/// 从不插入、只被 DEPT_DANGLING_CHILD 引用的父级主键，用来构成"父级部门行不存在"
const GHOST_PARENT_ID: i32 = 9741;
/// 同名一对：主键大的那条先插，使物理先后与主键先后相反。
/// 只按名称排序时同名列的先后由物理顺序决定，于是页序会给出主键降序；
/// 补上主键次级键后页序必须是主键升序，缺次级键就体现在这条判据上。
const DEPT_SAMEDUP_ROW_FIRST: i32 = 9743;
const DEPT_SAMEDUP_ROW_SECOND: i32 = 9742;

/// 顺序夹具四条根：主键先后与期望出参先后刻意相反，并列排序号的两条靠主键升序破平。
/// 只按主键或物理顺序排（HashMap 迭代序）都会给出别的序列，判据五才抓得住。
const DEPT_ORDER_ROOT_TAIL: i32 = 9744;
const DEPT_ORDER_ROOT_HEAD: i32 = 9745;
const DEPT_ORDER_ROOT_TIE_LOW: i32 = 9746;
const DEPT_ORDER_ROOT_TIE_HIGH: i32 = 9747;
/// 顺序夹具的父节点与它的四条子节点：同一套"主键先后与期望先后相反"的构造用在层内
const DEPT_ORDER_PARENT: i32 = 9748;
const DEPT_ORDER_CHILD_TAIL: i32 = 9749;
const DEPT_ORDER_CHILD_HEAD: i32 = 9750;
const DEPT_ORDER_CHILD_TIE_LOW: i32 = 9751;
const DEPT_ORDER_CHILD_TIE_HIGH: i32 = 9752;

/// 同名一对共用的部门名称：全库唯一，作为列表 search 关键词时只会命中这两行
const SAMEDUP_NAME: &str = "部门树分页同序锁定";

/// 每个用例都要落库的自有主键集合（按主键删除重建的范围）
fn fixture_ids() -> Vec<i32> {
    vec![
        DEPT_NORMAL_PARENT,
        DEPT_NORMAL_CHILD,
        DEPT_DANGLING_CHILD,
        DEPT_DANGLING_GRANDCHILD,
        DEPT_SELF_REF,
        DEPT_MUTUAL_A,
        DEPT_MUTUAL_B,
        DEPT_RING_A,
        DEPT_RING_B,
        DEPT_RING_C,
        DEPT_RING_LEAF,
        DEPT_SAMEDUP_ROW_FIRST,
        DEPT_SAMEDUP_ROW_SECOND,
        DEPT_ORDER_ROOT_TAIL,
        DEPT_ORDER_ROOT_HEAD,
        DEPT_ORDER_ROOT_TIE_LOW,
        DEPT_ORDER_ROOT_TIE_HIGH,
        DEPT_ORDER_PARENT,
        DEPT_ORDER_CHILD_TAIL,
        DEPT_ORDER_CHILD_HEAD,
        DEPT_ORDER_CHILD_TIE_LOW,
        DEPT_ORDER_CHILD_TIE_HIGH,
    ]
}

/// 一条待播种的部门行：主键、名称、父级、启用状态
struct SeedRow {
    id: i32,
    name: &'static str,
    parent_id: Option<i32>,
    is_active: bool,
}

/// 播种清单：正常父子一对、悬挂外键及其子节点、自引用、双节点成环、
/// 三节点成环及环下一个子节点、同名一对（主键大的先插）。
/// code 与 sort_order 由主键派生，便于断言出参值就是落库值。
fn seed_rows() -> Vec<SeedRow> {
    Vec::from([
        SeedRow {
            id: DEPT_NORMAL_PARENT,
            name: "部门树正常父",
            parent_id: None,
            is_active: true,
        },
        SeedRow {
            id: DEPT_NORMAL_CHILD,
            name: "部门树正常子",
            parent_id: Some(DEPT_NORMAL_PARENT),
            is_active: true,
        },
        SeedRow {
            id: DEPT_DANGLING_CHILD,
            name: "部门树悬挂子",
            parent_id: Some(GHOST_PARENT_ID),
            // 显式落成假值，用来证明出参的 is_active 来自真实列而不是写死
            is_active: false,
        },
        SeedRow {
            id: DEPT_DANGLING_GRANDCHILD,
            name: "部门树悬挂孙",
            parent_id: Some(DEPT_DANGLING_CHILD),
            is_active: true,
        },
        SeedRow {
            id: DEPT_SELF_REF,
            name: "部门树自引用",
            parent_id: Some(DEPT_SELF_REF),
            is_active: true,
        },
        SeedRow {
            id: DEPT_MUTUAL_A,
            name: "部门树互环甲",
            parent_id: Some(DEPT_MUTUAL_B),
            is_active: true,
        },
        SeedRow {
            id: DEPT_MUTUAL_B,
            name: "部门树互环乙",
            parent_id: Some(DEPT_MUTUAL_A),
            is_active: true,
        },
        SeedRow {
            id: DEPT_RING_A,
            name: "部门树三环甲",
            parent_id: Some(DEPT_RING_B),
            is_active: true,
        },
        SeedRow {
            id: DEPT_RING_B,
            name: "部门树三环乙",
            parent_id: Some(DEPT_RING_C),
            is_active: true,
        },
        SeedRow {
            id: DEPT_RING_C,
            name: "部门树三环丙",
            parent_id: Some(DEPT_RING_A),
            is_active: true,
        },
        SeedRow {
            id: DEPT_RING_LEAF,
            name: "部门树环下叶",
            parent_id: Some(DEPT_RING_B),
            is_active: true,
        },
        SeedRow {
            id: DEPT_SAMEDUP_ROW_FIRST,
            name: SAMEDUP_NAME,
            parent_id: None,
            is_active: true,
        },
        SeedRow {
            id: DEPT_SAMEDUP_ROW_SECOND,
            name: SAMEDUP_NAME,
            parent_id: None,
            is_active: true,
        },
        // 顺序夹具：四条根加一组父子四条，排序号见 sort_order_of，判据五据此断言先后
        SeedRow {
            id: DEPT_ORDER_ROOT_TAIL,
            name: "部门树顺序根末",
            parent_id: None,
            is_active: true,
        },
        SeedRow {
            id: DEPT_ORDER_ROOT_HEAD,
            name: "部门树顺序根首",
            parent_id: None,
            is_active: true,
        },
        SeedRow {
            id: DEPT_ORDER_ROOT_TIE_LOW,
            name: "部门树顺序根并列甲",
            parent_id: None,
            is_active: true,
        },
        SeedRow {
            id: DEPT_ORDER_ROOT_TIE_HIGH,
            name: "部门树顺序根并列乙",
            parent_id: None,
            is_active: true,
        },
        SeedRow {
            id: DEPT_ORDER_PARENT,
            name: "部门树顺序父",
            parent_id: None,
            is_active: true,
        },
        SeedRow {
            id: DEPT_ORDER_CHILD_TAIL,
            name: "部门树顺序子末",
            parent_id: Some(DEPT_ORDER_PARENT),
            is_active: true,
        },
        SeedRow {
            id: DEPT_ORDER_CHILD_HEAD,
            name: "部门树顺序子首",
            parent_id: Some(DEPT_ORDER_PARENT),
            is_active: true,
        },
        SeedRow {
            id: DEPT_ORDER_CHILD_TIE_LOW,
            name: "部门树顺序子并列甲",
            parent_id: Some(DEPT_ORDER_PARENT),
            is_active: true,
        },
        SeedRow {
            id: DEPT_ORDER_CHILD_TIE_HIGH,
            name: "部门树顺序子并列乙",
            parent_id: Some(DEPT_ORDER_PARENT),
            is_active: true,
        },
    ])
}

fn code_of(id: i32) -> String {
    format!("DTTREE{id}")
}

fn sort_order_of(id: i32) -> i32 {
    // 顺序夹具另给排序号：主键最小的那条拿最大排序号，使物理/主键先后与期望出参先后相反；
    // 成对的两条拿同一排序号，用来验并列时由主键升序破平。其余行仍由主键派生，
    // 保证判据三"出参 sort_order 就是落库 sort_order"的断言口径不变。
    match id {
        DEPT_ORDER_ROOT_TAIL => 120,
        DEPT_ORDER_ROOT_HEAD => 8,
        DEPT_ORDER_ROOT_TIE_LOW => 60,
        DEPT_ORDER_ROOT_TIE_HIGH => 60,
        DEPT_ORDER_PARENT => 4,
        DEPT_ORDER_CHILD_TAIL => 130,
        DEPT_ORDER_CHILD_HEAD => 9,
        DEPT_ORDER_CHILD_TIE_LOW => 70,
        DEPT_ORDER_CHILD_TIE_HIGH => 70,
        _ => id - 9700,
    }
}

/// 把自有主键段清空后重新写入。
/// 悬挂外键需要在本事务内临时关闭触发器（文件头说明），其余行也一并在同一事务落库，
/// 使这段异常数据对同库其他用例可见的窗口最短。
async fn seed(db: &DatabaseConnection) {
    department::Entity::delete_many()
        .filter(department::Column::Id.is_in(fixture_ids()))
        .exec(db)
        .await
        .expect("测试夹具：清理自有部门主键段失败");

    let now = Utc::now();
    let txn = db.begin().await.expect("测试夹具：开启播种事务失败");
    txn.execute_raw(Statement::from_string(
        DbBackend::Postgres,
        "SET LOCAL session_replication_role = replica".to_string(),
    ))
    .await
    .expect(
        "测试夹具：临时关闭外键触发器失败（SET LOCAL session_replication_role）——\
         连接用户不是超级用户，悬挂外键这一数据形态造不出来，本文件第一类判据失去前提",
    );

    for row in seed_rows() {
        department::ActiveModel {
            id: Set(row.id),
            name: Set(row.name.to_string()),
            code: Set(code_of(row.id)),
            parent_id: Set(row.parent_id),
            description: Set(Some(format!("部门树契约锁{}", row.id))),
            sort_order: Set(sort_order_of(row.id)),
            is_active: Set(row.is_active),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&txn)
        .await
        .unwrap_or_else(|e| panic!("测试夹具：播种部门 {} 失败: {e}", row.id));
    }

    txn.commit().await.expect("测试夹具：提交播种事务失败");
}

// ---------------------------------------------------------------------------
// 告警捕获（与 tests/contract_wave4_trace_and_envelope_test.rs 同款内存 writer）
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct SharedBuf(Arc<Mutex<Vec<u8>>>);

impl Write for SharedBuf {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut guard = self.0.lock().unwrap_or_else(|e| e.into_inner());
        guard.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct CapturedLogs {
    buf: Arc<Mutex<Vec<u8>>>,
    _guard: DefaultGuard,
}

impl CapturedLogs {
    fn text(&self) -> String {
        let bytes = self.buf.lock().unwrap_or_else(|e| e.into_inner()).clone();
        String::from_utf8_lossy(&bytes).to_string()
    }

    /// 取包含指定片段的那一行日志（取不到给空串，让后续 contains 断言如实判红）
    fn line_containing(&self, needle: &str) -> String {
        self.text()
            .lines()
            .find(|l| l.contains(needle))
            .unwrap_or("")
            .to_string()
    }
}

fn capture_logs() -> CapturedLogs {
    let buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let writer_buf = buf.clone();
    let subscriber = fmt()
        .with_ansi(false)
        .with_max_level(Level::TRACE)
        .with_writer(move || SharedBuf(writer_buf.clone()))
        .finish();
    let _guard = set_default(subscriber);
    CapturedLogs { buf, _guard }
}

// ---------------------------------------------------------------------------
// 树出参读取工具
// ---------------------------------------------------------------------------

/// 先序展开整棵出参，收集每个节点的主键（含重复，用来判同一节点只出现一次）
fn flatten_ids(nodes: &[DepartmentTreeNode], out: &mut Vec<i32>) {
    for node in nodes {
        out.push(node.id);
        flatten_ids(&node.children, out);
    }
}

fn collect_ids(nodes: &[DepartmentTreeNode]) -> Vec<i32> {
    let mut out = Vec::new();
    flatten_ids(nodes, &mut out);
    out
}

fn occurrence_count(all_ids: &[i32], id: i32) -> usize {
    all_ids.iter().filter(|x| **x == id).count()
}

fn find_node(nodes: &[DepartmentTreeNode], id: i32) -> Option<&DepartmentTreeNode> {
    for node in nodes {
        if node.id == id {
            return Some(node);
        }
        if let Some(hit) = find_node(&node.children, id) {
            return Some(hit);
        }
    }
    None
}

/// 顶层（根）节点主键集合，升序
fn top_level_ids(nodes: &[DepartmentTreeNode]) -> Vec<i32> {
    let mut ids: Vec<i32> = nodes.iter().map(|n| n.id).collect();
    ids.sort_unstable();
    ids
}

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("dept_tree_fixture_{user_id}"),
        role_id: Some(2),
        department_id: Some(DEPT_NORMAL_PARENT),
        data_scope: Some("all".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    }
}

async fn tree_of(db: &DatabaseConnection) -> Vec<DepartmentTreeNode> {
    let service = DepartmentService::new(Arc::new(db.clone()));
    service
        .get_department_tree()
        .await
        .expect("部门树服务层调用失败")
}

// ---------------------------------------------------------------------------
// 判据一：悬挂外键以根形态回到树出参，并点名告警
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dangling_parent_department_comes_back_as_root_with_warn() {
    let db = test_common::setup_test_db().await;
    seed(&db).await;
    let logs = capture_logs();
    let tree = tree_of(&db).await;
    let all_ids = collect_ids(&tree);

    let node = find_node(&tree, DEPT_DANGLING_CHILD)
        .unwrap_or_else(|| panic!("父级行不存在的部门应从根不可达回到出参: {all_ids:?}"));
    assert_eq!(
        node.parent_id,
        Some(GHOST_PARENT_ID),
        "提升为根只描述层级不可达，parent_id 必须保持数据库原值，不得改写语义"
    );
    assert!(
        top_level_ids(&tree).contains(&DEPT_DANGLING_CHILD),
        "悬挂部门应以顶层节点形态出现: 顶层={:?}",
        top_level_ids(&tree)
    );
    assert_eq!(
        occurrence_count(&all_ids, DEPT_DANGLING_CHILD),
        1,
        "同一节点在整棵出参里只能出现一次: {all_ids:?}"
    );

    let grandchild = node
        .children
        .iter()
        .find(|c| c.id == DEPT_DANGLING_GRANDCHILD)
        .unwrap_or_else(|| {
            panic!(
                "悬挂节点的子节点应随它一起带出: 子集={:?}",
                node.children.iter().map(|c| c.id).collect::<Vec<_>>()
            )
        });
    assert_eq!(
        grandchild.parent_id,
        Some(DEPT_DANGLING_CHILD),
        "悬挂下的子节点 parent_id 同样保持原值"
    );
    assert_eq!(
        occurrence_count(&all_ids, DEPT_DANGLING_GRANDCHILD),
        1,
        "子节点不得既挂在父级下、又作为顶层节点各出现一次: {all_ids:?}"
    );

    let text = logs.text();
    assert!(
        text.contains("部门树组装：") && text.contains("父级缺席或层级成环"),
        "回收必须点名告警，不得静默: 日志={text}"
    );
    let summary_line = logs.line_containing("按根节点返回（parent_id 保持原值");
    assert!(
        summary_line.contains(&format!(
            "id={DEPT_DANGLING_CHILD} 的父级 parent_id={GHOST_PARENT_ID}"
        )),
        "汇总行必须点名被提升的节点及其父级主键: 日志行={summary_line}"
    );
    assert!(
        summary_line.contains("parent_id 保持原值，不代表层级正确"),
        "告警必须如实说明这是层级不可达、不是假装层级正确: 日志行={summary_line}"
    );
    let dangling_line = logs.line_containing("悬挂外键");
    assert!(
        dangling_line.contains(&format!(
            "id={DEPT_DANGLING_CHILD} 的父部门 {GHOST_PARENT_ID} 不存在"
        )),
        "悬挂外键必须作为独立类别点名，并带出缺失的父级主键: 日志行={dangling_line}"
    );
}

// ---------------------------------------------------------------------------
// 判据二：自引用与成环不重复不丢失，并按根因分类告警
// ---------------------------------------------------------------------------

#[tokio::test]
async fn self_referencing_department_becomes_root_without_reexpanding() {
    let db = test_common::setup_test_db().await;
    seed(&db).await;
    let logs = capture_logs();
    let tree = tree_of(&db).await;
    let all_ids = collect_ids(&tree);

    let node = find_node(&tree, DEPT_SELF_REF)
        .unwrap_or_else(|| panic!("自引用部门必须回到出参而不是被丢弃: {all_ids:?}"));
    assert_eq!(
        occurrence_count(&all_ids, DEPT_SELF_REF),
        1,
        "自引用节点最多被取走一次: {all_ids:?}"
    );
    assert!(
        top_level_ids(&tree).contains(&DEPT_SELF_REF),
        "自引用部门应以顶层节点形态出现: 顶层={:?}",
        top_level_ids(&tree)
    );
    assert_eq!(
        node.parent_id,
        Some(DEPT_SELF_REF),
        "自引用的 parent_id 保持原值，不改写成顶级"
    );
    assert!(
        !node.children.iter().any(|c| c.id == DEPT_SELF_REF),
        "自引用节点不得把自己挂成自己的子节点（递归展开必须终止）"
    );

    let line = logs.line_containing("自引用成环");
    assert!(
        line.contains(&DEPT_SELF_REF.to_string()),
        "自引用必须与另外两类分开点名，并带上该节点主键: 日志行={line}"
    );
    assert!(
        line.contains("1 个节点"),
        "点名数量必须与实际自引用节点数一致: 日志行={line}"
    );
}

#[tokio::test]
async fn cyclic_departments_appear_exactly_once_and_are_not_dropped() {
    let db = test_common::setup_test_db().await;
    seed(&db).await;
    let logs = capture_logs();
    let tree = tree_of(&db).await;
    let all_ids = collect_ids(&tree);

    // 双节点成环、三节点成环、以及挂在环下的子节点：每个都必须出现且只出现一次
    for id in [
        DEPT_MUTUAL_A,
        DEPT_MUTUAL_B,
        DEPT_RING_A,
        DEPT_RING_B,
        DEPT_RING_C,
        DEPT_RING_LEAF,
    ] {
        assert!(
            find_node(&tree, id).is_some(),
            "环上或环下的部门 {id} 不得从树出参消失: {all_ids:?}"
        );
        assert_eq!(
            occurrence_count(&all_ids, id),
            1,
            "成环不得让节点 {id} 重复出现，也不得因取走顺序漏掉它: {all_ids:?}"
        );
    }

    // 每个环只由主键最小的成员承担顶层展开，环上其余成员挂在其下：
    // 顶层里同一环的成员至多一个，多于一个就是同一环被展开多次
    let mutual = [DEPT_MUTUAL_A, DEPT_MUTUAL_B];
    let ring = [DEPT_RING_A, DEPT_RING_B, DEPT_RING_C];
    for group in [mutual.to_vec(), ring.to_vec()] {
        let top_hits = top_level_ids(&tree)
            .into_iter()
            .filter(|id| group.contains(id))
            .count();
        assert_eq!(
            top_hits,
            1,
            "一个环只应有一个顶层入口，其余成员挂在其下: 环={group:?} 顶层={:?}",
            top_level_ids(&tree)
        );
    }

    let text = logs.text();
    let cycle_line = logs.line_containing("父级指向环内其他部门（层级成环）");
    assert!(
        !cycle_line.is_empty(),
        "成环必须单独点名，不能混进悬挂类: 日志={text}"
    );
    // 环上成员的父级是环内其他部门；环下叶的父级也在环上，两类都归这一条描述
    let mut cycle_reported: Vec<i32> = mutual.to_vec();
    cycle_reported.extend_from_slice(&ring);
    cycle_reported.push(DEPT_RING_LEAF);
    for id in &cycle_reported {
        assert!(
            cycle_line.contains(&id.to_string()),
            "父级指向环内部门的节点 {id} 必须被点名: 日志行={cycle_line}"
        );
    }

    // 悬挂节点的子节点自身不在环上、其父级也不是环上节点，必须落到第三类描述里，
    // 不能被谎报成"父级指向环内部门"
    let stuck_parent_line = logs.line_containing("父级同样从根不可达");
    assert!(
        stuck_parent_line.contains(&DEPT_DANGLING_GRANDCHILD.to_string()),
        "父级只是悬挂节点的子节点应按真实根因分类: 日志行={stuck_parent_line}"
    );
    assert!(
        !cycle_line.contains(&DEPT_DANGLING_GRANDCHILD.to_string()),
        "环下与悬挂下的子节点分类不得串到层级成环那条里: 日志行={cycle_line}"
    );
}

// ---------------------------------------------------------------------------
// 判据三：树节点出参带出 code 与 sort_order 与 is_active，键恒存在
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tree_node_output_carries_code_sort_order_and_is_active() {
    let db = test_common::setup_test_db().await;
    seed(&db).await;
    let _logs = capture_logs();
    let tree = tree_of(&db).await;

    let expected_keys = [
        "children",
        "code",
        "description",
        "id",
        "is_active",
        "manager_name",
        "name",
        "parent_id",
        "sort_order",
    ];

    // 被提升为根的悬挂节点（is_active 落库为假）、正常父子链的叶子、环下叶都要齐列
    for id in [DEPT_DANGLING_CHILD, DEPT_NORMAL_CHILD, DEPT_RING_LEAF] {
        let node = find_node(&tree, id).unwrap_or_else(|| panic!("出参应含节点 {id}"));
        let json = serde_json::to_value(node).expect("树节点应可序列化");
        let obj = json
            .as_object()
            .unwrap_or_else(|| panic!("树节点出参应为对象: {json}"));
        let mut keys: Vec<&String> = obj.keys().collect();
        keys.sort();
        let actual: Vec<&str> = keys.iter().map(|k| k.as_str()).collect();
        assert_eq!(
            actual, expected_keys,
            "树节点键集合必须精确等于九键且恒存在（前端列绑定的前提）: 节点={id}"
        );
        assert_eq!(
            obj["code"].as_str(),
            Some(code_of(id).as_str()),
            "code 必须来自 departments.code 真实列: 节点={id}"
        );
        assert_eq!(
            obj["sort_order"].as_i64(),
            Some(sort_order_of(id) as i64),
            "sort_order 必须来自 departments.sort_order 真实列: 节点={id}"
        );
        assert_eq!(
            obj["is_active"].as_bool(),
            Some(id != DEPT_DANGLING_CHILD),
            "is_active 必须来自 departments.is_active 真实列，不得写死真值: 节点={id}"
        );
        assert!(
            obj["children"].is_array(),
            "children 键恒存在，叶子应为空数组而不是缺席: 节点={id}"
        );
    }

    // 正常层级仍然真实嵌套（回收逻辑不得把可达链路拍平）
    let parent = find_node(&tree, DEPT_NORMAL_PARENT).expect("正常父部门应在出参中");
    let child = parent
        .children
        .iter()
        .find(|c| c.id == DEPT_NORMAL_CHILD)
        .expect("父节点 children 应含正常子部门");
    assert!(
        child.children.is_empty(),
        "正常子部门是叶子，children 应为空数组"
    );
}

#[tokio::test]
async fn handler_emits_bare_array_with_three_new_columns() {
    let db = Arc::new(test_common::setup_test_db().await);
    seed(&db).await;
    let _logs = capture_logs();

    let app = build_app(make_auth(1), db.clone());
    let req = Request::builder()
        .method(Method::GET)
        .uri("/erp/departments/tree")
        .body(Body::empty())
        .expect("测试夹具：请求构造失败");
    let resp = app.oneshot(req).await.expect("测试夹具：handler 调用失败");
    assert_eq!(resp.status(), StatusCode::OK, "树端点应 2xx");
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .expect("测试夹具：响应体读取失败");
    let v: Value =
        serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("响应应是合法 JSON: {e}"));

    // 出参载荷恒为裸数组（无分页信封），前端按 data 直读
    let list = v["data"]
        .as_array()
        .unwrap_or_else(|| panic!("树端点 data 应为裸数组: {v}"));
    let nodes: Vec<&Value> = list
        .iter()
        .filter(|n| n["id"].as_i64() == Some(DEPT_DANGLING_CHILD as i64))
        .collect();
    assert_eq!(
        nodes.len(),
        1,
        "被提升的悬挂部门应作为一个顶层节点出现在裸数组里: {v}"
    );
    let node = nodes[0];
    for key in ["code", "sort_order", "is_active"] {
        assert!(
            node.get(key).is_some(),
            "端点出参必须含 {key}，前端才能免拼全量: {node}"
        );
    }
    assert_eq!(
        node["code"].as_str(),
        Some(code_of(DEPT_DANGLING_CHILD).as_str()),
        "端点出参的 code 应等于落库编码"
    );
    assert_eq!(
        node["sort_order"].as_i64(),
        Some(sort_order_of(DEPT_DANGLING_CHILD) as i64),
        "端点出参的 sort_order 应等于落库排序号"
    );
    assert_eq!(
        node["is_active"].as_bool(),
        Some(false),
        "端点出参的 is_active 应等于落库启用状态"
    );
}

fn build_app(auth: AuthContext, db: Arc<DatabaseConnection>) -> Router {
    let state = AppState {
        db,
        ..Default::default()
    };
    Router::new()
        .route(
            "/erp/departments/tree",
            get(department_handler::get_department_tree),
        )
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn inject_auth(
    State(auth): State<AuthContext>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    request.extensions_mut().insert(auth);
    next.run(request).await
}

// ---------------------------------------------------------------------------
// 判据四：分页排序构成全序，同名跨页不重复不遗漏
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_pages_same_named_departments_without_dup_or_gap() {
    let db = test_common::setup_test_db().await;
    seed(&db).await;
    let _logs = capture_logs();
    let service = DepartmentService::new(Arc::new(db.clone()));

    let query = |page: u64| department_handler::DepartmentListQuery {
        page: Some(page),
        page_size: Some(1),
        parent_id: None,
        search: Some(SAMEDUP_NAME.to_string()),
    };

    let first = service.list(query(1)).await.expect("第一页查询失败");
    let second = service.list(query(2)).await.expect("第二页查询失败");

    assert_eq!(first.total, 2, "同名两条都应被 search 命中: {first:?}");
    let mut page_ids: Vec<i32> = first.items.iter().map(|d| d.id).collect();
    page_ids.extend(second.items.iter().map(|d| d.id));
    assert_eq!(
        page_ids,
        vec![DEPT_SAMEDUP_ROW_SECOND, DEPT_SAMEDUP_ROW_FIRST],
        "同名行的页序必须以主键升序破平（先插的主键大的那条不得排到前面），且翻页不重复不遗漏"
    );
    assert_eq!(page_ids.len(), 2, "两页并集应恰为同名两条，没有重复取回");
}

// ---------------------------------------------------------------------------
// 判据五：根级与层内先后按排序号升序、主键升序构成全序，且两次取树给出同一序列
// ---------------------------------------------------------------------------

/// 取顶层节点主键序列（保留出参先后，不排序）
fn root_sequence(nodes: &[DepartmentTreeNode]) -> Vec<i32> {
    nodes.iter().map(|n| n.id).collect()
}

/// 只看本用例的顺序夹具根，滤掉迁移种子与其他用例的行
fn keep_order_roots(ids: &[i32]) -> Vec<i32> {
    ids.iter()
        .copied()
        .filter(|id| {
            [
                DEPT_ORDER_ROOT_TAIL,
                DEPT_ORDER_ROOT_HEAD,
                DEPT_ORDER_ROOT_TIE_LOW,
                DEPT_ORDER_ROOT_TIE_HIGH,
            ]
            .contains(id)
        })
        .collect()
}

#[tokio::test]
async fn tree_orders_roots_and_siblings_by_sort_order_then_id() {
    let db = test_common::setup_test_db().await;
    seed(&db).await;
    let _logs = capture_logs();
    let tree = tree_of(&db).await;

    // 根级先后：排序号 8 的最先，120 的最后，两条并列 60 的按主键升序，
    // 与主键/物理写入先后（9744 最先）相反，只按迭代序或主键排都会给出别的序列
    assert_eq!(
        keep_order_roots(&root_sequence(&tree)),
        vec![
            DEPT_ORDER_ROOT_HEAD,
            DEPT_ORDER_ROOT_TIE_LOW,
            DEPT_ORDER_ROOT_TIE_HIGH,
            DEPT_ORDER_ROOT_TAIL
        ],
        "根级必须按 sort_order 升序、并列时按主键升序"
    );

    // 层内先后同一口径
    let parent = find_node(&tree, DEPT_ORDER_PARENT).expect("顺序夹具父节点应在出参中");
    let child_ids: Vec<i32> = parent.children.iter().map(|c| c.id).collect();
    assert_eq!(
        child_ids,
        vec![
            DEPT_ORDER_CHILD_HEAD,
            DEPT_ORDER_CHILD_TIE_LOW,
            DEPT_ORDER_CHILD_TIE_HIGH,
            DEPT_ORDER_CHILD_TAIL
        ],
        "同一父级下的 children 必须按 sort_order 升序、并列时按主键升序"
    );

    // 两次取树给出同一序列：dept_map 是 HashMap，迭代序随 RandomState 变化，
    // 未显式定序时这条会随机判红（前端一次取树直渲，顺序跳变就是用户可见缺陷）
    let second = tree_of(&db).await;
    assert_eq!(
        keep_order_roots(&root_sequence(&second)),
        keep_order_roots(&root_sequence(&tree)),
        "同一份数据两次取树的根级顺序必须一致"
    );
    let second_parent =
        find_node(&second, DEPT_ORDER_PARENT).expect("第二次取树也应含顺序夹具父节点");
    assert_eq!(
        second_parent
            .children
            .iter()
            .map(|c| c.id)
            .collect::<Vec<i32>>(),
        parent.children.iter().map(|c| c.id).collect::<Vec<i32>>(),
        "同一份数据两次取树的层内顺序必须一致"
    );
}
