<div align="center">
  <img src="https://github.com/bastion-rs/bastion/blob/master/img/bastion.png"><br>
</div>

-----------------

<h1 align="center">Bastion: Fault-tolerant Runtime for Rust applications</h1>

*The breeze of the cold winter had come. This time of the year, Crashers arrive in the village. These are the giants who can't be evaded. They born to destroy and take people to the Paniks Heights. Suddenly, The developer descried the blurry silhouettes of Crashers from afar.*

*With cold and cracked voice, he whispered:*

*It is time to go to **Bastion Fort**.*

---

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
  <td>Build Status (Linux / MacOS)</td>
  <td>
    <a href="https://actions-badge.atrox.dev/bastion-rs/bastion/goto">
    <img alt="Build Status" src="https://img.shields.io/endpoint.svg?url=https%3A%2F%2Factions-badge.atrox.dev%2Fbastion-rs%2Fbastion%2Fbadge&style=flat" />
    </a>
  </td>
</tr>
<tr>
  <td>Build Status (Windows)</td>
  <td>
    <a href="https://ci.appveyor.com/project/vertexclique/bastion-4i0dk">
    <img src="https://ci.appveyor.com/api/projects/status/pbug23islg80de33/branch/master?svg=true" alt="appveyor build status" />
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

Bastion is a Fault-tolerant Runtime for Rust applications.
It detect panics during runs of your code and serves a runtime to
prevent abrupt exits. Also, it enables you to continue serving in case of
a failure. You can select your own recovery scenario, scale your workers and
define whole application on top of it. 

---

## Usage

Bastion comes with a default one-for-one strategy root supervisor.
You can use this to launch automatically supervised tasks.

## Why Bastion?
If one of the questions below answered with yes, then Bastion is just for you:
* Do I need fault-tolerancy in my project?
* Do I hate to implement weird Actor traits?
* I shouldn't need a webserver to run an actor system, right?
* Do I want to make my existing code unbreakable?
* Do I have some trust issues against orchestration systems? Because I want to implement my own application lifecycle.

## Features
* Message-based communication makes this project a lean mesh of actor system.
    * without web servers, weird shenanigans, forced trait implementations, and static dispatch.
* Runtime fault-tolerance makes it a good candidate for small scale distributed system code.
    * If you want to smell of Erlang and it's powerful aspects in Rust. That's it!
* Supervision makes it easy to manage lifecycles.
    * Kill your application in certain condition or restart you subprocesses whenever a certain condition met.
All up to you. And it should be up to you.

### Get Started
Check basic [root supervisor](https://github.com/bastion-rs/bastion/blob/master/examples/root_spv.rs) example in examples.

[Examples](https://github.com/bastion-rs/bastion/blob/master/examples) cover possible use cases in the frame of the crate.

Include bastion to your project with:
```toml
bastion = "0.2"
```

In most simple way you can use Bastion like here:
```rust
use bastion::prelude::*;

fn main() {
    Bastion::platform();

    // Define, calculate or prepare messages to be sent to the processes. 
    let message = String::from("Some message to be passed");

    Bastion::spawn(
        |context: BastionContext, msg: Box<dyn Message>| {
            // Message can be selected with receiver here. Take action!
            receive! { msg,
                String => |e| { println!("Received string :: {}", e)},
                i32 => |e| {println!("Received i32 :: {}", e)},
                _ => println!("No message as expected. Default")
            }

            // Do some processing in body
            println!("root supervisor - spawn_at_root - 1");

            // Rebind to the system
            context.hook();
        },
        message,
    );

    Bastion::start()
}
```

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
Please head to our [Discord](https://discord.gg/DqRqtRT) or use [StackOverflow](https://stackoverflow.com/questions/tagged/bastionframework)

## Discussion and Development
We use [Discord](https://discord.gg/DqRqtRT) for development discussions. Also please don't hesitate to open issues on GitHub ask for features, report bugs, comment on design and more!
More interaction and more ideas are better!

## Contributing to Bastion [![Open Source Helpers](https://www.codetriage.com/bastion-rs/bastion/badges/users.svg)](https://www.codetriage.com/bastion-rs/bastion)

All contributions, bug reports, bug fixes, documentation improvements, enhancements and ideas are welcome.

A detailed overview on how to contribute can be found in the  [CONTRIBUTING guide](.github/CONTRIBUTING.md) on GitHub.

### Thanks

Thanks to my dear mom (Günnur Bulut) who is an artist with many things to do but
spending her efforts without any hesitation on small things that I requested
(Like this logo). My shining star.

Also, thanks to my friend [Berkan Yavri](http://github.com/yavrib) who helped to the idea of making this.
Debated over the approaches that I took, spent time on thinking about this project with me.
