//!
//! Pool of threads to run lightweight processes
//!
//! Pool management and tracking belongs here.
//! We spawn futures onto the pool with [spawn] method of global run queue or
//! with corresponding [Worker]'s spawn method.
use crate::run_queue::{Injector, Stealer};
use crate::sleepers::Sleepers;
use crate::worker;
use lazy_static::lazy_static;
use lightproc::prelude::*;
use std::future::Future;
use tracing::warn;

///
/// Spawn a process (which contains future + process stack) onto the executor from the global level.
///
/// # Example
/// ```rust
/// use bastion_executor::prelude::*;
/// use lightproc::prelude::*;
///
/// let pid = 1;
/// let stack = ProcStack::default().with_pid(pid);
///
/// let handle = spawn(
///     async {
///         panic!("test");
///     },
///     stack.clone(),
/// );
///
/// run(
///     async {
///         handle.await;
///     },
///     stack.clone(),
/// );
/// ```
pub fn spawn<F, T>(future: F, stack: ProcStack) -> RecoverableHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let (task, handle) = LightProc::recoverable(future, worker::schedule, stack);
    task.schedule();
    handle
}

impl Pool {
    ///
    /// Spawn a process (which contains future + process stack) onto the executor via [Pool] interface.
    pub fn spawn<F, T>(&self, future: F, stack: ProcStack) -> RecoverableHandle<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        // Log this `spawn` operation.
        let _child_id = stack.get_pid() as u64;
        let _parent_id = worker::get_proc_stack(|t| t.get_pid() as u64).unwrap_or(0);

        let (task, handle) = LightProc::recoverable(future, worker::schedule, stack);
        task.schedule();
        handle
    }
}

///
/// Acquire the static Pool reference
#[inline]
pub fn get() -> &'static Pool {
    &*POOL
}

use std::collections::VecDeque;
use std::iter::Iterator;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use std::{env, thread};

use crossbeam_channel::{unbounded, Receiver, Sender};
use crossbeam_queue::ArrayQueue;

use lightproc::lightproc::LightProc;
use lightproc::proc_stack::ProcStack;
use lightproc::recoverable_handle::RecoverableHandle;

use crate::placement::CoreId;
use crate::{load_balancer, placement};
use once_cell::sync::OnceCell;
use thread::Thread;
use tracing::{debug, error, trace};

/// If low watermark isn't configured this is the default scaler value.
/// This value is used for the heuristics of the scaler
const DEFAULT_LOW_WATERMARK: u64 = 2;

/// The default thread park timeout before checking for new tasks.
const THREAD_PARK_TIMEOUT: Duration = Duration::from_millis(1);

/// The default dynamic thread receive timeout before we park / terminate.
const THREAD_RECV_TIMEOUT_MILLIS: u64 = 100;

const THREAD_RECV_TIMEOUT: Duration = Duration::from_millis(THREAD_RECV_TIMEOUT_MILLIS);

/// Pool managers interval time (milliseconds).
/// This is the actual interval which makes adaptation calculation.
const MANAGER_POLL_INTERVAL: u64 = 10000;

/// Frequency histogram's sliding window size.
/// Defines how many frequencies will be considered for adaptation.
const FREQUENCY_QUEUE_SIZE: usize = 10;

/// Exponential moving average smoothing coefficient for limited window.
/// Smoothing factor is estimated with: 2 / (N + 1) where N is sample size.
const EMA_COEFFICIENT: f64 = 2_f64 / (FREQUENCY_QUEUE_SIZE as f64 + 1_f64);

/// Pool task frequency variable.
/// Holds scheduled tasks onto the thread pool for the calculation time window.
static FREQUENCY: AtomicU64 = AtomicU64::new(0);

/// Possible max threads (without OS contract).
static MAX_THREADS: usize = 10_000;

/// Pool interface between the scheduler and thread pool
pub struct Pool {
    sender: Sender<LightProc>,
    receiver: Receiver<LightProc>,
}

