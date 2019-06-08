use crate::messages::*;
use crate::settings::Service;

use actix::prelude::*;
use actix::spawn;
use arraydeque::{ArrayDeque, Wrapping};
use failure::Fallible;
use futures::{Future, Stream};
use metrohash::MetroHashMap;

use std::io;
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock};
use tokio_process::{Child, CommandExt};


#[derive(Fail, Debug)]
pub enum ControllerError {
    #[fail(display = "Failed to load services from data, services already loaded!")]
    ServicesNotEmpty,
    #[fail(display = "Invalid instance ID: {}", _0)]
    InvalidInstance(usize),
    #[fail(display = "Unable to start, IO error: {}", _0)]
    StartupIOError(::std::io::Error),
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
        let services: Vec<Arc<Instance>> = data.into_iter().map(|d| Arc::new(d.into())).collect();
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
        match self.services.get(&msg.id) {
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

    fn handle(&mut self, msg: GetOutput, ctx: &mut Context<Self>) -> Self::Result {
        trace!("Getting latest output for {}", msg.id);
        if let Some(instance) = self.services.get(&msg.id) {
            let tty_r = instance.tty.read().expect("Can't read tty!");
            let msg = tty_r
                .iter()
                .map(|s| String::from_utf8_lossy(&s).into_owned())
                .collect::<Vec<_>>();
            let msg: String = msg.join("\n");
            Ok(msg)
        } else {
            Err(ControllerError::InvalidInstance(msg.id).into())
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

pub type LoadedService = Arc<Instance>;

struct Instance {
    model: Service,
    running: Arc<AtomicBool>,
    tty: Arc<RwLock<ArrayDeque<[Vec<u8>; 200], Wrapping>>>,
}

impl Instance {
    fn run(&self, addr: Addr<ServiceController>) -> Result<(), ::std::io::Error> {
        if self.model.enabled
            && !self
                .running
                .compare_and_swap(false, true, Ordering::Relaxed)
        {
            trace!("Starting {}", self.model.name);
            {
                let mut buffer_w = self.tty.write().expect("Can't write buffer!");
                buffer_w.push_back(format!("Starting {}", self.model.name).into_bytes());
                drop(buffer_w);
            }
            let mut cmd = Command::new(&self.model.command);
            //TODO: fix this to use better ENV
            // cmd.env_clear();
            cmd.args(&self.model.args);
            cmd.current_dir(&self.model.directory);
            cmd.stderr(Stdio::inherit());
            cmd.stdout(Stdio::piped());
            cmd.stdin(Stdio::null());
            let mut child = cmd.spawn_async()?;

            addr.do_send(ServiceStateChanged {
                id: self.model.id,
                running: true,
            });
            let stdout = child.stdout().take().unwrap();
            let reader = io::BufReader::new(stdout);
            let lines = crate::readline::lines(reader);
            let buffer_c = self.tty.clone();
            let cycle = lines.for_each(move |l| {
                let mut buffer_w = buffer_c.write().expect("Can't write buffer!");
                buffer_w.push_back(l);
                Ok(())
            });
            let buffer_c = self.tty.clone();
            let child = child.then(move |res| {
                let mut buffer_w = buffer_c.write().expect("Can't write buffer!");
                match res {
                    Ok(v) => {
                        buffer_w.push_back(format!("Process ended with signal {}", v).into_bytes());
                    }
                    Err(e) => {
                        buffer_w.push_back(format!("Unable to read exit state!").into_bytes());
                        warn!("Error reading process exit status: {}", e);
                    }
                }
                Ok(())
            });

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
                    Err(e) => error!("From child-fut: {}", e),
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
        }
    }
}
