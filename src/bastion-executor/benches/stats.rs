#![feature(test)]

extern crate test;
use bastion_executor::load_balancer::{stats, SmpStats};
use bastion_executor::placement;
use std::thread;

fn stress_stats<S: SmpStats + Sync + Send>(stats: &'static S) {
    let cores = placement::get_core_ids().expect("Core mapping couldn't be fetched");
    let mut handles = Vec::new();
    for core in cores {
        let handle = thread::spawn(move || {
            placement::set_for_current(core);
            for i in 0..100 {
                stats.store_load(core.id, 10);
                if i % 3 == 0 {
                    let _sorted_load = stats.get_sorted_load();
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

fn stress_stats_worst_merge_sort<S: SmpStats + Sync + Send>(stats: &'static S) {
    let _sorted_load = stats.get_sorted_load();
}
use test::Bencher;

// previous lock based stats benchmark 1,352,791 ns/iter (+/- 2,682,013)

// 158,278 ns/iter (+/- 117,103)
// #[bench]
// fn lockless_stats_bench(b: &mut Bencher) {
//     b.iter(|| {
//         stress_stats(stats());
//     });
// }

// 158,278 ns/iter (+/- 117,103)
#[bench]
fn lockless_stats_bad_load(b: &mut Bencher) {
    let stats = stats();
    let cores = placement::get_core_ids().expect("Core mapping couldn't be fetched");
    let total_cores = cores.len();
    const MAX_CORE: usize = 256;
    for i in 0..MAX_CORE {
        // Generating the worst possible mergesort scenario
        // [0,2,4,6,8,10,1,3,5,7,9]...
        if i <= MAX_CORE / 2 {
            stats.store_load(i, i * 2);
        } else {
            stats.store_load(i, i - 1 - MAX_CORE / 2);
        }
    }

    b.iter(|| {
        let mut handles = Vec::with_capacity(1000);
        for i in 0..100 {
            let handle = thread::spawn(move || {
                let _sorted_load = stats.get_sorted_load();
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
    });
}

// 158,278 ns/iter (+/- 117,103)
#[bench]
fn lockless_stats_good_load(b: &mut Bencher) {
    let stats = stats();
    let cores = placement::get_core_ids().expect("Core mapping couldn't be fetched");
    let total_cores = cores.len();
    const MAX_CORE: usize = 256;
    for i in 0..MAX_CORE {
        // Generating the best possible mergesort scenario
        // [0,1,2,3,4,5,6,7,8,9]...
        stats.store_load(i, i);
    }

    b.iter(|| {
        let mut handles = Vec::with_capacity(1000);

        for i in 0..100 {
            let handle = thread::spawn(move || {
                let _sorted_load = stats.get_sorted_load();
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
    });
}
