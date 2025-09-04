use std::{
    collections::{BTreeMap, HashSet},
    sync::LazyLock,
};

use global_utils::logger::{LoggerGuard, init_logger};

pub static TEST_LOGGER: LazyLock<LoggerGuard> = LazyLock::new(|| init_logger());
