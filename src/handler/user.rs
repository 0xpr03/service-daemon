use super::error::{StartupError, UserError};
use super::messages::unchecked::*;
use super::messages::*;
use crate::crypto::*;
use crate::db;
use crate::db::{models::*, DBInterface, DB};
use crate::handler::service::ServiceController;
use crate::web::models::{CreateUserResp, LoginState, NewUser, UserMin, UID};
use actix;
use actix::fut::*;
use actix::prelude::*;
use actix_threadpool::run as blocking;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::iter;

/// Initial root user
const ROOT_NAME: &'static str = "Root";
const ROOT_EMAIL: &'static str = "root@localhost";
const ROOT_PASSWORD_LENGTH: usize = 20;
const CLEANUP_INTERVAL: u64 = 60 * 20; // seconds

/// Service for handling user related things
pub struct UserService {
    brcypt_cost: u32,
    login_max_age: u32,
}

type Result<T> = ::std::result::Result<T, UserError>;

impl UserService {
    /// Returns UID for session if currently fully logged in
    fn get_session_uid(&self, session: &str) -> Result<UID> {
        use db::models::LoginState as DBLoginState;
        match DB.get_login(session, self.login_max_age)? {
            Some(v) => match v.state {
                DBLoginState::Complete => Ok(v.id),
                _ => Err(UserError::InvalidSession),
            },
            None => Err(UserError::InvalidSession),
        }
    }
    /// Insert full service perms for admins
    ///
    /// We could also just check the admin state, but this would require a second lookup
    fn setup_admin_permissions(&self) {
        trace!("Setting up admin permissions");
        let fut = ServiceController::from_registry()
            .send(GetServiceIDs {})
            .map_err(StartupError::from)
            .and_then(|response| match response {
                Ok(services) => {
                    for user in DB.get_perm_admin().map_err(UserError::from)? {
                        for service in services.iter() {
                            DB.set_perm_service(user, *service, ServicePerm::all())
                                .map_err(UserError::from)?;
                        }
                    }
                    Ok(())
                }
                Err(e) => Err(e.into()),
            })
            .map_err(|e| {
                error!("Unable to initialize admin permissions, aborting! {}", e);
                panic!("Can't init admin permissions, aborting");
            });
        actix::spawn(fut);
    }
    /// Delete old sessions
    fn cleanup_sessions(&mut self, _context: &mut Context<Self>) {
        match DB.delete_old_logins(self.login_max_age) {
            Ok(v) => debug!("Removed {} outdated logins.", v),
            Err(e) => warn!("Unable to remove old logins: {}", e),
        }
    }
    fn is_admin(&self, user: UID) -> Result<bool> {
        Ok(DB.get_user(user)?.admin)
    }
    /// Check if user is admin, errors otherwise
    fn check_admin(&self, user: UID) -> Result<()> {
        match self.is_admin(user)? {
            true => Ok(()),
            false => Err(UserError::InvalidPermissions),
        }
    }
    fn get_root_user(&self) -> Result<Option<FullUser>> {
        match DB.get_user(DB.get_root_id()) {
            Ok(user) => Ok(Some(user)),
            Err(db::Error::InvalidUser(_)) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    fn create_user_unchecked(&self, user: NewUser) -> Result<CreateUserResp> {
        let user_enc = NewUserEnc {
            email: user.email,
            name: user.name,
            password_enc: bcrypt_password(&user.password, self.brcypt_cost)?,
        };
        let user = DB.create_user(user_enc)?;
        Ok(CreateUserResp { user: user.id })
    }
}

impl Default for UserService {
    fn default() -> Self {
        Self {
            brcypt_cost: 12,
            login_max_age: 3600,
        }
    }
}

impl Handler<StartupCheck> for UserService {
    type Result = Result<()>;

    fn handle(&mut self, _msg: StartupCheck, _ctx: &mut Context<Self>) -> Result<()> {
        let user = self.get_root_user()?;
        let create_root = user.is_none();
        if let Some(user) = user {
            if !user.totp_complete {
                warn!("2FA setup incomplete for root!");
            }
        }

        if create_root {
            // TODO: use chacha20 once rand is compatible again with rand_chacha
            let mut rng = thread_rng();
            let password: String = iter::repeat(())
                .map(|()| rng.sample(Alphanumeric))
                .take(ROOT_PASSWORD_LENGTH)
                .collect();
            let start = std::time::Instant::now();
            match self.create_user_unchecked(NewUser {
                name: ROOT_NAME.to_string(),
                password: password.clone(),
                email: ROOT_EMAIL.to_string(),
            }) {
                Ok(v) => {
                    let uid = v.user;
                    assert_eq!(uid, DB.get_root_id());
                    let end = start.elapsed().as_millis();

                    let mut user_full = DB.get_user(uid)?;
                    user_full.admin = true;
                    DB.update_user(user_full)?;

                    if end > 2_000 {
                        warn!(
                            "Took {} ms to encrypt password using current configuration!",
                            end
                        );
                    } else {
                        info!("Took {} ms to encrypt password.", end)
                    }
                    info!("Created root user:");
                    info!("Email: {} Passwort: {}", ROOT_EMAIL, password);
                    info!("Please login to setup 2FA!");
                }
                Err(e) => {
                    return Err(UserError::InternalError(format!(
                        "Couldn't create root user: {:?}",
                        e
                    )))
                }
            }
        }

        self.setup_admin_permissions();
        Ok(())
    }
}

impl Handler<LoginTOTP> for UserService {
    type Result = Result<LoginState>;

    fn handle(&mut self, msg: LoginTOTP, _ctx: &mut Context<Self>) -> Self::Result {
        let mut login = match DB.get_login(&msg.session, self.login_max_age)? {
            Some(v) => v,
            None => return Ok(LoginState::NotLoggedIn),
        };
        let mut user = DB.get_user(login.id)?;
        let expected_totp = totp_calculate(&user.totp);
        if expected_totp == msg.totp {
            login.state = db::models::LoginState::Complete;
            DB.set_login(&msg.session, Some(login))?;
            let user_min = UserMin::from(&user);
            if !user.totp_complete {
                user.totp_complete = true;
                DB.update_user(user)?;
            }
            Ok(LoginState::LoggedIn(user_min))
        } else {
            Ok(match user.totp_complete {
                true => LoginState::RequiresTOTP,
                false => LoginState::RequiresTOTPSetup(user.totp.into()),
            })
        }
    }
}

impl Handler<LoginUser> for UserService {
    type Result = ResponseActFuture<Self, LoginState, UserError>;

    fn handle(&mut self, msg: LoginUser, _ctx: &mut Context<Self>) -> Self::Result {
        if let Err(e) = DB.set_login(&msg.session, None) {
            return Box::new(err(e.into()));
        }
        let uid = match DB.get_id_by_email(&msg.email) {
            Ok(Some(v)) => v,
            Ok(None) => return Box::new(ok(LoginState::NotLoggedIn)),
            Err(e) => return Box::new(err(e.into())),
        };
        let user = match DB.get_user(uid) {
            Ok(u) => u,
            Err(e) => return Box::new(err(e.into())),
        };

        let fut =
            blocking(move || bcrypt_verify(&msg.password, &user.password).map(|v| (v, msg, user)));
        let fut = actix::fut::wrap_future::<_, Self>(fut)
            .map_err(|e, _, _| e.into())
            .and_then(move |(v, msg, user), _, _| {
                if v {
                    let state = match user.totp_complete {
                        true => db::models::LoginState::Missing2Fa,
                        false => db::models::LoginState::Requires2FaSetup,
                    };
                    if let Err(e) = DB.set_login(&msg.session, Some(ActiveLogin { state, id: uid }))
                    {
                        return Either::B(err(e.into()));
                    }
                    if user.totp_complete {
                        Either::A(Either::A(ok(LoginState::RequiresTOTP)))
                    } else {
                        Either::A(Either::B(ok(LoginState::RequiresTOTPSetup(
                            user.totp.into(),
                        ))))
                    }
                } else {
                    if let Err(e) = DB.set_login(&msg.session, None) {
                        return Either::B(err(e.into()));
                    }
                    Either::B(ok(LoginState::NotLoggedIn))
                }
            });
        Box::new(fut)
    }
}

impl Handler<CheckSession> for UserService {
    type Result = Result<LoginState>;

    fn handle(&mut self, msg: CheckSession, _ctx: &mut Context<Self>) -> Self::Result {
        use db::models::LoginState as DBLoginState;
        Ok(match DB.get_login(&msg.session, self.login_max_age)? {
            Some(v) => match v.state {
                DBLoginState::Complete => LoginState::LoggedIn(UserMin::from(DB.get_user(v.id)?)),
                DBLoginState::Missing2Fa => LoginState::RequiresTOTP,
                DBLoginState::Requires2FaSetup => {
                    LoginState::RequiresTOTPSetup(DB.get_user(v.id)?.totp.into())
                }
            },
            None => LoginState::NotLoggedIn,
        })
    }
}

impl Handler<GetSessionServiceIDs> for UserService {
    type Result = Result<Vec<SID>>;

    fn handle(&mut self, msg: GetSessionServiceIDs, _ctx: &mut Context<Self>) -> Self::Result {
        let id = self.get_session_uid(&msg.session)?;
        Ok(DB
            .get_all_perm_service(id)?
            .into_iter()
            .map(|(k, _)| k)
            .collect())
    }
}

impl Handler<GetAdminPerm> for UserService {
    type Result = Result<bool>;

    fn handle(&mut self, msg: GetAdminPerm, _ctx: &mut Context<Self>) -> Self::Result {
        let uid = self.get_session_uid(&msg.session)?;
        Ok(DB.get_user(uid)?.admin)
    }
}

impl Handler<GetServicePermUser> for UserService {
    type Result = Result<ServicePerm>;

    fn handle(&mut self, msg: GetServicePermUser, _ctx: &mut Context<Self>) -> Self::Result {
        Ok(DB.get_perm_service(msg.user, msg.service)?)
    }
}

impl Handler<SetServicePermUser> for UserService {
    type Result = Result<()>;

    fn handle(&mut self, msg: SetServicePermUser, _ctx: &mut Context<Self>) -> Self::Result {
        DB.set_perm_service(msg.user, msg.service, msg.perm)?;
        Ok(())
    }
}

impl Handler<GetServicePerm> for UserService {
    type Result = Result<ServicePerm>;

    fn handle(&mut self, msg: GetServicePerm, _ctx: &mut Context<Self>) -> Self::Result {
        let uid = self.get_session_uid(&msg.session)?;
        Ok(DB.get_perm_service(uid, msg.service)?)
    }
}

impl Handler<LogoutUser> for UserService {
    type Result = Result<()>;

    fn handle(&mut self, msg: LogoutUser, _ctx: &mut Context<Self>) -> Self::Result {
        DB.set_login(&msg.session, None)?;
        //TODO: kick from websocket
        Ok(())
    }
}

impl Handler<GetAllUsers> for UserService {
    type Result = Result<Vec<UserMin>>;

    fn handle(&mut self, _msg: GetAllUsers, _ctx: &mut Context<Self>) -> Self::Result {
        Ok(DB.get_users()?)
    }
}

impl Handler<CreateUser> for UserService {
    type Result = Result<CreateUserResp>;

    fn handle(&mut self, msg: CreateUser, _ctx: &mut Context<Self>) -> Self::Result {
        let uid = self.get_session_uid(&msg.invoker)?;
        self.check_admin(uid)?;
        self.create_user_unchecked(msg.user)
    }
}

impl Handler<ResetUserTOTP> for UserService {
    type Result = Result<()>;

    fn handle(&mut self, msg: ResetUserTOTP, _ctx: &mut Context<Self>) -> Self::Result {
        let invoker_id = self.get_session_uid(&msg.invoker)?;
        let mut user = DB.get_user(msg.id)?;
        // foreign account, check admin
        if invoker_id != msg.id {
            if !self.is_admin(invoker_id)? {
                return Err(UserError::InvalidPermissions);
            }
        } else {
            if let Some(pw) = msg.data.password {
                if !bcrypt_verify(&pw, &user.password)? {
                    return Err(UserError::InvalidPassword);
                }
            } else {
                return Err(UserError::BadRequest("missing password"));
            }
        }

        user.totp = crate::crypto::totp_gen_secret();
        user.totp_complete = false;
        DB.update_user(user)?;

        Ok(())
    }
}

impl Handler<SetUserPassword> for UserService {
    type Result = Result<()>;

    fn handle(&mut self, msg: SetUserPassword, _ctx: &mut Context<Self>) -> Self::Result {
        let invoker_id = self.get_session_uid(&msg.invoker)?;
        let mut user = DB.get_user(msg.id)?;
        // foreign account, check admin
        if invoker_id != msg.id {
            if !self.is_admin(invoker_id)? {
                return Err(UserError::InvalidPermissions);
            }
        } else {
            if let Some(pw) = msg.data.old_password {
                if !bcrypt_verify(&pw, &user.password)? {
                    return Err(UserError::InvalidPassword);
                }
            } else {
                return Err(UserError::BadRequest("missing password"));
            }
        }

        user.password = bcrypt_password(&msg.data.password, self.brcypt_cost)?;
        DB.update_user(user)?;

        Ok(())
    }
}

impl Handler<SetUserInfo> for UserService {
    type Result = Result<()>;

    fn handle(&mut self, msg: SetUserInfo, _ctx: &mut Context<Self>) -> Self::Result {
        let invoker_id = self.get_session_uid(&msg.invoker)?;
        // foreign account, check admin
        if invoker_id != msg.user {
            if !self.is_admin(invoker_id)? {
                return Err(UserError::InvalidPermissions);
            }
        }
        let mut user_full = DB.get_user(msg.user)?;
        user_full.name = msg.data.name;
        user_full.email = msg.data.email;
        DB.update_user(user_full)?;
        Ok(())
    }
}

impl Handler<GetUserInfo> for UserService {
    type Result = Result<UserMin>;

    fn handle(&mut self, msg: GetUserInfo, _ctx: &mut Context<Self>) -> Self::Result {
        Ok(DB.get_user(msg.user)?.into())
    }
}

impl Handler<DeleteUser> for UserService {
    type Result = Result<()>;

    fn handle(&mut self, msg: DeleteUser, _ctx: &mut Context<Self>) -> Self::Result {
        let uid = self.get_session_uid(&msg.invoker)?;
        if uid == msg.user {
            // can't delete admins
            warn!("{} tried to delete admin user {}", uid, msg.user);
            return Err(UserError::InvalidPermissions);
        }
        self.check_admin(uid)?;
        Ok(DB.delete_user(msg.user)?)
    }
}

impl Handler<SetPasswordCost> for UserService {
    type Result = ();

    fn handle(&mut self, msg: SetPasswordCost, _ctx: &mut Context<Self>) {
        self.brcypt_cost = msg.cost;
    }
}

impl Handler<EditUser> for UserService {
    type Result = Result<bool>;

    fn handle(&mut self, msg: EditUser, _ctx: &mut Context<Self>) -> Self::Result {
        if msg.invoker != msg.user_uid {
            if !self.is_admin(msg.invoker)? {
                return Ok(false);
            }
        }
        if let EditUserData::ServicePermission((service, perm)) = msg.data {
            DB.set_perm_service(msg.user_uid, service, perm)?;
        } else {
            let mut user = DB.get_user(msg.user_uid)?;
            match msg.data {
                EditUserData::Name(name) => user.name = name,
                EditUserData::Mail(email) => user.email = email,
                EditUserData::Password(pw) => {
                    user.password = bcrypt_password(&pw, self.brcypt_cost)?
                }
                // EditUserData::TOTP(secret) => user.totp_secret = secret,
                EditUserData::ServicePermission(_) => unreachable!(),
            }
            DB.update_user(user)?;
        }
        Ok(true)
    }
}

impl SystemService for UserService {}
impl Supervised for UserService {}
impl Actor for UserService {
    type Context = Context<Self>;

    fn started(&mut self, context: &mut Context<Self>) {
        IntervalFunc::new(
            std::time::Duration::from_secs(CLEANUP_INTERVAL),
            Self::cleanup_sessions,
        )
        .finish()
        .spawn(context);
    }
}
