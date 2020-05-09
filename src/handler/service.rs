use super::error::*;
use crate::db::models::{LogAction, LogEntryResolved, NewLogEntry};
use crate::db::{DBInterface, DB};
use crate::handler::user::UserService;
use crate::messages::unchecked::*;
use crate::messages::*;
use crate::settings::Service;
use crate::web::models::SID;

use actix::fut::{err, ok, Either};
use actix::prelude::*;
use actix::spawn;
use arraydeque::{ArrayDeque, Wrapping};
use failure::Fallible;
use metrohash::MetroHashMap;
use serde::Serialize;
use strip_ansi_escapes as ansi_esc;
use tokio::io::AsyncWriteExt;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::Command;
use futures::stream::StreamExt;

use futures_util::future::TryFutureExt;

use futures::prelude::*;

use std::collections::{HashMap, HashSet};
use std::process::Stdio;
use std::sync::atomic::Ordering;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize};
use std::sync::{Arc, RwLock};

pub struct ServiceController {
    services: MetroHashMap<SID, LoadedService>,
}

impl Default for ServiceController {
    fn default() -> Self {
        Self {
            services: MetroHashMap::default(),
        }
    }
}

fn get_system_time_64() -> u64 {
    ::std::time::SystemTime::now()
        .duration_since(::std::time::UNIX_EPOCH)
        .expect("Invalid SystemTime!")
        .as_secs()
}

impl SystemService for ServiceController {}
impl Supervised for ServiceController {}

impl ServiceController {
    fn load_services(&mut self, data: Vec<Service>) -> Fallible<()> {
        trace!("Loading services");
        if !self.services.is_empty() {
            return Err(ControllerError::ServicesNotEmpty.into());
        }
        let services: Vec<Instance> = data.into_iter().map(|d| d.into()).collect();
        services.into_iter().for_each(|i| {
            Self::log(NewLogEntry::new(LogAction::SystemStartup, None), i.model.id);
            let _ = self.services.insert(i.model.id, i);
        });
        trace!("Loaded {} services",self.services.len());
        Ok(())
    }
    /// Wrapper to log to DB
    pub fn log(entry: NewLogEntry, sid: SID) {
        if let Err(e) = DB.insert_log_entry(sid, entry) {
            error!("Can't insert DB log entry! {}", e);
        }
    }
}

impl Actor for ServiceController {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        debug!("ServiceController is alive");
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        info!("ServiceController is stopped");
    }
}

impl Handler<StartService> for ServiceController {
    type Result = Result<(), ControllerError>;

    fn handle(&mut self, msg: StartService, ctx: &mut Context<Self>) -> Self::Result {
        trace!("Start received: {}", msg.id);
        match self.services.get_mut(&msg.id) {
            Some(instance) => {
                if instance.running.load(Ordering::SeqCst) {
                    return Err(ControllerError::ServiceRunning);
                }
                trace!("starting..");
                if let Err(e) = instance.run(ctx.address()) {
                    return Err(ControllerError::StartupIOError(e));
                }
                Self::log(
                    NewLogEntry::new(LogAction::ServiceCmdStart, msg.user),
                    msg.id,
                );
                trace!("started");
                Ok(())
            }
            None => Err(ControllerError::InvalidInstance(msg.id)),
        }
    }
}

impl Handler<SendStdin> for ServiceController {
    type Result = Result<(), ControllerError>;

    fn handle(&mut self, msg: SendStdin, _ctx: &mut Context<Self>) -> Self::Result {
        if let Some(service) = self.services.get_mut(&msg.id) {
            if !service.running.load(Ordering::Relaxed) {
                return Err(ControllerError::ServiceStopped);
            }
            if let Some(stdin) = service.stdin.as_mut() {
                match stdin.try_send(format!("{}\n", msg.input)) {
                    Ok(()) => {
                        Self::log(
                            NewLogEntry::new(LogAction::Stdin(msg.input), msg.user),
                            msg.id,
                        );
                        return Ok(());
                    }
                    Err(e) => {
                        warn!("Unable to send message to {} {}", service.model.name, e);
                        return Err(ControllerError::BrokenPipe);
                    }
                }
            }
            Err(ControllerError::NoServiceHandle)
        } else {
            Err(ControllerError::InvalidInstance(msg.id))
        }
    }
}

impl Handler<KillService> for ServiceController {
    type Result = Result<(), ControllerError>;

