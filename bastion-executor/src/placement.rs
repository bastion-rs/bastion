/// This function tries to retrieve information
/// on all the "cores" active on this system.
pub fn get_core_ids() -> Option<Vec<CoreId>> {
    get_core_ids_helper()
}

pub fn set_for_current(core_id: CoreId) {
    set_for_current_helper(core_id);
}

#[derive(Copy, Clone, Debug)]
pub struct CoreId {
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
        use num_cpus;

        use super::*;

        #[test]
        fn test_linux_get_affinity_mask() {
            match get_affinity_mask() {
                Some(_) => {}
                None => {
                    assert!(false);
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
                    assert!(false);
                }
            }
        }

        #[test]
        fn test_linux_set_for_current() {
            let ids = get_core_ids().unwrap();

            assert!(ids.len() > 0);

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
extern crate kernel32;
#[cfg(target_os = "windows")]
extern crate winapi;

#[cfg(target_os = "windows")]
mod windows {
    use kernel32::{
        GetCurrentProcess, GetCurrentThread, GetProcessAffinityMask, SetThreadAffinityMask,
    };
    use winapi::basetsd::{DWORD_PTR, PDWORD_PTR};

    use super::CoreId;

    pub fn get_core_ids() -> Option<Vec<CoreId>> {
        if let Some(mask) = get_affinity_mask() {
            // Find all active cores in the bitmask.
            let mut core_ids: Vec<CoreId> = Vec::new();

            for i in 0..64 as u64 {
                let test_mask = 1 << i;

                if (mask & test_mask) == test_mask {
                    core_ids.push(CoreId { id: i as usize });
                }
            }

            Some(core_ids)
        } else {
            None
        }
    }

    pub fn set_for_current(core_id: CoreId) {
        // Convert `CoreId` back into mask.
        let mask: u64 = 1 << core_id.id;

        // Set core affinity for current thread.
        unsafe {
            SetThreadAffinityMask(GetCurrentThread(), mask as DWORD_PTR);
        }
    }

    fn get_affinity_mask() -> Option<u64> {
        #[cfg(target_pointer_width = "64")]
        let mut process_mask: u64 = 0;
        #[cfg(target_pointer_width = "32")]
        let mut process_mask: u32 = 0;
        #[cfg(target_pointer_width = "64")]
        let mut system_mask: u64 = 0;
        #[cfg(target_pointer_width = "32")]
        let mut system_mask: u32 = 0;

        let res = unsafe {
            GetProcessAffinityMask(
                GetCurrentProcess(),
                &mut process_mask as PDWORD_PTR,
                &mut system_mask as PDWORD_PTR,
            )
        };

        // Successfully retrieved affinity mask
        if res != 0 {
            Some(process_mask as u64)
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

    use num_cpus;

    use super::CoreId;

    type kern_return_t = c_int;
    type integer_t = c_int;
    type natural_t = c_uint;
    type thread_t = c_uint;
    type thread_policy_flavor_t = natural_t;
    type mach_msg_type_number_t = natural_t;

    #[repr(C)]
    struct thread_affinity_policy_data_t {
        affinity_tag: integer_t,
    }

    type thread_policy_t = *mut thread_affinity_policy_data_t;

    const THREAD_AFFINITY_POLICY: thread_policy_flavor_t = 4;

    #[link(name = "System", kind = "framework")]
    extern "C" {
        fn thread_policy_set(
            thread: thread_t,
            flavor: thread_policy_flavor_t,
            policy_info: thread_policy_t,
            count: mach_msg_type_number_t,
        ) -> kern_return_t;
    }

    pub fn get_core_ids() -> Option<Vec<CoreId>> {
        Some(
            (0..(num_cpus::get()))
                .into_iter()
                .map(|n| CoreId { id: n as usize })
                .collect::<Vec<_>>(),
        )
    }

    pub fn set_for_current(core_id: CoreId) {
        let THREAD_AFFINITY_POLICY_COUNT: mach_msg_type_number_t =
            mem::size_of::<thread_affinity_policy_data_t>() as mach_msg_type_number_t
                / mem::size_of::<integer_t>() as mach_msg_type_number_t;

        let mut info = thread_affinity_policy_data_t {
            affinity_tag: core_id.id as integer_t,
        };

        unsafe {
            thread_policy_set(
                pthread_self() as thread_t,
                THREAD_AFFINITY_POLICY,
                &mut info as thread_policy_t,
                THREAD_AFFINITY_POLICY_COUNT,
            );
        }
    }

    #[cfg(test)]
    mod tests {
        use num_cpus;

        use super::*;

        #[test]
        fn test_windows_get_core_ids() {
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
    use num_cpus;

    use super::*;

    #[test]
    fn test_get_core_ids() {
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
    fn test_set_for_current() {
        let ids = get_core_ids().unwrap();

        assert!(ids.len() > 0);

        set_for_current(ids[0]);
    }
}
