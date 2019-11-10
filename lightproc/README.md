# LightProc


<table align=left style='float: left; margin: 4px 10px 0px 0px; border: 1px solid #000000;'>
<tr>
  <td>Latest Release</td>
  <td>
    <a href="https://crates.io/crates/lightproc">
    <img alt="Crates.io" src="https://img.shields.io/crates/v/lightproc.svg?style=popout-square">
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
    <a href="https://crates.io/crates/lightproc">
    <img alt="Crates.io" src="https://img.shields.io/crates/d/lightproc.svg?style=popout-square">
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

LightProc is Lightweight Process abstraction for Rust.

Beneath the implementation:
* It uses futures with lifecycle callbacks to implement Erlang like processes.
* Contains basic pid(process id) to identify processes.
* All panics inside futures are propagated to upper layers. 
