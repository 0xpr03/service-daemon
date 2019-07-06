use super::error::ControllerError;
use crate::handler::user::UserService;
use crate::messages::*;
use crate::settings::Service;
use crate::web::models::SID;

use strip_ansi_escapes as ansi_esc;
use actix::fut::{err, ok, Either};
use actix::prelude::*;
use actix::spawn;
use arraydeque::{ArrayDeque, Wrapping};
use failure::Fallible;
use metrohash::MetroHashMap;
use serde::Serialize;
use tokio_io::io::write_all;
use tokio_process::CommandExt;

use futures::{self, Future, Stream};

use std::collections::HashSet;
use std::io;
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;
use std::sync::atomic::{AtomicBool, AtomicUsize};
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
        if self.services.len() != 0 {
            return Err(ControllerError::ServicesNotEmpty.into());
        }
        let services: Vec<Instance> = data.into_iter().map(|d| d.into()).collect();
        services.into_iter().for_each(|i| {
            let _ = self.services.insert(i.model.id, i);
        });
        Ok(())
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
                    return Err(ControllerError::ServiceRunning.into());
                }
                trace!("starting..");
                if let Err(e) = instance.run(ctx.address()) {
                    error!("Can't start instance: {}", e);
                    return Err(ControllerError::StartupIOError(e).into());
                }
                trace!("started");
                Ok(())
            }
            None => Err(ControllerError::InvalidInstance(msg.id).into()),
        }
    }
}

impl Handler<SendStdin> for ServiceController {
    type Result = Result<(), ControllerError>;

    fn handle(&mut self, msg: SendStdin, _ctx: &mut Context<Self>) -> Self::Result {
        if let Some(service) = self.services.get_mut(&msg.id) {
            if !service.running.load(Ordering::Relaxed) {
                return Err(ControllerError::ServiceStopped.into());
            }
            if let Some(stdin) = service.stdin.as_mut() {
                match stdin.try_send(format!("{}\n",msg.input)) {
                    Ok(()) => return Ok(()),
                    Err(e) => {
                        warn!("Unable to send message to {} {}", service.model.name, e);
                        return Err(ControllerError::BrokenPipe.into());
                    }
                }
            }
            Err(ControllerError::NoServiceHandle.into())
        } else {
            Err(ControllerError::InvalidInstance(msg.id).into())
        }
    }
}

impl Handler<StopService> for ServiceController {
    type Result = Result<(), ControllerError>;

    fn handle(&mut self, msg: StopService, _ctx: &mut Context<Self>) -> Self::Result {
        if let Some(service) = self.services.get_mut(&msg.id) {
            if !service.running.load(Ordering::Relaxed) {
                return Err(ControllerError::ServiceStopped.into());
            }
            service.state.set_state(State::Stopped);
            service.stdin = None;
            if let Some(stdin) = service.stdin.as_mut() {
                if let Some(stop_msg) = service.model.soft_stop.as_ref() {
                    match stdin.try_send(stop_msg.clone()) {
                        Err(e) => warn!("Can't soft-stop process: {}", e),
                        Ok(_) => return Ok(()),
                    }
                }
            }
            if let Some(v) = service.stop_channel.take() {
                let _ = v.send(());
                return Ok(());
            }
            Err(ControllerError::NoServiceHandle.into())
        } else {
            Err(ControllerError::InvalidInstance(msg.id).into())
        }
    }
}

impl Handler<ServiceStateChanged> for ServiceController {
    type Result = ();
    fn handle(&mut self, msg: ServiceStateChanged, ctx: &mut Context<Self>) {
        if let Some(instance) = self.services.get(&msg.id) {
            if instance.model.restart
                && !msg.running
                && instance.state.get_state() == State::Crashed
            {
                ctx.address().do_send(StartService {
                    id: instance.model.id,
                });
            }
        }
    }
}

impl Handler<GetOutput> for ServiceController {
    type Result = Result<Vec<LogType<String>>, ControllerError>;

    fn handle(&mut self, msg: GetOutput, _ctx: &mut Context<Self>) -> Self::Result {
        if let Some(instance) = self.services.get(&msg.id) {
            let tty_r = instance.tty.read().expect("Can't read tty!");
            let msg = tty_r
                .iter()
                .map(|s| match s {
                    LogType::State(s) => {
                        LogType::State(String::from_utf8_lossy(&s).into_owned())
                    }
                    LogType::Stderr(s) => {
                        LogType::Stderr(String::from_utf8_lossy(&s).into_owned())
                    }
                    LogType::Stdout(s) => {
                        LogType::Stdout(String::from_utf8_lossy(&s).into_owned())
                    }
                    LogType::Stdin(s) => {
                        LogType::Stdin(String::from_utf8_lossy(&s).into_owned())
                    }
                })
                .collect::<Vec<_>>();
            Ok(msg)
        } else {
            Err(ControllerError::InvalidInstance(msg.id).into())
        }
    }
}

