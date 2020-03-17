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
use test::Bencher;

// previous lock based stats benchmark 1,352,791 ns/iter (+/- 2,682,013)

// 158,278 ns/iter (+/- 117,103)
#[bench]
fn lockless_stats_bench(b: &mut Bencher) {
    b.iter(|| {
        stress_stats(stats());
    });
}
