use std::{
    collections::{BTreeMap, HashSet},
    sync::LazyLock,
};

use global_utils::logger::{LoggerGuard, init_logger};

pub static TEST_LOGGER: LazyLock<LoggerGuard> = LazyLock::new(|| init_logger());

/// Transforms a vector into a BTreeMap using a key extraction function.
pub fn vec_to_btreemap<K, V, F>(vec: Vec<V>, key_fn: F) -> BTreeMap<K, V>
where
    K: Ord,
    F: Fn(&V) -> K,
{
    let mut map = BTreeMap::new();
    for v in vec {
        map.insert(key_fn(&v), v);
    }
    map
}

pub fn vecs_equal_unordered<T: Eq + std::hash::Hash>(a: &[T], b: &[T]) -> bool {
    let set_a: HashSet<_> = a.iter().collect();
    let set_b: HashSet<_> = b.iter().collect();
    set_a == set_b
}
