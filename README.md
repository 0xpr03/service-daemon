### Service-Daemon

***Currently in development and not production ready!***

Rust based process controller allowing to start/stop/input processes via your browser.

 [ ] Web Interface
 [ ] 2FA Authentification
 [X] Autostart
 [X] Stdout & Stderr
 [X] Stdin control
 [X] Start/Stop
 [X] Auto-Restart
 [X] View exit codes etc

### Why ?

Letting clients restart their own services without full access to the machine and investigate crashes.
Starting/Stopping stuff that doesn't have to be up all day.
Can be used to monitor new software, unstable services etc.

### But how secure is it ?

- You need 2 factor authentification to sign in.
- Service configuration (start command, parameters..) are not configurable from the web-interface, only via config files.  
  This has some usability drawbacks but decreases the attack surface drastically.
- You can decide to allow only stdout-inspection or certain commands (not yet implemented).
- It's written in rust, making it more resiliant to typical buffer overflows.