impl Handler<GetServiceIDs> for ServiceController {
    type Result = Result<Vec<SID>, ControllerError>;

    fn handle(&mut self, _msg: GetServiceIDs, _ctx: &mut Context<Self>) -> Self::Result {
        Ok(self.services.values().map(|v| v.model.id.clone()).collect())
    }
}

impl Handler<GetUserServices> for ServiceController {
    type Result = ResponseActFuture<Self, Vec<ServiceMin>, ControllerError>;
    // kind of breaks separation of concerns, but to make this performant we'll have to call UserService from here
    fn handle(&mut self, msg: GetUserServices, _ctx: &mut Context<Self>) -> Self::Result {
        let fut = UserService::from_registry()
            .send(GetUserServiceIDs {
                session: msg.session,
            })
            .map_err(ControllerError::from);
        let fut = actix::fut::wrap_future::<_, Self>(fut);
        let fut = fut.and_then(|v, actor, _ctx| {
            let v = match v.map_err(ControllerError::from) {
                Err(e) => return Either::B(err(e)),
                Ok(v) => v,
            };
            let mut services = HashSet::with_capacity(v.len());
            services.extend(v);

            Either::A(ok(actor
                .services
                .values()
                .filter_map(|v| {
                    if services.contains(&v.model.id) {
                        Some(ServiceMin {
                            id: v.model.id,
                            name: v.model.name.clone(),
                            running: v.running.load(Ordering::Relaxed),
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

impl Handler<GetServiceState> for ServiceController {
    type Result = Result<ServiceState, ControllerError>;
    fn handle(&mut self, msg: GetServiceState, _ctx: &mut Context<Self>) -> Self::Result {
        if let Some(v) = self.services.get(&msg.id) {
            Ok(ServiceState {
                name: v.model.name.clone(),
                state: v.state.get_state(),
                uptime: v
                    .start_time
                    .as_ref()
                    .map_or(0, |v| get_system_time_64() - v),
            })
        } else {
            Err(ControllerError::InvalidInstance(msg.id))
        }
    }
}

impl Handler<LoadServices> for ServiceController {
    type Result = ();
    fn handle(&mut self, msg: LoadServices, ctx: &mut Context<Self>) {
        if let Ok(_) = self.load_services(msg.data) {
            for (key, val) in self.services.iter() {
                if val.model.autostart {
                    trace!("Autostarting {}", key);
                    spawn(
                        ctx.address()
                            .send(StartService { id: key.clone() })
                            .map(|v| debug!("{:?}", v))
                            .map_err(|e| panic!("{}", e)),
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
    tty: Arc<RwLock<ArrayDeque<[LogType<Vec<u8>>; 2048], Wrapping>>>,
    state: StateFlag,
    stop_channel: Option<futures::sync::oneshot::Sender<()>>,
    stdin: Option<futures::sync::mpsc::Sender<String>>,
    start_time: Option<u64>,
}

#[derive(Serialize)]
pub enum LogType<T> {
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
            _ => unreachable!(),
        }
    }
}

impl Instance {
    fn run(&mut self, addr: Addr<ServiceController>) -> Result<(), ::std::io::Error> {
        if self.model.enabled
            && !self
                .running
                .compare_and_swap(false, true, Ordering::Relaxed)
        {
            trace!("Starting {}", self.model.name);
            {
                let mut buffer_w = self.tty.write().expect("Can't write buffer!");
                buffer_w.push_back(LogType::State(
                    format!("Starting {}", self.model.name).into_bytes(),
                ));
                drop(buffer_w);
            }
            let mut cmd = Command::new(&self.model.command);
            //TODO: fix this to use better ENV
            // cmd.env_clear();
            cmd.args(&self.model.args);
            cmd.current_dir(&self.model.directory);
            cmd.stderr(Stdio::piped());
            cmd.stdout(Stdio::piped());
            cmd.stdin(Stdio::piped());
            self.state.set_state(State::Running);
            let mut child = cmd.spawn_async()?;
            self.start_time = Some(get_system_time_64());

            addr.do_send(ServiceStateChanged {
                id: self.model.id,
                running: true,
            });

            let service_info = format!("{}-{}", self.model.id, self.model.name);

            // handle stdin
            let stdin = child.stdin().take().unwrap();
            let (tx, rx) = futures::sync::mpsc::channel::<String>(16);
            let buffer_c = self.tty.clone();
            let fut_stdin = rx
                .fold(stdin, move |stdin, msg| {
                    let bytes = msg.clone().into_bytes();
                    let buffer_c2 = buffer_c.clone();
                    let buffer_c3 = buffer_c.clone();
                    let service_info = service_info.clone();
                    write_all(stdin, bytes)
                        .map(move |(stdin, res)| {
                            let mut buffer_w = buffer_c2.write().expect("Can't write buffer!");
                            buffer_w.push_back(LogType::Stdin(res));

                            stdin
                        })
                        .map_err(move |e| {
                            error!("Couldn't write to stdin of {}: {}", service_info, e);
                            let mut buffer_w = buffer_c3.write().expect("Can't write buffer!");
                            buffer_w.push_back(LogType::State(
                                format!("Couldn't write to stdout! \"{}\"", msg).into_bytes(),
                            ));
                            ()
                        })
                })
                .map(|_| ());
            spawn(fut_stdin);
            self.stdin = Some(tx);

            // handle stdout
            let stdout = child.stdout().take().unwrap();
            let reader = io::BufReader::new(stdout);
            let lines = crate::readline::lines(reader);
            let buffer_c = self.tty.clone();
            let cycle_stdout = lines
                .for_each(move |l| {
                    let mut buffer_w = buffer_c.write().expect("Can't write buffer!");
                    buffer_w.push_back(LogType::Stdout(ansi_esc::strip(l).unwrap()));
                    Ok(())
                })
                .map_err(|e| {
                    warn!("From child-future: {}", e);
                    ()
                });

            // handle stderr
            let stderr = child.stderr().take().unwrap();
            let reader = io::BufReader::new(stderr);
            let lines = crate::readline::lines(reader);
            let buffer_c = self.tty.clone();
            let cycle_stderr = lines
                .for_each(move |l| {
                    let mut buffer_w = buffer_c.write().expect("Can't write buffer!");
                    buffer_w.push_back(LogType::Stderr(ansi_esc::strip(l).unwrap()));
                    Ok(())
                })
                .map_err(|e| {
                    warn!("From child-future: {}", e);
                    ()
                });

            // handle child exit-return
            let buffer_c = self.tty.clone();
            let state_c = self.state.clone();
            let child = child.then(move |res| {
                let mut buffer_w = buffer_c.write().expect("Can't write buffer!");
                match res {
                    Ok(state) => {
                        let code_formated = sysexit::from_status(state.clone());
                        buffer_w.push_back(LogType::State(
                            format!("Process ended with signal {}({:?})", state, code_formated)
                                .into_bytes(),
                        ));
                        if state_c.get_state() == State::Running {
                            if state.success() {
                                state_c.set_state(State::Ended);
                            } else {
                                state_c.set_state(State::Crashed);
                            }
                        }
                        Ok(())
                    }
                    Err(e) => {
                        buffer_w.push_back(LogType::State(
                            format!("Unable to read exit state!").into_bytes(),
                        ));
                        state_c.set_state(State::Crashed);
                        warn!("Error reading process exit status: {}", e);
                        Err(())
                    }
                }
            });

            // stop-handle
            let buffer_c = self.tty.clone();
            let (tx, rx) = futures::sync::oneshot::channel::<()>();
            let child = child
                .select(rx.map_err(|_| ()).map(move |_| {
                    let mut buffer_w = buffer_c.write().expect("Can't write buffer!");
                    buffer_w.push_back(LogType::State(
                        String::from("Process killed").into_bytes(),
                    ));
                }))
                .then(|_x| Ok(()));
            self.stop_channel = Some(tx);

            let name_c = self.model.name.clone();
            let running_c = self.running.clone();
            let id_c = self.model.id.clone();
            let future = child.join3(cycle_stdout, cycle_stderr).then(move |result| {
                running_c.store(false, Ordering::Relaxed);
                addr.do_send(ServiceStateChanged {
                    id: id_c,
                    running: false,
                });
                match result {
                    Ok(_) => trace!("Service {} stopped", name_c),
                    Err(_) => error!("Error in child-fut"),
                }
                Ok(())
            });
            spawn(future);
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
            stop_channel: None,
            stdin: None,
            start_time: None,
        }
    }
}