lazy_static! {
    /// Blocking pool with static starting thread count.
    static ref POOL: Pool = {

        DYNAMIC_THREAD_MANAGER
            .set(DynamicThreadManager::new(*low_watermark() as usize))
            .expect("couldn't setup the dynamic thread manager");

        let (sender, receiver) = unbounded();
        Pool { sender, receiver }
    };

    static ref ROUND_ROBIN_PIN: Mutex<CoreId> = Mutex::new(CoreId { id: 0 });

    /// Sliding window for pool task frequency calculation
    static ref FREQ_QUEUE: Mutex<VecDeque<u64>> = {
        Mutex::new(VecDeque::with_capacity(FREQUENCY_QUEUE_SIZE.saturating_add(1)))
    };
}

/// Exponentially Weighted Moving Average calculation
///
/// This allows us to find the EMA value.
/// This value represents the trend of tasks mapped onto the thread pool.
/// Calculation is following:
/// ```text
/// +--------+-----------------+----------------------------------+
/// | Symbol |   Identifier    |           Explanation            |
/// +--------+-----------------+----------------------------------+
/// | α      | EMA_COEFFICIENT | smoothing factor between 0 and 1 |
/// | Yt     | freq            | frequency sample at time t       |
/// | St     | acc             | EMA at time t                    |
/// +--------+-----------------+----------------------------------+
/// ```
/// Under these definitions formula is following:
/// ```text
/// EMA = α * [ Yt + (1 - α)*Yt-1 + ((1 - α)^2)*Yt-2 + ((1 - α)^3)*Yt-3 ... ] + St
/// ```
/// # Arguments
///
/// * `freq_queue` - Sliding window of frequency samples
#[inline]
fn calculate_ema(freq_queue: &VecDeque<u64>) -> f64 {
    freq_queue.iter().enumerate().fold(0_f64, |acc, (i, freq)| {
        acc + ((*freq as f64) * ((1_f64 - EMA_COEFFICIENT).powf(i as f64) as f64))
    }) * EMA_COEFFICIENT as f64
}

/// Adaptive pool scaling function
///
/// This allows to spawn new threads to make room for incoming task pressure.
/// Works in the background detached from the pool system and scales up the pool based
/// on the request rate.
///
/// It uses frequency based calculation to define work. Utilizing average processing rate.
fn scale_pool() {
    // Fetch current frequency, it does matter that operations are ordered in this approach.
    let current_frequency = FREQUENCY.swap(0, Ordering::SeqCst);
    let mut freq_queue = FREQ_QUEUE.lock().unwrap();

    // Make it safe to start for calculations by adding initial frequency scale
    if freq_queue.len() == 0 {
        freq_queue.push_back(0);
    }

    // Calculate message rate for the given time window
    let frequency = (current_frequency as f64 / MANAGER_POLL_INTERVAL as f64) as u64;

    // Calculates current time window's EMA value (including last sample)
    let prev_ema_frequency = calculate_ema(&freq_queue);

    // Add seen frequency data to the frequency histogram.
    freq_queue.push_back(frequency);
    if freq_queue.len() == FREQUENCY_QUEUE_SIZE.saturating_add(1) {
        freq_queue.pop_front();
    }

    // Calculates current time window's EMA value (including last sample)
    let curr_ema_frequency = calculate_ema(&freq_queue);

    // Adapts the thread count of pool
    //
    // Sliding window of frequencies visited by the pool manager.
    // Pool manager creates EMA value for previous window and current window.
    // Compare them to determine scaling amount based on the trends.
    // If current EMA value is bigger, we will scale up.
    if curr_ema_frequency > prev_ema_frequency {
        // "Scale by" amount can be seen as "how much load is coming".
        // "Scale" amount is "how many threads we should spawn".
        let scale_by: f64 = curr_ema_frequency - prev_ema_frequency;
        let scale = num_cpus::get().min(
            ((DEFAULT_LOW_WATERMARK as f64 * scale_by) + DEFAULT_LOW_WATERMARK as f64) as usize,
        );
        trace!("unparking {} threads", scale);

        // It is time to scale the pool!
        dynamic_thread_manager().provision_threads(scale);
    } else if (curr_ema_frequency - prev_ema_frequency).abs() < std::f64::EPSILON
        && current_frequency != 0
    {
        // Throughput is low. Allocate more threads to unblock flow.
        // If we fall to this case, scheduler is congested by longhauling tasks.
        // For unblock the flow we should add up some threads to the pool, but not that many to
        // stagger the program's operation.
        trace!("unparking {} threads", DEFAULT_LOW_WATERMARK);
        dynamic_thread_manager().provision_threads(DEFAULT_LOW_WATERMARK as usize);
    }
}