    fn handle(&mut self, msg: KillService, _ctx: &mut Context<Self>) -> Self::Result {
        if let Some(service) = self.services.get_mut(&msg.id) {
            if let Some(v) = service.kill_handle.take() {
                let _ = v.send(());
                Self::log(
                    NewLogEntry::new(LogAction::ServiceCmdKilled, msg.user),
                    msg.id,
                );
                return Ok(());
            }
            Err(ControllerError::NoServiceHandle)
        } else {
            Err(ControllerError::InvalidInstance(msg.id))
        }
    }
}

impl Handler<StopService> for ServiceController {
    type Result = Result<(), ControllerError>;

    fn handle(&mut self, msg: StopService, _ctx: &mut Context<Self>) -> Self::Result {
        if let Some(service) = self.services.get_mut(&msg.id) {
            if !service.running.load(Ordering::Acquire) {
                return Err(ControllerError::ServiceStopped);
            }
            let stdin = match service.stdin.as_mut() {
                Some(stdin) => stdin,
                None => return Err(ControllerError::NoServiceHandle),
            };
            let stop_msg = match service.model.soft_stop.as_ref() {
                Some(stop_msg) => stop_msg,
                None => return Err(ControllerError::NoSoftStop),
            };
            if let Err(e) = stdin.try_send(format!("{}\n", stop_msg)) {
                warn!("Can't soft-stop process: {}", e);
            }
            Self::log(
                NewLogEntry::new(LogAction::ServiceCmdStop, msg.user),
                msg.id,
            );
            service.state.set_state(State::Stopping);
            Ok(())
        } else {
            Err(ControllerError::InvalidInstance(msg.id))
        }
    }
}

impl Handler<ServiceStateChanged> for ServiceController {
    type Result = ();
    fn handle(&mut self, msg: ServiceStateChanged, ctx: &mut Context<Self>) {
        if let Some(instance) = self.services.get_mut(&msg.id) {
            let state = instance.state.get_state();

            let log_action = match state {
                State::Ended => LogAction::ServiceEnded,
                State::Running => LogAction::ServiceStarted,
                State::Crashed => {
                    LogAction::ServiceCrashed(instance.crash_code.load(Ordering::Acquire))
                }
                State::Stopped => LogAction::ServiceStopped,
                State::Killed => LogAction::ServiceKilled,
                State::Stopping => {
                    unreachable!("unreachable: service-stopping-state in state update!")
                }
            };
            Self::log(NewLogEntry::new(log_action, None), msg.id);

            if !msg.running {
                instance.end_time = Some(get_system_time_64());

                let restart = if instance.model.restart_always && state == State::Ended {
                    true
                } else {
                    instance.model.restart && state == State::Crashed
                };

                if restart {
                    ctx.address().do_send(StartService {
                        id: instance.model.id,
                        user: None,
                    });
                } else {
                    // cleanup
                    instance.kill_handle = None;
                    instance.stdin = None;
                }
            }
        }
    }
}

impl Handler<GetOutput> for ServiceController {
    type Result = Result<Vec<ConsoleType<String>>, ControllerError>;

    fn handle(&mut self, msg: GetOutput, _ctx: &mut Context<Self>) -> Self::Result {
        if let Some(instance) = self.services.get(&msg.id) {
            let tty_r = instance.tty.read().expect("Can't read tty!");
            let msg = tty_r
                .iter()
                .map(|s| match s {
                    ConsoleType::State(s) => {
                        ConsoleType::State(String::from_utf8_lossy(&s).into_owned())
                    }
                    ConsoleType::Stderr(s) => {
                        ConsoleType::Stderr(String::from_utf8_lossy(&s).into_owned())
                    }
                    ConsoleType::Stdout(s) => {
                        ConsoleType::Stdout(String::from_utf8_lossy(&s).into_owned())
                    }
                    ConsoleType::Stdin(s) => {
                        ConsoleType::Stdin(String::from_utf8_lossy(&s).into_owned())
                    }
                })
                .collect::<Vec<_>>();
            Ok(msg)
        } else {
            Err(ControllerError::InvalidInstance(msg.id))
        }
    }
}

impl Handler<GetUserServicePermsAll> for ServiceController {
    type Result = Result<HashMap<SID, SPMin>, ControllerError>;

