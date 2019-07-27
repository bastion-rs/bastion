#[macro_export]
macro_rules! receive {
    ( $msg:expr, $($rest:tt)* ) => {
        receive!(@private $msg, ($($rest)*) -> ())
    };

    (@private $msg:expr, ($recvty:ty => $clo:expr, _ => $fallback:expr) -> ($($parsed:tt)*) ) => {
        receive!(@private $msg, () -> ($($parsed)* ($recvty => $clo)) $fallback)
    };

    (@private $msg:expr, ($recvty:ty => $clo:expr, $($rest:tt)*) -> ($($parsed:tt)*) ) => {
        receive!(@private $msg, ($($rest)*) -> ($($parsed)* ($recvty => $clo)))
    };

    (@private $msg:expr, () -> ($(($recvty:ty => $clo:expr))*) $fallback:expr ) => {
        {
            loop {
                $(
                let data = objekt::clone_box(&*$msg);
                if let Receive(Some(o)) = Receive::<$recvty>::from(data) {
                    $clo(o);
                    break
                }
                )+
                else { $fallback; break }
            }
        }
    };
}
