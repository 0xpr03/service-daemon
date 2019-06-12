### Service-Daemon

***Currently in development and not production ready!***

Rust based process controller allowing to start/stop/input processes via your browser.

- Autostart
- Stdin/Stdout/Stderr control via browser
- Auto-Restart
- View exit codes etc
- 2FA Authentification

### Why ?

Letting clients restart their own services without full access to the machine and investigate crashes.
Starting/Stopping stuff that doesn't have to be up all day.
Can be used to monitor new software, unstable services etc.

### But how secure is it ?

- You need 2 factor authentification to sign in.
- Service configuration (start command, parameters..) are not configurable from the web-interface, only via config files.  
  This has some drawbacks but lessens the attack surface drastically.
- You can decide to allow only stdout-inspection or certain commands (not yet implemented).
- It's written in rust, making it more resiliant to typical buffer overflows.