    fn handle(&mut self, msg: GetUserServicePermsAll, _ctx: &mut Context<Self>) -> Self::Result {
        let mut data = HashMap::with_capacity(self.services.len());
        self.services.iter().for_each(|(k, v)| {
            data.insert(
                k.clone(),
                SPMin {
                    id: *k,
                    name: v.model.name.clone(),
                    has_perm: false,
                },
            );
        });
        DB.get_all_perm_service(msg.user)?
            .iter()
            .for_each(|(k, v)| {
                if let Some(mut entry) = data.get_mut(k) {
                    entry.has_perm = !v.is_empty();
                }
            });
        Ok(data)
    }
}

impl Handler<GetServiceIDs> for ServiceController {
    type Result = Result<Vec<SID>, ControllerError>;

    fn handle(&mut self, _msg: GetServiceIDs, _ctx: &mut Context<Self>) -> Self::Result {
        Ok(self.services.values().map(|v| v.model.id).collect())
    }
}

impl Handler<GetSessionServices> for ServiceController {
    type Result = ResponseActFuture<Self, Result<Vec<ServiceState>, ControllerError>>;
    // kind of breaks separation of concerns, but to make this performant we'll have to call UserService from here
    fn handle(&mut self, msg: GetSessionServices, _ctx: &mut Context<Self>) -> Self::Result {
        let fut = UserService::from_registry()
            .send(GetSessionServiceIDs {
                session: msg.session,
            })
            .map_err(ControllerError::from);
        let fut = actix::fut::wrap_future::<_, Self>(fut);
        let fut = fut.then(|v, actor, _ctx| {
            // tame Result<Result<K,V>> with early return
            let v: Vec<SID> = match v.map_err(ControllerError::from) {
                Err(e) => return Either::Right(err(e)),
                Ok(Err(e)) => return Either::Right(err(e.into())),
                Ok(Ok(v)) => v,
            };
            let mut services = HashSet::with_capacity(v.len());
            services.extend(v);

            Either::Left(ok(actor
                .services
                .values()
                .filter_map(|v| {
                    if services.contains(&v.model.id) {
                        Some(ServiceState {
                            id: v.model.id,
                            name: v.model.name.clone(),
                            state: v.state.get_state(),
                            uptime: v.uptime(),
                        })
                    } else {
                        None
                    }
                })
                .collect()))
        });
        Box::new(fut)
    }
}

impl Handler<GetAllServicesMin> for ServiceController {
    type Result = Result<Vec<ServiceMin>, ControllerError>;
    fn handle(&mut self, _msg: GetAllServicesMin, _ctx: &mut Context<Self>) -> Self::Result {
        Ok(self
            .services
            .values()
            .map(|v| ServiceMin {
                id: v.model.id,
                name: v.model.name.clone(),
            })
            .collect())
    }
}

impl Handler<GetServiceState> for ServiceController {
    type Result = Result<ServiceState, ControllerError>;
    fn handle(&mut self, msg: GetServiceState, _ctx: &mut Context<Self>) -> Self::Result {
        if let Some(v) = self.services.get(&msg.id) {
            Ok(ServiceState {
                id: msg.id,
                name: v.model.name.clone(),
                state: v.state.get_state(),
                uptime: v.uptime(),
            })
        } else {
            Err(ControllerError::InvalidInstance(msg.id))
        }
    }
}

impl Handler<GetLogLatest> for ServiceController {
    type Result = Result<Vec<LogEntryResolved>, ControllerError>;
    fn handle(&mut self, msg: GetLogLatest, _ctx: &mut Context<Self>) -> Self::Result {
        // TODO: refactor, should we directly call the DB from the web API?
        if self.services.get(&msg.id).is_some() {
            Ok(DB.service_log_limited(msg.id, msg.amount)?)
        } else {
            Err(ControllerError::InvalidInstance(msg.id))
        }
    }
}

impl Handler<LoadServices> for ServiceController {
    type Result = ();
    fn handle(&mut self, msg: LoadServices, ctx: &mut Context<Self>) {
        if self.load_services(msg.data).is_ok() {
            for (key, val) in self.services.iter() {
                if val.model.autostart {
                    trace!("Autostarting {}", key);
                    let key = *key;
                    spawn(
                        ctx.address()
                            .send(StartService {
                                id: key,
                                user: None,
                            })
                            .map(move |v| {
                                if let Err(e) = v {
                                    error!("Starting instance {}: {}", key, e);
                                }
                            }),
                    );
                }
            }
        }
    }
}

type LoadedService = Instance;

struct Instance {
    model: Service,
    running: Arc<AtomicBool>,
    tty: Arc<RwLock<ArrayDeque<[ConsoleType<Vec<u8>>; 2048], Wrapping>>>,
    state: StateFlag,
    crash_code: Arc<AtomicI32>,
    kill_handle: Option<tokio::sync::oneshot::Sender<()>>,
    stdin: Option<tokio::sync::mpsc::Sender<String>>,
    start_time: Option<u64>,
    end_time: Option<u64>,
}

