#[cfg(test)]
mod tests {
    use bastion_executor::{placement, pool};

    #[test]
    fn affinity_replacement() {
        let core_ids = placement::get_core_ids().unwrap();
        dbg!(core_ids);
    }

    #[test]
    fn pool_check() {
        pool::get();
    }
}
