use bastion::prelude::*;

use tracing::error;

// prime_number contains all the required functions
// and structures to generate a prime number and return it.
mod prime_number {
    use std::iter;
    use std::time::{Duration, Instant};

    #[derive(Debug)]
    pub struct Response {
        prime_number: u128,
        num_digits: usize,
        compute_time: Duration,
    }

    pub fn prime_number(num_digits: usize) -> Response {
        // Start a stopwatch
        let start = Instant::now();
        // Get a prime number
        let prime_number = get_prime(num_digits);
        // Stop the stopwatch
        let elapsed = Instant::now().duration_since(start);

        Response {
            prime_number,
            num_digits,
            compute_time: elapsed,
        }
    }

    impl std::fmt::Display for Response {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{} is a prime number that has {} digits. If was found in {}s and {}ms",
                self.prime_number,
                self.num_digits,
                self.compute_time.as_secs(),
                self.compute_time.as_millis() % 1000
            )
        }
    }

    fn get_prime(num_digits: usize) -> u128 {
        // with num_digits = 4, min_bound == 10000
        let min_bound = get_min_bound(num_digits);
        // with num_digits = 4, max_bound == 10000
        let max_bound = get_max_bound(num_digits);
        // maybe_prime is a number in range [1000, 10000)
        // the closing parenthesiss means it won't reach the number.
        // the maximum allowed value for maybe_prime is 9999.
        use rand::Rng;
        let mut maybe_prime = rand::thread_rng().gen_range(min_bound..max_bound);
        loop {
            if is_prime(maybe_prime) {
                return number_or_panic(maybe_prime);
            }
            // for any integer n > 3,
            // there always exists at least one prime number p
            // with n < p < 2n - 2
            maybe_prime += 1;
            // We don't want to return a number
            // that doesn't have the right number of digits
            if maybe_prime == max_bound {
                maybe_prime = min_bound;
            }
        }
    }

    fn number_or_panic(number_to_return: u128) -> u128 {
        // Let's roll a dice
        if rand::random::<u8>() % 6 == 0 {
            panic!(format!(
                "I was about to return {} but I chose to panic instead!",
                number_to_return
            ))
        }
        number_to_return
    }

    fn get_min_bound(num_digits: usize) -> u128 {
        let lower_bound_iter =
            iter::once(1usize).chain(iter::repeat(0usize).take(num_digits - 1 as usize));
        digits_to_number(lower_bound_iter)
    }

    fn get_max_bound(num_digits: usize) -> u128 {
        let lower_bound_iter = iter::once(1usize).chain(iter::repeat(0usize).take(num_digits));
        digits_to_number(lower_bound_iter)
    }

    // given a sequence of digits, return the corresponding number
    // eg: assert_eq!(1234, digits_to_number(vec![1,2,3,4]))
    fn digits_to_number(iter: impl Iterator<Item = usize>) -> u128 {
        iter.fold(0, |acc, b| acc * 10 + b as u128)
    }

    // in order to determine if n is prime
    // we will use a primality test.
    // https://en.wikipedia.org/wiki/Primality_test#Pseudocode
    fn is_prime(n: u128) -> bool {
        if n <= 3 {
            n > 1
        } else if n % 2 == 0 || n % 3 == 0 {
            false
        } else {
            let approximated_square_root = (((n >> 16) as f64).sqrt() as u128) << 16;
            for i in (5..=approximated_square_root).step_by(6) {
                if n % i == 0 || n % (i + 2) == 0 {
                    return false;
                }
            }
            true
        }
    }
}

// `serve_prime_numbers` is the bastion child's behavior.
// a more general waltkhrough of the msg! macro can be found in the fibonacci example.
async fn serve_prime_numbers(ctx: BastionContext) -> Result<(), ()> {
    // let's put the context in an arc, so we can pass it to other threads
    let arc_ctx = std::sync::Arc::new(ctx);
    // a child will keep processing messages until it crashes
    // (or until it gets told to shutdown)
    loop {
        // msg! is our message receiver helper.
        // we will only use one variant here
        // =!> means messages that can be replied to
        // nb_digits (in contrast to ref nb_digits)
        // means messages that have only one recipient (in contrast to broadcasts)
        // usize means it will only match against messages that are a usize
        msg! { arc_ctx.clone().recv().await?,
            nb_digits: usize =!> {
                // clone the context to send it to a thread
                let ctx2 = arc_ctx.clone();
                // answer! takes a context and will automagically figure out whom to reply to
                std::thread::spawn(move || answer!(ctx2, prime_number::prime_number(nb_digits)).expect("couldn't reply :("));
            };
            // this is a catch all for any message
            // that isn't a question with the child
            // as sole recipient and a usize as parameter
            unknown:_ => {
                error!("uh oh, I received a message I didn't understand\n {:?}", unknown);
            };
        }
    }
}

