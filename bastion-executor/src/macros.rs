#[doc(hidden)]
macro_rules! unstable_api {
    ($($block:item)*) => {
        $(
            #[cfg(feature = "unstable")]
            $block
        )*
    }
}
