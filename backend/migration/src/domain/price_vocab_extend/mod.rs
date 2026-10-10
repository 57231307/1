//! 价目状态词表 CHECK 后继扩集域（m_price_vocab_check 的下游）
//!
//! - m_price_vocab_check: 首次把 ('pending','approved') /
//!   ('pending','approved','inactive') 钉入两表 status CHECK。
//! - 本域 m_price_vocab_extend: 拒绝动作上线批次（同批=词表加 rejected +
//!   写入方 + 本 CHECK 扩集 + 契约锁），以相同约束名扩至含 'rejected' 的取值集，
//!   守卫与回滚均为 fail-visible（照 price_vocab_check「拒绝造值」范式）。
//!
//! 注册位置（lib.rs 迁移链）必须在 m_price_vocab_check 之后、m_price_fk 之前：
//! 前驱已完成默认值收敛与 'ACTIVE' 回填（本迁移守卫以此为存量基线）；
//! 置于 price_fk 之前不影响外键面（两迁移触碰的对象不相交）。

mod m0080_extend_price_status_check_rejected;

pub use m0080_extend_price_status_check_rejected::Migration;
