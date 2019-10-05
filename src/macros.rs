//!
//! Macros to use in processes and defining the architecture of runtime.
//!

///
/// Matches incoming messages to the process.
/// Always have a `default` case to execute if unknown message arrives to the process.
///
/// # Examples
///```rust
///# use bastion::prelude::*;
///#
///# fn main() {
///#     Bastion::platform();
///#
///#     /// Define, calculate or prepare messages to be sent to the processes.
///#     let message = String::from("Some message to be passed");
///#
///#     Bastion::spawn(
///#         |context: BastionContext, msg: Box<dyn Message>| {
///#             /// Message can be selected with receiver here. Take action!
///receive! { msg,
///    String => |e| { println!("Received string :: {}", e)},
///    i32 => |e| {println!("Received i32 :: {}", e)},
///    _ => println!("No message as expected. Default")
///}
///#
///#             /// Do some processing in body
///#             println!("root supervisor - spawn_at_root - 1");
///#
///#             /// Rebind to the system
///#             context.hook();
///#         },
///#         message,
///#     );
///# }
/// ```
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
