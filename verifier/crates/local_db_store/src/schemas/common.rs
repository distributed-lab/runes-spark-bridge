pub(crate) trait ValuesMaxCapacity {
    const MAX_CAPACITY: usize;
}

pub(crate) trait ValuesToModifyInit<'a, T: ValuesMaxCapacity> {
    /// Function initializes boilerplate code for passing dynamic amount of values into sqlx to apply modifications in db
    #[inline]
    fn init_values_to_modify(init_param: usize) -> (Vec<String>, Box<dyn FnMut(&str) -> String>) {
        let conditions: Vec<String> = Vec::with_capacity(T::MAX_CAPACITY);
        let get_condition_closure = {
            let mut i = init_param;
            move |name: &str| {
                let s = format!("{} = ${}", name, i);
                i += 1;
                s
            }
        };
        (conditions, Box::new(get_condition_closure))
    }
}

impl<'a, T: ValuesMaxCapacity> ValuesToModifyInit<'a, T> for T {}