/// Enqueues work, attempting to send to the thread pool in a
/// nonblocking way and spinning up needed amount of threads
/// based on the previous statistics without relying on
/// if there is not a thread ready to accept the work or not.
pub(crate) fn schedule(t: LightProc) {
    // Add up for every incoming scheduled task
    FREQUENCY.fetch_add(1, Ordering::Acquire);

    if let Err(err) = POOL.sender.try_send(t) {
        // We were not able to send to the channel without
        // blocking.
        POOL.sender.send(err.into_inner()).unwrap();
    }
}

/// Spawns a blocking task.
///
/// The task will be spawned onto a thread pool specifically dedicated to blocking tasks.
pub fn spawn_blocking<F, R>(future: F, stack: ProcStack) -> RecoverableHandle<R>
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    let (task, handle) = LightProc::recoverable(future, schedule, stack);
    task.schedule();
    handle
}

///
/// Low watermark value, defines the bare minimum of the pool.
/// Spawns initial thread set.
/// Can be configurable with env var `BASTION_BLOCKING_THREADS` at runtime.
#[inline]
pub fn low_watermark() -> &'static u64 {
    lazy_static! {
        static ref LOW_WATERMARK: u64 = {
            env::var_os("BASTION_BLOCKING_THREADS")
                .map(|x| x.to_str().unwrap().parse::<u64>().unwrap())
                .unwrap_or(DEFAULT_LOW_WATERMARK)
        };
    }

    &*LOW_WATERMARK
}

///
/// Affinity pinner for blocking pool
/// Pinning isn't going to be enabled for single core systems.
#[inline]
pub fn affinity_pinner() {
    if 1 != *load_balancer::core_count() {
        let mut core = ROUND_ROBIN_PIN.lock().unwrap();
        placement::set_for_current(*core);
        core.id = (core.id + 1) % *load_balancer::core_count();
    }
}

static DYNAMIC_THREAD_MANAGER: OnceCell<DynamicThreadManager> = OnceCell::new();

fn dynamic_thread_manager() -> &'static DynamicThreadManager {
    DYNAMIC_THREAD_MANAGER.get().unwrap_or_else(|| {
        error!("couldn't get the dynamic thread manager");
        panic!("couldn't get the dynamic thread manager");
    })
}

static POOL_SIZE: OnceCell<AtomicUsize> = OnceCell::new();

#[derive(Debug)]
struct DynamicThreadManager {
    parked_threads: ArrayQueue<Thread>,
}

impl DynamicThreadManager {
    pub fn new(static_threads: usize) -> Self {
        let dynamic_threads = 1.max(num_cpus::get() - static_threads);

        POOL_SIZE
            .set(AtomicUsize::new(static_threads + dynamic_threads))
            .expect("couldn't initialize pool_size");

        Self::initialize_thread_pool(static_threads, dynamic_threads);
        Self {
            parked_threads: ArrayQueue::new(dynamic_threads),
        }
    }

