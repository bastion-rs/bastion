workflow "Gitter Release Notification" {
  resolves = ["Call Gitter"]
  on = "release"
}

action "Call Gitter" {
  uses = "swinton/httpie.action@master"
  secrets = ["TOKEN"]
  args = ["--auth-type=jwt", "--auth=$TOKEN", "POST", "https://api.gitter.im/v1/rooms/5d34b303d73408ce4fc69801/chatMessages", "text=New\\ version\\ of\\ Bastion\\ released!\\ Check\\ out!\\ https://crates.io/crates/bastion"]
}
