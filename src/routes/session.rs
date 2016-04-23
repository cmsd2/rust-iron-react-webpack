use std::sync::Arc;
use std::sync::Mutex;
use std::io::Read;
use std::collections::HashMap;
use rand;
use rand::Rng;

use rustc_serialize::{json, base64};
use rustc_serialize::base64::ToBase64;
use iron::prelude::*;
use iron::status;
use iron::middleware::{BeforeMiddleware};
use iron::typemap::Key;
use oven::prelude::*;
use persistent;
use plugin;
use plugin::Extensible;

use result::*;
use login::*;

#[derive(Clone, Debug, RustcDecodable, RustcEncodable)]
pub struct Credentials {
    username: String,
    password: String,
}

#[derive(Clone, Debug)]
pub struct User {
    id: String,
    username: String,
    password: String,
}

impl User {
    pub fn new(id: String, username: String, password: String) -> User {
        User {
            id: id,
            username: username,
            password: password,
        }
    }
}

#[derive(Clone, Debug, RustcDecodable, RustcEncodable)]
pub struct UserSession {
    username: String,
    user_id: String,
    session_id: String,
    authenticated: bool,
}

impl UserSession {
    pub fn new(user_id: String, username: String, session_id: String) -> UserSession {
        UserSession {
            username: username,
            user_id: user_id,
            session_id: session_id,
            authenticated: false,
        }
    }
}

impl LoginSession for UserSession {
    fn get_id(&self) -> String {
        self.session_id.clone()
    }
}

pub trait Sessions: Send + Sync + 'static {
    fn authenticate(&self, creds: &Credentials) -> Result<UserSession>;
    
    fn lookup(&self, session_id: &str) -> Result<Option<UserSession>>;
    
    fn remove(&self, session_id: &str) -> Result<bool>;
}

pub trait UserRepo: Send + Sync + 'static {
    fn find_user(&self, username: &str) -> Result<Option<User>>;
    
    fn add_user(&self, user: User) -> Result<()>;
    
    fn remove_user(&self, username: &str) -> Result<bool>;
}

pub struct InMemorySessions {
    users: Arc<Box<UserRepo>>,
    sessions: Arc<Mutex<HashMap<String, UserSession>>>,
}

impl InMemorySessions {
    pub fn new(users: Arc<Box<UserRepo>>) -> InMemorySessions {
        InMemorySessions {
            users: users,
            sessions: Arc::new(Mutex::new(HashMap::new()))
        }
    }
    
    fn new_session_id(&self) -> String {
        let mut id = vec![0u8; 16];
        rand::thread_rng().fill_bytes(id.as_mut_slice());
        id.to_base64(base64::STANDARD)
    }
    
    fn new_session(&self, user_id: &str, username: &str) -> UserSession {
        UserSession::new(user_id.to_owned(), username.to_owned(), self.new_session_id())
    }
}

pub struct InMemoryUserRepo {
    users: Arc<Mutex<HashMap<String, User>>>,
}

