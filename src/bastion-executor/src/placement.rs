//!
//! Core placement configuration and management
//!
//! Placement module enables thread placement onto the cores.
//! CPU level affinity assignment is done here.

/// This function tries to retrieve information
/// on all the "cores" active on this system.
pub fn get_core_ids() -> Option<Vec<CoreId>> {
    get_core_ids_helper()
}

/// This function tries to retrieve
/// the number of active "cores" on the system.
pub fn get_num_cores() -> Option<usize> {
    get_core_ids().map(|ids| ids.len())
}
///
/// Sets the current threads affinity
pub fn set_for_current(core_id: CoreId) {
    tracing::info!(
        "Bastion-executor: placement: set affinity on core {}",
        core_id.id
    );
    set_for_current_helper(core_id);
}

///
/// CoreID implementation to identify system cores.
#[derive(Copy, Clone, Debug)]
pub struct CoreId {
    /// Used core ID
    pub id: usize,
}

// Linux Section

#[cfg(target_os = "linux")]
#[inline]
fn get_core_ids_helper() -> Option<Vec<CoreId>> {
    linux::get_core_ids()
}

#[cfg(target_os = "linux")]
#[inline]
fn set_for_current_helper(core_id: CoreId) {
    linux::set_for_current(core_id);
}

#[cfg(target_os = "linux")]
mod linux {
    use std::mem;

    use libc::{cpu_set_t, sched_getaffinity, sched_setaffinity, CPU_ISSET, CPU_SET, CPU_SETSIZE};

    use super::CoreId;

    pub fn get_core_ids() -> Option<Vec<CoreId>> {
        if let Some(full_set) = get_affinity_mask() {
            let mut core_ids: Vec<CoreId> = Vec::new();

            for i in 0..CPU_SETSIZE as usize {
                if unsafe { CPU_ISSET(i, &full_set) } {
                    core_ids.push(CoreId { id: i });
                }
            }

            Some(core_ids)
        } else {
            None
        }
    }

    pub fn set_for_current(core_id: CoreId) {
        // Turn `core_id` into a `libc::cpu_set_t` with only
        // one core active.
        let mut set = new_cpu_set();

        unsafe { CPU_SET(core_id.id, &mut set) };

        // Set the current thread's core affinity.
        unsafe {
            sched_setaffinity(
                0, // Defaults to current thread
                mem::size_of::<cpu_set_t>(),
                &set,
            );
        }
    }

    fn get_affinity_mask() -> Option<cpu_set_t> {
        let mut set = new_cpu_set();

        // Try to get current core affinity mask.
        let result = unsafe {
            sched_getaffinity(
                0, // Defaults to current thread
                mem::size_of::<cpu_set_t>(),
                &mut set,
            )
        };

        if result == 0 {
            Some(set)
        } else {
            None
        }
    }

    fn new_cpu_set() -> cpu_set_t {
        unsafe { mem::zeroed::<cpu_set_t>() }
    }

    #[cfg(test)]
    mod tests {

        use super::*;

        #[test]
        fn test_linux_get_affinity_mask() {
            match get_affinity_mask() {
                Some(_) => {}
                None => {
                    panic!();
                }
            }
        }

        #[test]
        fn test_linux_get_core_ids() {
            match get_core_ids() {
                Some(set) => {
                    assert_eq!(set.len(), num_cpus::get());
                }
                None => {
                    panic!();
                }
            }
        }

        #[test]
        fn test_linux_set_for_current() {
            let ids = get_core_ids().unwrap();

            assert!(!ids.is_empty());

            set_for_current(ids[0]);

            // Ensure that the system pinned the current thread
            // to the specified core.
            let mut core_mask = new_cpu_set();
            unsafe { CPU_SET(ids[0].id, &mut core_mask) };

            let new_mask = get_affinity_mask().unwrap();

            let mut is_equal = true;

            for i in 0..CPU_SETSIZE as usize {
                let is_set1 = unsafe { CPU_ISSET(i, &core_mask) };
                let is_set2 = unsafe { CPU_ISSET(i, &new_mask) };

                if is_set1 != is_set2 {
                    is_equal = false;
                }
            }

            assert!(is_equal);
        }
    }
}

// Windows Section

#[cfg(target_os = "windows")]
#[inline]
fn get_core_ids_helper() -> Option<Vec<CoreId>> {
    windows::get_core_ids()
}

#[cfg(target_os = "windows")]
#[inline]
fn set_for_current_helper(core_id: CoreId) {
    windows::set_for_current(core_id);
}

#[cfg(target_os = "windows")]
extern crate winapi;

#[cfg(target_os = "windows")]
mod windows {
    #[allow(unused_imports)]
    use winapi::shared::basetsd::{DWORD_PTR, PDWORD_PTR};
    use winapi::um::processthreadsapi::{GetCurrentProcess, GetCurrentThread};
    use winapi::um::winbase::{GetProcessAffinityMask, SetThreadAffinityMask};

    use super::CoreId;