    /// Initialize the dynamic pool
    /// That will be scaled
    fn initialize_thread_pool(static_threads: usize, dynamic_threads: usize) {
        // Static thread manager that will always be available
        trace!("setting up the static thread manager");
        (0..static_threads).for_each(|_| {
            thread::Builder::new()
                .name("bastion-driver-static".to_string())
                .spawn(|| {
                    self::affinity_pinner();
                    loop {
                        for task in &POOL.receiver {
                            trace!("static thread: running task");
                            task.run();
                        }

                        trace!("static thread: empty queue, parking_timeout before polling again");
                        thread::park_timeout(THREAD_PARK_TIMEOUT);
                    }
                })
                .expect("couldn't spawn static thread");
        });

        // Dynamic thread manager that will allow us to unpark threads
        // According to the needs
        trace!("setting up the dynamic thread manager");
        (0..dynamic_threads).for_each(|_| {
            thread::Builder::new()
                .name("bastion-driver-dynamic".to_string())
                .spawn(move || {
                    self::affinity_pinner();
                    loop {
                        while let Ok(task) = POOL.receiver.recv_timeout(THREAD_RECV_TIMEOUT) {
                            trace!("dynamic thread: running task");
                            task.run();
                        }
                        trace!(
                            "dynamic thread: parking - {:?}",
                            std::thread::current().id()
                        );
                        dynamic_thread_manager().park_thread();
                    }
                }) .expect("cannot start dynamic thread");
        });

        // Pool manager to check frequency of task rates
        // and take action by scaling the pool accordingly.
        thread::Builder::new()
            .name("bastion-pool-manager".to_string())
            .spawn(|| {
                let poll_interval = Duration::from_millis(MANAGER_POLL_INTERVAL);
                trace!("setting up the pool manager");
                loop {
                    scale_pool();
                    thread::park_timeout(poll_interval);
                }
            })
            .expect("thread pool manager cannot be started");
    }

    /// Provision threads takes a number of threads that need to be made available.
    /// It will try to unpark threads from the dynamic pool, and spawn more threads if needs be.
    pub fn provision_threads(&self, n: usize) {
        for i in 0..n {
            if !self.unpark_thread() {
                let new_threads = n - i;
                trace!(
                    "no more threads to unpark, spawning {} new threads",
                    new_threads
                );
                return self.spawn_threads(new_threads);
            }
        }
    }

    fn spawn_threads(&self, n: usize) {
        if (MAX_THREADS
            - POOL_SIZE
                .get()
                .expect("couldn't get pool size")
                .load(Ordering::SeqCst))
            < n
        {
            warn!("reached thread spawn limit");
            return;
        }
        POOL_SIZE.get().unwrap().fetch_add(n, Ordering::SeqCst);
        (0..n).for_each(|_| {
            thread::Builder::new()
                .name("bastion-blocking-driver-standalone".to_string())
                .spawn(move || {
                    self::affinity_pinner();
                    while let Ok(task) = POOL.receiver.recv_timeout(THREAD_RECV_TIMEOUT) {
                        task.run();
                    }
                    trace!("standalone thread: quitting.");
                    POOL_SIZE.get().unwrap().fetch_sub(1, Ordering::SeqCst);
                }).unwrap();
        })
    }

    /// Parks a thread until unpark_thread unparks it
    pub fn park_thread(&self) {
        let _ = self
            .parked_threads
            .push(std::thread::current())
            .map(|_| {
                trace!("parking thread {:?}", std::thread::current().id());
                std::thread::park();
            })
            .map_err(|e| {
                debug!(
                    "couldn't park thread {:?} - {}",
                    std::thread::current().id(),
                    e
                );
            });
    }

    /// Pops a thread from the parked_threads queue and unparks it.
    /// returns true on success.
    fn unpark_thread(&self) -> bool {
        if self.parked_threads.is_empty() {
            trace!("no parked threads");
            false
        } else {
            trace!("parked_threads: len is {}", self.parked_threads.len());
            self.parked_threads
                .pop()
                .map(|thread| {
                    debug!("Executor: unpark_thread: unparking {:?}", thread.id());
                    thread.unpark();
                })
                .map_err(|e| {
                    debug!("Executor: unpark_thread: couldn't unpark thread - {}", e);
                })
                .is_ok()
        }
    }
}