// `client` contains everything we need
// _on the client side_ to generate load on the bastion.
// we leverage rayon to make sure our client keeps stressing
// the bastion
mod client {
    use super::prime_number::Response;
    use bastion::prelude::*;
    use lightproc::prelude::*;
    use rayon::prelude::*;
    use tracing::{error, info};

    pub struct Result {
        pub child_id: usize,
        pub response: Response,
    }

    pub struct Task {
        pub child_id: usize,
        pub task: RecoverableHandle<std::io::Result<Response>>,
    }

    // `serve_digits` will allow us
    // to have enough work for each child
    // so no one gets bored :D
    pub fn serve_digits(nb_children: usize) -> Vec<usize> {
        let max = 15usize;
        let mut list = std::iter::repeat(1..=max)
            .flatten()
            .take(max * nb_children)
            .collect::<Vec<_>>();
        list.sort();
        list.reverse();
        list
    }

    // `schedule_tasks` allows us to spawn our prime requests.
    // see request_prime to see where everything is happening.
    pub fn schedule_tasks(children_ref: Vec<ChildRef>, concurrency_level: usize) -> Vec<Task> {
        // make sure rayon's thread pool is wide enough
        // for us to use all of the cpus.
        // we wouldn't do this in the real world
        // but our goal here is to use
        // all of the available resources
        rayon::ThreadPoolBuilder::new()
            .num_threads(concurrency_level)
            .build_global()
            .unwrap();

        // get a list of nb_digits to process
        serve_digits(concurrency_level)
            .into_par_iter()
            .enumerate()
            // map_with allows us to keep `children_ref` in scope
            // so we can use it in our map.
            // we will refer to it as `children` in our closure
            .map_with(children_ref, |children, (index, nb_digits)| {
                // this is our basic round robin strategy
                // each child gets 1 task to perform
                let child_number = index % children.len();
                // get the child corresponding to the index
                let child = children
                    .get(child_number)
                    .expect("missing child, this is weird")
                    .clone();
                // let's ask for prime numbers !
                let task = spawn!(request_prime(child, nb_digits));

                Task {
                    child_id: child_number,
                    task,
                }
            })
            .collect()
    }

    // `run_tasks` will run! each spawned task, and block until they're done
    pub fn run_tasks(tasks: Vec<Task>) -> Vec<std::io::Result<Result>> {
        tasks
            .into_par_iter()
            .map(|prime_task| {
                let response = run!(prime_task.task).expect("task run failed")?;
                info!("{} | {}", prime_task.child_id, response);
                Ok(Result {
                    child_id: prime_task.child_id,
                    response,
                })
            })
            .collect()
    }

    // `request_prime` sends a question to a child, and .await s a response
    async fn request_prime(child: ChildRef, nb_digits: usize) -> std::io::Result<Response> {
        // ask a question
        let reply = child
            .ask_anonymously(nb_digits)
            .expect("couldn't perform request");

        use std::io::{Error, ErrorKind};
        // wait for an answer
        msg! { reply.await.map_err(|_| Error::new(ErrorKind::Other, "child crashed"))?,
            prime_response: Response => {
                Ok(prime_response)
            };
            unknown:_ => {
                error!("uh oh, I received a message I didn't understand: {:?}", unknown);
                Err(Error::new(ErrorKind::Other, "unknown reply"))
            };
        }
    }
}

// RUST_LOG=info cargo run --release --example prime_numbers
fn main() {
    env_logger::init();

    Bastion::init();
    Bastion::start();

    // Let's create a supervisor that will watch out for children
    // and dispatch requests
    let supervisor: SupervisorRef = Bastion::supervisor(|sp| {
        sp.with_strategy(SupervisionStrategy::OneForOne)
        // ...and return it.
    })
    .expect("Couldn't create the supervisor.");

    // Let's create children that will serve prime numbers
    let concurrency_level = num_cpus::get();

    // We're using children_ref here
    // because we want to keep the children adress.
    // It will allow us to send message to specific children.
    let children = supervisor
        .children(|children| {
            children
                .with_redundancy(concurrency_level)
                .with_exec(serve_prime_numbers)
        })
        .expect("couldn't create children");

    // let's try to measure how long things will take
    let total_timer = std::time::Instant::now();

    let tasks = client::schedule_tasks(children.elems().to_vec(), concurrency_level);

    // let's run the tasks until completion
    let results = client::run_tasks(tasks);

    // let's now count how many calls failed
    let mut succeeded = 0;
    let mut failed = 0;
    let total = results.len();

    for result in results {
        if result.is_ok() {
            succeeded += 1;
        } else {
            failed += 1;
        }
    }

    println!(
        "Completed {}/{} tasks. - {} failures",
        succeeded, total, failed
    );

    println!(
        "total duration {}s {}ms",
        total_timer.elapsed().as_secs(),
        total_timer.elapsed().as_millis() - (total_timer.elapsed().as_secs() as u128 * 1000)
    );

    Bastion::stop();
    Bastion::block_until_stopped();
}
