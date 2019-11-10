<div align="center">
  <img src="https://github.com/bastion-rs/bastion/blob/master/img/bastion.png"><br>
</div>

-----------------

<h1 align="center">Highly-available Distributed Fault-tolerant Runtime</h1>

<table align=left style='float: left; margin: 4px 10px 0px 0px; border: 1px solid #000000;'>
<tr>
  <td>Latest Release</td>
  <td>
    <a href="https://crates.io/crates/bastion">
    <img alt="Crates.io" src="https://img.shields.io/crates/v/bastion.svg?style=popout-square">
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
    <a href="https://crates.io/crates/bastion">
    <img alt="Crates.io" src="https://img.shields.io/crates/d/bastion.svg?style=popout-square">
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

---

Bastion is a highly-available, fault-tolerant runtime system
with dynamic dispatch oriented lightweight process model.
It supplies actor model like concurrency with lightweight process implementation
and utilize all the system resources efficiently with giving promise of
at-most-once message delivery guarantee.

---

## Usage

Bastion comes with a default one-for-one strategy root supervisor.
You can use this to launch automatically supervised tasks.

## Features
* Message-based communication makes this project a lean mesh of actor system.
    * without web servers, weird shenanigans, forced trait implementations, and static dispatch.
* Runtime fault-tolerance makes it a good candidate for distributed systems.
    * If you want to smell of Erlang and it's powerful aspects in Rust. That's it!
* Completely asynchronous runtime with NUMA-aware and cache-affine SMP executor.
    * Exploiting hardware locality wherever it is possible. It is designed for servers.
* Supervision system makes it easy to manage lifecycles.
    * Kill your application in certain condition or restart you subprocesses whenever a certain condition met.

## Guarantees
* At most once delivery for all the messages.
* Completely asynchronous system design.
* Asynchronous program boundaries with [fort].
* Dynamic supervision of supervisors (adding a subtree later during the execution)
* Lifecycle management both at `futures` and `lightproc` layers.
* Faster middleware development.
* Above all "fault-tolerance".

## Why Bastion?
If one of the questions below answered with yes, then Bastion is just for you:
* Do I need fault-tolerancy in my project?
* Do I hate to implement weird Actor traits?
* I shouldn't need a webserver to run an actor system, right?
* Do I want to make my existing code unbreakable?
* Do I have some trust issues against orchestration systems? Because I want to implement my own application lifecycle.

### Get Started
Check [getting started example](https://github.com/bastion-rs/bastion/blob/master/examples/getting_started.rs) in examples.

[Examples](https://github.com/bastion-rs/bastion/blob/master/examples) cover possible use cases in the frame of the crate.

Include bastion to your project with:
```toml
bastion = "0.3"
```

For more information please check [Bastion Documentation](https://docs.rs/bastion)

## Architecture of the Runtime

Runtime is structured by the user. Only root supervision comes in batteries-included fashion.
Worker code, worker group redundancy, supervisors and their supervision strategies are defined by the user.

You can see architecture of the framework [HERE](https://github.com/bastion-rs/bastion/blob/master/img/bastion-arch.png). 

## Projects using Bastion
If you are using Bastion open a PR so we can include it in our showcase.
* [SkyNet](https://github.com/vertexclique/skynet) (a Discord bot which is resending deleted messages)
    * Skynet is running since 0.1.3 release of Bastion on the cloud and haven't killed yet.

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Documentation

Official documentation is hosted on [docs.rs](https://docs.rs/bastion).

## Getting Help
Please head to our [Discord](https://discord.gg/DqRqtRT) or use [StackOverflow](https://stackoverflow.com/questions/tagged/bastion)

## Discussion and Development
We use [Discord](https://discord.gg/DqRqtRT) for development discussions. Also please don't hesitate to open issues on GitHub ask for features, report bugs, comment on design and more!
More interaction and more ideas are better!

## Contributing to Bastion [![Open Source Helpers](https://www.codetriage.com/bastion-rs/bastion/badges/users.svg)](https://www.codetriage.com/bastion-rs/bastion)

All contributions, bug reports, bug fixes, documentation improvements, enhancements and ideas are welcome.

A detailed overview on how to contribute can be found in the  [CONTRIBUTING guide](.github/CONTRIBUTING.md) on GitHub.