#[derive(Serialize)]
pub enum ConsoleType<T> {
    Stdin(T),
    Stdout(T),
    Stderr(T),
    State(T),
}

#[derive(PartialEq, Serialize)]
pub enum State {
    Stopped = 0,
    Running = 1,
    Ended = 2,
    Crashed = 3,
    Stopping = 4,
    Killed = 5,
}

// derived from https://gist.github.com/polypus74/eabc7bb00873e6b90abe230f9e632989
#[derive(Clone)]
pub struct StateFlag {
    inner: Arc<AtomicUsize>,
}

impl StateFlag {
    pub fn new(state: State) -> Self {
        StateFlag {
            inner: Arc::new(AtomicUsize::new(state as usize)),
        }
    }

    #[inline]
    pub fn get_state(&self) -> State {
        self.inner.load(Ordering::SeqCst).into()
    }
    pub fn set_state(&self, state: State) {
        self.inner.store(state as usize, Ordering::SeqCst)
    }
}

impl From<usize> for State {
    fn from(val: usize) -> Self {
        use self::State::*;
        match val {
            0 => Stopped,
            1 => Running,
            2 => Ended,
            3 => Crashed,
            4 => Stopping,
            5 => Killed,
            _ => unreachable!(),
        }
    }
}

impl Instance {
    fn uptime(&self) -> u64 {
        let subtrahend = match self.end_time {
            Some(v) => v,
            None => get_system_time_64(),
        };
        self.start_time.as_ref().map_or(0, |v| subtrahend - v)
    }
    /// Run instance, outer catch function to log startup errors to tty
    fn run(&mut self, addr: Addr<ServiceController>) -> Result<(), ::std::io::Error> {
        let res = self.run_internal(addr);
        if let Err(e) = &res {
            let mut buffer_w = self.tty.write().expect("Can't write buffer!");
            buffer_w.push_back(ConsoleType::State(
                format!("Can't start instance: {}", e).into_bytes(),
            ));
            drop(buffer_w);
            ServiceController::log(
                NewLogEntry::new(LogAction::ServiceStartFailed(format!("{}", e)), None),
                self.model.id,
            );
        }
        res
    }
    /// real service starter
    fn run_internal(&mut self, addr: Addr<ServiceController>) -> Result<(), ::std::io::Error> {
        if self.model.enabled
            && !self
                .running
                .compare_and_swap(false, true, Ordering::Acquire)
        {
            trace!("Starting {}", self.model.name);
            {
                let mut buffer_w = self.tty.write().expect("Can't write buffer!");
                buffer_w.push_back(ConsoleType::State(
                    format!("Starting {}", self.model.name).into_bytes(),
                ));
                drop(buffer_w);
            }
            let mut cmd = Command::new(&self.model.command);
            //TODO: fix this to use better ENV
            // cmd.env_clear();
            cmd.kill_on_drop(true);
            cmd.args(&self.model.args);
            cmd.current_dir(&self.model.directory);
            cmd.stderr(Stdio::piped());
            cmd.stdout(Stdio::piped());
            cmd.stdin(Stdio::piped());
            self.state.set_state(State::Running);
            let mut child = cmd.spawn()?;
            self.start_time = Some(get_system_time_64());
            self.end_time = None;

            addr.do_send(ServiceStateChanged {
                id: self.model.id,
                running: true,
            });

            let service_info = format!("{}-{}", self.model.id, self.model.name);

            let mut stdin = child.stdin.take().unwrap();
            let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(16);
            let buffer_c = self.tty.clone();
            // handle stdin
            let stdin_fut = async move {
                while let Some(msg) = rx.recv().await {
                    let buffer_c2 = buffer_c.clone();
                    let buffer_c3 = buffer_c.clone();
                    let service_info = service_info.clone();
                    match stdin.write_all(msg.as_bytes()).await {
                        Ok(()) => {
                            let mut buffer_w = buffer_c2.write().expect("Can't write buffer!");
                            buffer_w.push_back(ConsoleType::Stdin(msg.into_bytes()));
                        },
                        Err(e) => {
                            error!("Couldn't write to stdin of {}: {}", service_info, e);
                            let mut buffer_w = buffer_c3.write().expect("Can't write buffer!");
                            buffer_w.push_back(ConsoleType::State(
                                format!("Couldn't write to stdout! \"{}\"", msg).into_bytes(),
                            ));
                        }
                    }
                }
            };
            self.stdin = Some(tx);

            let buffer_c = self.tty.clone();
            let stdout = child.stdout.take().unwrap();
            // handle stdout
            let stdout_fut = async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                while let Some(l) = lines.next().await {
                    match l {
                        Err(e) => error!("Error handling stdout: {}", e),
                        Ok(line) => {
                            let mut buffer_w = buffer_c.write().expect("Can't write buffer!");
                            buffer_w.push_back(ConsoleType::Stdout(ansi_esc::strip(line).unwrap()));
                        }
                    }
                }
            };

            let buffer_c = self.tty.clone();
            let stderr = child.stderr.take().unwrap();
            // handle stderr
            let stderr_fut = async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Some(l) = lines.next().await {
                    match l {
                        Ok(line) => {
                            let mut buffer_w = buffer_c.write().expect("Can't write buffer!");
                            buffer_w.push_back(ConsoleType::Stderr(ansi_esc::strip(line).unwrap()));
                        },
                        Err(e) => error!("Error handling stderr: {}", e),
                    }
                }
            };

