# Bastion Executor

<table align=left style='float: left; margin: 4px 10px 0px 0px; border: 1px solid #000000;'>
<tr>
  <td>Latest Release</td>
  <td>
    <a href="https://crates.io/crates/bastion">
    <img alt="Crates.io" src="https://img.shields.io/crates/v/bastion-executor.svg?style=popout-square">
    </a>
  </td>
</tr>
<tr>
  <td></td>
</tr>
<tr>
  <td>License</td>
  <td>
    <a href="https://github.com/bastion-rs/bastion/blob/master/LICENSE">
    <img alt="Crates.io" src="https://img.shields.io/crates/l/bastion.svg?style=popout-square">
    </a>
</td>
</tr>
<tr>
  <td>Build Status</td>
  <td>
    <a href="https://actions-badge.atrox.dev/bastion-rs/bastion/goto">
    <img alt="Build Status" src="https://img.shields.io/endpoint.svg?url=https%3A%2F%2Factions-badge.atrox.dev%2Fbastion-rs%2Fbastion%2Fbadge&style=flat" />
    </a>
  </td>
</tr>
<tr>
  <td>Downloads</td>
  <td>
    <a href="https://crates.io/crates/bastion-executor">
    <img alt="Crates.io" src="https://img.shields.io/crates/d/bastion-executor.svg?style=popout-square">
    </a>
  </td>
</tr>
<tr>
	<td>Discord</td>
	<td>
		<a href="https://discord.gg/DqRqtRT">
		<img src="https://img.shields.io/discord/628383521450360842.svg?logo=discord" />
		</a>
	</td>
</tr>
</table>

Bastion Executor is NUMA-aware SMP based Fault-tolerant Executor

Bastion Executor is a highly-available, fault-tolerant, async communication
oriented executor. Bastion's main idea is supplying a fully async runtime
with fault-tolerance to work on heavy loads.

Main differences between other executors are:
* Uses SMP based execution scheme to exploit cache affinity on multiple cores and execution is
equally distributed over the system resources, which means utilizing the all system.
* Uses NUMA-aware allocation for scheduler's queues and exploit locality on server workloads.
* Tailored for creating middleware and working with actor model like concurrency and distributed communication.

**NOTE:** Bastion Executor is independent of it's framework implementation.
It uses [lightproc](https://docs.rs/lightproc) to encapsulate and provide fault-tolerance to your future based workloads.
You can use your futures with [lightproc](https://docs.rs/lightproc) to run your workloads on Bastion Executor without the need to have framework.

## Example Usage

```rust
use bastion_executor::prelude::*;
use lightproc::proc_stack::ProcStack;

fn main() {
    let pid = 1;
    let stack = ProcStack::default()
        .with_pid(pid)
        .with_after_panic(move || println!("after panic {}", pid.clone()));

    let handle = spawn(
        async {
            panic!("test");
        },
        stack,
    );

    let pid = 2;
    let stack = ProcStack::default().with_pid(pid);

    run(
        async {
            handle.await;
        },
        stack.clone(),
    );
}
```