impl InMemoryUserRepo {
    pub fn new() -> InMemoryUserRepo {
        InMemoryUserRepo {
            users: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl UserRepo for InMemoryUserRepo {
    fn find_user(&self, username: &str) -> Result<Option<User>> {
        let users = self.users.lock().unwrap();
        
        Ok(users.get(username).map(|u| (*u).clone()))
    }
    
    fn add_user(&self, user: User) -> Result<()> {
        let mut users = self.users.lock().unwrap();
        
        users.entry(user.username.clone()).or_insert(user);
        
        Ok(())
    }
    
    fn remove_user(&self, username: &str) -> Result<bool> {
        let mut users = self.users.lock().unwrap();
        
        Ok(users.remove(username).is_some())
    }
}

impl Sessions for InMemorySessions {
    
    fn authenticate(&self, creds: &Credentials) -> Result<UserSession> {
        if let Some(user) = try!(self.users.find_user(&creds.username)) {
            if user.password == creds.password {
                let mut session = self.new_session(&user.id, &user.username);
                session.authenticated = true;
                
                let mut sessions = self.sessions.lock().unwrap();
                sessions.insert(session.session_id.clone(), session.clone());
                
                Ok(session)
            } else {
                // TODO add random wait jitter
                Err(AppError::InvalidUsernameOrPassword)
            }
        } else {
            // TODO add random wait jitter
            Err(AppError::InvalidUsernameOrPassword)
        }
    }
    
    fn lookup(&self, session_id: &str) -> Result<Option<UserSession>> {
        let sessions = self.sessions.lock().unwrap();
        
        Ok(sessions.get(session_id).map(|u| (*u).clone()))
    }
    
    fn remove(&self, session_id: &str) -> Result<bool> {
        let mut sessions = self.sessions.lock().unwrap();
        
        Ok(sessions.remove(session_id).is_some())
    }
}

#[derive(Clone)]
pub struct SessionController {
    sessions: Arc<Box<Sessions>>,
    login_manager: LoginManager,
}

impl SessionController {
    pub fn new(sessions: Arc<Box<Sessions>>, login_manager: LoginManager) -> Self {
        SessionController {
            sessions: sessions,
            login_manager: login_manager,
        }
    }
    
    pub fn load_session_id(&self, req: &mut Request) -> Result<Option<String>> {
        let config = try!(req.get::<persistent::Read<Config>>());
                
        let session = match req.get_cookie(&config.cookie_base.name) {
            Some(c) if !c.value.is_empty() => {
                Some(c.value.clone())
            },
            _ => None,
        };

        Ok(session)
    }

    pub fn load_session(&self, req: &mut Request) -> Result<Login<UserSession>> {
        let config_arc = try!(req.get::<persistent::Read<Config>>());
        let config = (*config_arc).clone();
                
        let session = if let Some(session_id) = try!(self.load_session_id(req)) {
            try!(self.sessions.lookup(&session_id))
        } else {
            None
        };
        
        Ok(Login::new(&config, session))
    }
    
    pub fn clear_session(&self, req: &mut Request) -> Result<bool> { 
        if let Some(session_id) = try!(self.load_session_id(req)) {
            self.sessions.remove(&session_id)
        } else {
            Ok(false)
        }
    }
}

impl Key for UserSession { type Value = Option<UserSession>; }

impl<'a, 'b> plugin::Plugin<Request<'a, 'b>> for UserSession {
    type Error = AppError;
    
    fn eval(req: &mut Request) -> Result<Option<UserSession>> {
        debug!("getting session from middleware chain");
        req.extensions().get::<UserSession>().ok_or(AppError::NoSessionLoaded).map(|s| s.to_owned())
    }
}

impl BeforeMiddleware for SessionController {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        match self.load_session(req) {
            Ok(login) => {
                debug!("injecting session into middleware chain {:?}", login);
                req.extensions_mut().insert::<UserSession>(login.session);
                Ok(())
            },
            Err(AppError::PersistenceError(persistent::PersistentError::NotFound)) => {
                debug!("no session found");
                req.extensions_mut().insert::<UserSession>(None);
                Ok(())
            }
            Err(e) => {
                req.extensions_mut().insert::<UserSession>(None);
                Err(IronError::from(AppError::from(e)))
            }
        }
    }
}

pub fn parse_credentials(req: &mut Request) -> Result<Credentials> {
    let mut creds_str = String::new();
    
    try!(req.body.read_to_string(&mut creds_str));
    
    let creds = try!(json::decode(&creds_str));
    
    Ok(creds)
}

pub fn serialize_session(session: &UserSession) -> Result<String> {
    let json_str = try!(json::encode(&session));
    
    Ok(json_str)
}

pub fn get_session(sc: &SessionController, req: &mut Request) -> IronResult<Response> {
    let login = try!(sc.load_session(req));
    
    if login.session.is_some() {
        let session_json = try!(serialize_session(&login.session.unwrap()));
    
        Ok(Response::new()
            .set(status::Ok)
            .set(session_json)) 
    } else {
        Ok(Response::new()
            .set(status::NotFound)
            .set("Not Found"))
    }
}

pub fn post_session(sc: &SessionController, req: &mut Request) -> IronResult<Response> {
    let creds = try!(parse_credentials(req));
    
    debug!("received credentials: {:?}", creds);
    
    match sc.sessions.authenticate(&creds) {
        Ok(session) => {
            let session_json = try!(serialize_session(&session));
            
            Ok(Response::new()
                .set(status::Ok)
                .set(Login::new(&sc.login_manager.config, Some(session)).cookie())
                .set(session_json))
        },
        Err(AppError::InvalidUsernameOrPassword) => {
            Ok(Response::with((status::Forbidden, "invalid username or password")))   
        },    
        _ => {
            Ok(Response::with((status::InternalServerError, "post session not implemented")))
        }
    }      
}

pub fn delete_session(sc: &SessionController, req: &mut Request) -> IronResult<Response> {
    try!(sc.clear_session(req));
    
    Ok(Response::with((status::Ok, "not implemented")))
}