### Service-Daemon

***Currently in development and not production ready!***

Process controller allowing to start/stop/input processes via your browser, in async Rust.

- [ ] Web Interface
- [X] 2FA Authentification
- [X] Autostart
- [X] Stdout & Stderr
- [X] Stdin control
- [X] Start/Stop
- [X] Auto-Restart
- [X] View exit codes etc
- [ ] Command-Preset

### Navigation

- [Why](#why)
- [Security concerncs](#but-how-secure-is-it-)
- [Setup](#setup)
- [Building](#building)
- [Contributing](#contributing)

### Why ?

To let clients investigate crashes and restart their own services without giving full access to the machine.
As well as starting/stopping stuff that doesn't have to be up all day.
And to monitor new software, unstable services etc.

### But how secure is it ?

- You need 2 factor authentification to sign in.
- Service configuration (start command, parameters..) are not configurable from the web-interface, only via config files.  
  This has some usability drawbacks but decreases the attack surface drastically.
- You can disable stdin globally for a service.
- It's written in rust, making it more resiliant to typical buffer overflows.

### Setup

- Build: First of all you will need to go through the [building](#building) section.
- First Run: After this you run the program, which will setup the root account and print the login credentials.
- Setup 2FA: Now you login with those credentials and setup TOTP (for example , google authenticator, 1Password)
- Configure: in /config/default.toml you can now specify your services. Please restart service-daemon to apply those changes.

### Building

Fetch the repo
```
$ git clone https://github.com/0xpr03/service-daemon
$ cd service-daemon
```

service-daemon is written in Rust, so you'll need to grab a
[Rust installation](https://www.rust-lang.org/) in order to compile it.
service-daemon compiles with Rust 1.34.0 (stable) or newer.

To build the backend in release mode:

```
$ cargo build --release
```

To build the frontend you first need [npm & nodejs](https://nodejs.org/en/), then run

```
$ npm run build
```

### Contributing

You can contribute code by opening PRs or interacting with issues.

To contribute code you will need the setup for Building and use
```
$ npm start
```
to start a live frontend instance.

And 
```
$ cargo run
```
to start the backend from your current code.
