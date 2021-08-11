### Service-Daemon [![Build Status](https://travis-ci.com/0xpr03/service-daemon.svg?branch=master)](https://travis-ci.com/0xpr03/service-daemon)

***Currently in development and not production ready!***

Process daemon allowing to start/stop/input processes via your browser, in async Rust.

- [ ] Web Interface
  - [ ] Serverside-Push of changes
  - [X] Inspect console
  - [X] Start/Stop/Kill of services
  - [X] User Management
- [X] 2FA Authentification
- [X] Autostart
- [X] Stdout & Stderr
- [X] Stdin control
- [X] Start/Stop
- [X] Auto-Restart
- [X] View exit codes etc
- [x] Log of service state changes
- [X] Stdout/err snapshots on crash/stop
- [ ] Command-Preset
- [X] Built-In DB (users,state,logs)
- [ ] DBMS support (mariadb,mysql)

### Navigation

- [Why](#why-)
- [Security concerncs](#but-how-secure-is-it-)
- [Caveats](#caveats)
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

### Caveats

- SD has no mechanic internally for dropping privileges and thus running services as a different user, except for providing a bash script and running SD itself as root. This is *not* adivced to do!
- Thus SD has to run as the same user its service should run, which imposes a certain security risk based on your application. In general you should not run untrusted software with SD. You can mitigate this by running [systemd nspawn](https://wiki.debian.org/nspawn) or docker containers via SD, lessening some risks.

### Setup

- Build: First of all you will need to go through the [building](#building) section.
- Configure: copy config/template.toml to config/services.toml, you can now specify your services inside it. Please restart SD to apply changes.  
  You can also configure everything via ENV variables by prepending `sd__`. For example `sd__web_bind_port=9000`.
- First Run: After this you run the program, which will setup the root account and print the login credentials. `cargo run --release` or run the executable in target/release/service-daemon
- Setup 2FA: Now you login with those credentials and setup TOTP (for example andOTP, google authenticator, 1Password)
- To run SD on system startup in systemd (*nix) you can use the provided `service-daemon.service` file, edit paths accordingly and move it to /etc/systemd/systemd. Now run `systemctl daemon-reload` to reload systemd config. Then `systemctl enable service-daemon.service` to enable auto-start and `systemctl start service-daemon.service` to run it now.

### Building

Fetch the repo
```
$ git clone https://github.com/0xpr03/service-daemon
$ cd service-daemon
```

service-daemon is written in Rust, so you'll need to grab a
[Rust installation](https://www.rust-lang.org/) in order to compile it.
service-daemon compiles with Rust 1.41.1 stable or newer.

To build the backend in release mode:

```
$ cargo build --release
```

To build the frontend you first need [npm & nodejs](https://nodejs.org/en/), then run inside the `frontend` dir:

```
$ npm install
$ npm run build
```

If you want to try out the test configuration run also
```
$ cargo build --examples
```
This will generate the example services specified in `config/default.toml`
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
