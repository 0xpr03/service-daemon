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
