use std::collections::HashMap;
use std::hash::Hash;

/// 树节点特征，实现此 trait 的结构体可用于 `build_tree` 构建树形结构
pub trait TreeNode {
    type Id: Eq + Hash + Clone;

    fn id(&self) -> Self::Id;
    fn parent_id(&self) -> Option<Self::Id>;
}

/// 将扁平列表转换为树形结构
///
/// # 参数
/// - `items`: 扁平数据列表
/// - `set_children`: 设置子节点的闭包，接收 `&mut T` 和 `Vec<T>`
///
/// # 返回
/// 根节点列表（parent_id 为 None 的节点）
pub fn build_tree<T, F>(items: Vec<T>, mut set_children: F) -> Vec<T>
where
    T: TreeNode + Clone,
    F: FnMut(&mut T, Vec<T>),
{
    let mut map: HashMap<T::Id, T> = HashMap::new();
    let mut child_ids: HashMap<T::Id, Vec<T::Id>> = HashMap::new();
    let mut root_ids: Vec<T::Id> = Vec::new();

    for item in items {
        let id = item.id();
        let parent_id = item.parent_id();

        match parent_id {
            Some(pid) => child_ids.entry(pid).or_default().push(id.clone()),
            None => root_ids.push(id.clone()),
        }
        map.insert(id, item);
    }

    fn build_node<T, F>(
        id: &T::Id,
        map: &mut HashMap<T::Id, T>,
        child_ids: &HashMap<T::Id, Vec<T::Id>>,
        set_children: &mut F,
    ) -> T
    where
        T: TreeNode + Clone,
        F: FnMut(&mut T, Vec<T>),
    {
        let mut node = map.remove(id).expect("node should exist in map");
        let children_ids = child_ids.get(id).cloned().unwrap_or_default();
        let children: Vec<T> = children_ids
            .iter()
            .map(|child_id| build_node(child_id, map, child_ids, set_children))
            .collect();
        if !children.is_empty() {
            set_children(&mut node, children);
        }
        node
    }

    root_ids
        .iter()
        .map(|id| build_node(id, &mut map, &child_ids, &mut set_children))
        .collect()
}
