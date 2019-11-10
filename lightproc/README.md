# LightProc

LightProc is Lightweight Process abstraction for Rust.

Beneath the implementation:
* It uses futures with lifecycle callbacks to implement Erlang like processes.
* Contains basic pid(process id) to identify processes.
* All panics inside futures are propagated to upper layers. 
