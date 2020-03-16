#![feature(test)]

extern crate test;
use bastion_executor::load_balancer::{SmpStats, stats, lockless_stats}; 
use std::thread;
use bastion_executor::placement;

fn stress_stats<S: SmpStats + Sync + Send>(stats: &'static S){
    let cores = placement::get_core_ids().expect("Core mapping couldn't be fetched");
    let mut handles = Vec::new();
    for core in cores{
        let handle = thread::spawn(move ||{
            placement::set_for_current(core);
            for i in 0..100{
                stats.store_load(core.id, 10);
                if i % 3 == 0{
                    let _sorted_load = stats.get_sorted_load();
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles{
        handle.join().unwrap();
    }
}
use test::Bencher;

// 1,352,791 ns/iter (+/- 2,682,013)
#[bench]
fn locked_stats_bench(b: &mut Bencher) {
    b.iter(|| {
        stress_stats(stats());
    });
}

// 158,278 ns/iter (+/- 117,103)
#[bench]
fn lockless_stats_bench(b: &mut Bencher) {
    b.iter(|| {
        stress_stats(lockless_stats());
    });
}
