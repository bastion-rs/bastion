#[cfg(test)]
mod tests {
    use bastion_executor::{placement, pool};

    #[test]
    fn affinity_replacement() {
        let core_ids = placement::get_core_ids().unwrap();
        dbg!(core_ids);
    }

    #[cfg(feature = "tokio-runtime")]
    mod tokio_tests {
        #[tokio::test]
        async fn pool_check() {
            super::pool::get();
        }
    }

    #[cfg(not(feature = "tokio-runtime"))]
    mod no_tokio_tests {
        #[test]
        fn pool_check() {
            super::pool::get();
        }
    }
}
