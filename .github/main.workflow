workflow "Gitter Release Notification" {
  resolves = ["Call Gitter"]
  on = "release"
}

action "Call Gitter" {
  uses = "swinton/httpie.action@69125d73caa2c6821f6a41a86112777a37adc171"
  secrets = ["TOKEN"]
  args = ["--auth-type=token", "--auth='bearer:$TOKEN'", "POST", "https://api.gitter.im/v1/rooms/bastionframework/community/chatMessages", "text=New version of Bastion released! Check out! https://crates.io/crates/bastion"]
}