            let buffer_c = self.tty.clone();
            let state_c = self.state.clone();
            let crash_code = self.crash_code.clone();
            // handle child exit-return
            let child_fut = async move {
                let result = child.await;
                let mut buffer_w = buffer_c.write().expect("Can't write buffer!");
                match result {
                    Ok(state) => {
                        #[cfg(target_family = "unix")]
                        let code_formated = sysexit::from_status(state);
                        #[cfg(target_family = "windows")]
                        let code_formated = "";
                        buffer_w.push_back(ConsoleType::State(
                            format!("Process ended with signal {}({:?})", state, code_formated)
                                .into_bytes(),
                        ));
                        if let Some(code) = state.code() {
                            crash_code.store(code, Ordering::Release);
                        }
                        match state_c.get_state() {
                            State::Running => {
                                if state.success() {
                                    state_c.set_state(State::Ended);
                                } else {
                                    state_c.set_state(State::Crashed);
                                }
                            }
                            State::Stopping => {
                                state_c.set_state(State::Stopped);
                            }
                            // should we override anyway ?
                            _ => (),
                        }
                    }
                    Err(e) => {
                        buffer_w.push_back(ConsoleType::State(
                            "Unable to read exit state!".to_string().into_bytes(),
                        ));
                        state_c.set_state(State::Crashed);
                        warn!("Error reading process exit status: {}", e);
                    }
                }
            };

            // kill-switch handling
            let buffer_c = self.tty.clone();
            let state_c = self.state.clone();
            let (tx, rx) = tokio::sync::oneshot::channel::<()>();
            let exit_fut = async move {
                tokio::select! {
                    _ = child_fut => (),
                    _ = rx.map_err(|_| ()).map(move |_| {
                            state_c.set_state(State::Killed);
                            let mut buffer_w = buffer_c.write().expect("Can't write buffer!");
                            buffer_w.push_back(ConsoleType::State(
                                String::from("Process killed").into_bytes(),
                            ));
                        }) => (),
                }
            };
            self.kill_handle = Some(tx);

            // future end handler, will always trigger
            // regardless of kill or process end
            let name_c = self.model.name.clone();
            let running_c = self.running.clone();
            let id_c = self.model.id;
            spawn(async move {
                let _ = tokio::join!(
                    exit_fut,
                    stdin_fut,
                    stderr_fut,
                    stdout_fut
                );
                running_c.store(false, Ordering::Relaxed);
                addr.do_send(ServiceStateChanged {
                    id: id_c,
                    running: false,
                });
                trace!("Service {} stopped", name_c);
                // match result {
                //     Ok(_) => trace!("Service {} stopped", name_c),
                //     Err(_) => error!("Error in child-fut"),
                // }
            });
        } else {
            trace!("Ignoring startup of {}, already running!", self.model.name);
        }
        Ok(())
    }
}

impl From<Service> for Instance {
    fn from(service: Service) -> Self {
        Self {
            model: service,
            running: Arc::new(AtomicBool::new(false)),
            tty: Arc::new(RwLock::new(ArrayDeque::new())),
            state: StateFlag::new(State::Stopped),
            kill_handle: None,
            crash_code: Arc::new(AtomicI32::new(0)),
            stdin: None,
            start_time: None,
            end_time: None,
        }
    }
}
