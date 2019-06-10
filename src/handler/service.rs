use crate::messages::*;
use crate::settings::Service;

use actix::prelude::*;
use actix::spawn;
use arraydeque::{ArrayDeque, Wrapping};
use failure::Fallible;
use metrohash::MetroHashMap;
use tokio_process::{Child, CommandExt};

use futures::sync::mpsc::TrySendError;
use futures::{self, Future, Stream};

use std::io;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock};

#[derive(Fail, Debug)]
pub enum ControllerError {
    #[fail(display = "Failed to load services from data, services already loaded!")]
    ServicesNotEmpty,
    #[fail(display = "Invalid instance ID: {}", _0)]
    InvalidInstance(usize),
    #[fail(display = "Unable to start, IO error: {}", _0)]
    StartupIOError(::std::io::Error),
    #[fail(display = "Service is stopped!")]
    ServiceStopped,
    #[fail(display = "Unable to execute, missing service handles!")]
    NoServiceHandle,
}

pub struct ServiceController {
    services: MetroHashMap<usize, LoadedService>,
}

impl Default for ServiceController {
    fn default() -> Self {
        Self {
            services: MetroHashMap::default(),
        }
    }
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

    fn started(&mut self, ctx: &mut Context<Self>) {
        debug!("ServiceController is alive");
    }

    fn stopped(&mut self, ctx: &mut Context<Self>) {
        info!("ServiceController is stopped");
    }
}

impl Handler<StartService> for ServiceController {
    type Result = Result<(), ControllerError>;

    fn handle(&mut self, msg: StartService, ctx: &mut Context<Self>) -> Self::Result {
        trace!("Start received: {}", msg.id);
        match self.services.get_mut(&msg.id) {
            Some(instance) => {
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

impl Handler<StopService> for ServiceController {
    type Result = Result<(), ControllerError>;

    fn handle(&mut self, msg: StopService, _ctx: &mut Context<Self>) -> Self::Result {
        if let Some(mut service) = self.services.get_mut(&msg.id) {
            if service.running.load(Ordering::Relaxed) {
                return Err(ControllerError::ServiceStopped.into());
            }
            service.state = State::Stopped;
            // TODO: actually stop it
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
            if instance.model.restart && !msg.running {
                ctx.address().do_send(StartService {
                    id: instance.model.id,
                });
            }
        }
    }
}

impl Handler<GetOutput> for ServiceController {
    type Result = Result<String, ControllerError>;

    fn handle(&mut self, msg: GetOutput, _ctx: &mut Context<Self>) -> Self::Result {
        trace!("Getting latest output for {}", msg.id);
        if let Some(instance) = self.services.get(&msg.id) {
            let tty_r = instance.tty.read().expect("Can't read tty!");
            let msg = tty_r
                .iter()
                .map(|s| match s {
                    MessageType::State(s) => {
                        format!("STATE: {}", String::from_utf8_lossy(&s).into_owned())
                    }
                    MessageType::Stderr(s) => {
                        format!("STDERR: {}", String::from_utf8_lossy(&s).into_owned())
                    }
                    MessageType::Stdout(s) => {
                        format!("STDOUT: {}", String::from_utf8_lossy(&s).into_owned())
                    }
                    MessageType::Stdin(s) => {
                        format!("STDIN: {}", String::from_utf8_lossy(&s).into_owned())
                    }
                })
                .collect::<Vec<_>>();
            let msg: String = msg.join("\n");
            Ok(msg)
        } else {
            Err(ControllerError::InvalidInstance(msg.id).into())
        }
    }
}

impl Handler<GetServices> for ServiceController {
    type Result = Result<Vec<ServiceMin>, ControllerError>;

    fn handle(&mut self, _msg: GetServices, _ctx: &mut Context<Self>) -> Self::Result {
        Ok(self
            .services
            .values()
            .map(|v| ServiceMin {
                id: v.model.id,
                name: v.model.name.clone(),
                running: v.running.load(Ordering::Relaxed),
            })
            .collect())
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

pub type LoadedService = Instance;

struct Instance {
    model: Service,
    running: Arc<AtomicBool>,
    tty: Arc<RwLock<ArrayDeque<[MessageType; 200], Wrapping>>>,
    state: State,
    stop_channel: Option<futures::sync::oneshot::Sender<()>>,
    stdin: Option<futures::sync::mpsc::Sender<String>>,
}

pub enum MessageType {
    Stdin(Vec<u8>),
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
    State(Vec<u8>),
}

#[derive(PartialEq)]
pub enum State {
    Stopped,
    Crashed,
    Ended,
    Running,
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
                buffer_w.push_back(MessageType::State(
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
            let mut child = cmd.spawn_async()?;

            addr.do_send(ServiceStateChanged {
                id: self.model.id,
                running: true,
            });

            let service_info = format!("{}-{}", self.model.id, self.model.name);

            // handle stdin
            let mut stdin = child.stdin().take().unwrap();
            let (tx, rx) = futures::sync::mpsc::channel::<String>(16);
            let buffer_c = self.tty.clone();
            let fut_stdin = rx.for_each(move |msg| {
                let bytes = msg.as_bytes();
                match stdin.write_all(bytes) {
                    Err(e) => {
                        error!("Couldn't write to stdin of {}: {}", service_info, e);
                        let mut buffer_w = buffer_c.write().expect("Can't write buffer!");
                        buffer_w.push_back(MessageType::State(
                            format!("Couldn't write to stdout! \"{}\"", msg).into_bytes(),
                        ));
                    }
                    Ok(v) => {
                        let mut buffer_w = buffer_c.write().expect("Can't write buffer!");
                        buffer_w.push_back(MessageType::Stdin(msg.into_bytes()));
                    }
                }
                Ok(())
            });
            spawn(fut_stdin);
            self.stdin = Some(tx);

            // handle stdout
            let stdout = child.stdout().take().unwrap();
            let reader = io::BufReader::new(stdout);
            let lines = crate::readline::lines(reader);
            let buffer_c = self.tty.clone();
            let cycle = lines
                .for_each(move |l| {
                    let mut buffer_w = buffer_c.write().expect("Can't write buffer!");
                    buffer_w.push_back(MessageType::Stdout(l));
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
            let cycle = lines
                .for_each(move |l| {
                    let mut buffer_w = buffer_c.write().expect("Can't write buffer!");
                    buffer_w.push_back(MessageType::Stderr(l));
                    Ok(())
                })
                .map_err(|e| {
                    warn!("From child-future: {}", e);
                    ()
                });

            // handle child exit-return
            let buffer_c = self.tty.clone();
            let child = child.then(move |res| {
                let mut buffer_w = buffer_c.write().expect("Can't write buffer!");
                match res {
                    Ok(v) => {
                        buffer_w.push_back(MessageType::State(
                            format!("Process ended with signal {}", v).into_bytes(),
                        ));
                        Ok(())
                    }
                    Err(e) => {
                        buffer_w.push_back(MessageType::State(
                            format!("Unable to read exit state!").into_bytes(),
                        ));
                        warn!("Error reading process exit status: {}", e);
                        Err(())
                    }
                }
            });

            // stop-handle
            let (tx, rx) = futures::sync::oneshot::channel::<()>();
            let child = child.select(rx.map_err(|_| ())).then(|_x| Ok(()));
            self.stop_channel = Some(tx);

            let name_c = self.model.name.clone();
            let running_c = self.running.clone();
            let id_c = self.model.id.clone();
            let future = cycle.join(child).then(move |result| {
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
            state: State::Stopped,
            stop_channel: None,
            stdin: None,
        }
    }
}
