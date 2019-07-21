use log::LevelFilter;

#[derive(Clone, Copy)]
pub struct BastionConfig {
    pub log_level: LevelFilter,
    pub in_test: bool,
}