    pub fn get_core_ids() -> Option<Vec<CoreId>> {
        if let Some(mask) = get_affinity_mask() {
            // Find all active cores in the bitmask.
            let mut core_ids: Vec<CoreId> = Vec::new();

            for i in 0..usize::MIN.count_zeros() as usize {
                let test_mask = 1 << i;

                if (mask & test_mask) == test_mask {
                    core_ids.push(CoreId { id: i });
                }
            }

            Some(core_ids)
        } else {
            None
        }
    }

    pub fn set_for_current(core_id: CoreId) {
        // Convert `CoreId` back into mask.
        let mask: DWORD_PTR = 1 << core_id.id;

        // Set core affinity for current thread.
        unsafe {
            SetThreadAffinityMask(GetCurrentThread(), mask);
        }
    }

    fn get_affinity_mask() -> Option<usize> {
        let mut process_mask: usize = 0;
        let mut system_mask: usize = 0;

        let res = unsafe {
            GetProcessAffinityMask(
                GetCurrentProcess(),
                &mut process_mask as PDWORD_PTR,
                &mut system_mask as PDWORD_PTR,
            )
        };

        // Successfully retrieved affinity mask
        if res != 0 {
            Some(process_mask)
        }
        // Failed to retrieve affinity mask
        else {
            None
        }
    }

    #[cfg(test)]
    mod tests {
        use num_cpus;

        use super::*;

        #[test]
        fn test_macos_get_core_ids() {
            match get_core_ids() {
                Some(set) => {
                    assert_eq!(set.len(), num_cpus::get());
                }
                None => {
                    assert!(false);
                }
            }
        }

        #[test]
        fn test_macos_set_for_current() {
            let ids = get_core_ids().unwrap();

            assert!(ids.len() > 0);

            set_for_current(ids[0]);
        }
    }
}

// MacOS Section

#[cfg(target_os = "macos")]
#[inline]
fn get_core_ids_helper() -> Option<Vec<CoreId>> {
    macos::get_core_ids()
}

#[cfg(target_os = "macos")]
#[inline]
fn set_for_current_helper(core_id: CoreId) {
    macos::set_for_current(core_id);
}

#[cfg(target_os = "macos")]
mod macos {
    use std::mem;

    use libc::{c_int, c_uint, pthread_self};

    use super::CoreId;

    type KernReturnT = c_int;
    type IntegerT = c_int;
    type NaturalT = c_uint;
    type ThreadT = c_uint;
    type ThreadPolicyFlavorT = NaturalT;
    type MachMsgTypeNumberT = NaturalT;

    #[repr(C)]
    struct ThreadAffinityPolicyDataT {
        affinity_tag: IntegerT,
    }

    type ThreadPolicyT = *mut ThreadAffinityPolicyDataT;

    const THREAD_AFFINITY_POLICY: ThreadPolicyFlavorT = 4;

    #[link(name = "System", kind = "framework")]
    extern "C" {
        fn thread_policy_set(
            thread: ThreadT,
            flavor: ThreadPolicyFlavorT,
            policy_info: ThreadPolicyT,
            count: MachMsgTypeNumberT,
        ) -> KernReturnT;
    }

    pub fn get_core_ids() -> Option<Vec<CoreId>> {
        Some(
            (0..(num_cpus::get()))
                .map(|n| CoreId { id: n as usize })
                .collect::<Vec<_>>(),
        )
    }

    pub fn set_for_current(core_id: CoreId) {
        let thread_affinity_policy_count: MachMsgTypeNumberT =
            mem::size_of::<ThreadAffinityPolicyDataT>() as MachMsgTypeNumberT
                / mem::size_of::<IntegerT>() as MachMsgTypeNumberT;

        let mut info = ThreadAffinityPolicyDataT {
            affinity_tag: core_id.id as IntegerT,
        };

        unsafe {
            thread_policy_set(
                pthread_self() as ThreadT,
                THREAD_AFFINITY_POLICY,
                &mut info as ThreadPolicyT,
                thread_affinity_policy_count,
            );
        }
    }

    #[cfg(test)]
    mod tests {

        use super::*;

        #[test]
        fn test_windows_get_core_ids() {
            match get_core_ids() {
                Some(set) => {
                    assert_eq!(set.len(), num_cpus::get());
                }
                None => {
                    panic!();
                }
            }
        }

        #[test]
        fn test_windows_set_for_current() {
            let ids = get_core_ids().unwrap();

            assert!(ids.len() > 0);

            set_for_current(ids[0]);
        }
    }
}

// Stub Section

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
#[inline]
fn get_core_ids_helper() -> Option<Vec<CoreId>> {
    None
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
#[inline]
fn set_for_current_helper(core_id: CoreId) {}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_get_core_ids() {
        match get_core_ids() {
            Some(set) => {
                assert_eq!(set.len(), num_cpus::get());
            }
            None => {
                panic!();
            }
        }
    }

    #[test]
    fn test_set_for_current() {
        let ids = get_core_ids().unwrap();

        assert!(!ids.is_empty());

        set_for_current(ids[0]);
    }
}
