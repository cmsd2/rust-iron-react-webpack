/// Derived from iron_login: https://github.com/untitaker/iron-login

use iron::prelude::*;
use iron::middleware;
use iron::typemap::Key;
use iron::modifier;
use oven;
use oven::prelude::*;
use persistent;
use cookie::Cookie;

#[derive(Clone, Debug)]
pub struct LoginManager {
    signing_key: Vec<u8>,
    /// Configuration for this manager
    pub config: Config,
}

impl LoginManager {
    /// Construct a new login middleware using the provided signing key
    pub fn new(signing_key: Vec<u8>) -> LoginManager {
        LoginManager {
            signing_key: signing_key,
            config: Config::defaults(),
        }
    }
}

/// Configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// This cookie contains the default values that will be used for session cookies.
    ///
    /// You may e.g. override `httponly` or `secure` however you wish.
    pub cookie_base: Cookie,
}

impl Config {
    /// Construct a configuration instance with default values
    pub fn defaults() -> Self {
        Config {
            cookie_base: {
                let mut c = Cookie::new("logged_in_user".to_owned(), "".to_owned());
                c.httponly = true;
                c.path = Some("/".to_owned());
                c
            },
        }
    }
}

impl Key for Config { type Value = Config; }

impl middleware::AroundMiddleware for LoginManager {
    fn around(self, handler: Box<middleware::Handler>) -> Box<middleware::Handler> {
        let mut ch = Chain::new(handler);
        let key = self.signing_key;

        ch.link(oven::new(key));
        ch.link(persistent::Read::<Config>::both(self.config));

        Box::new(ch)
    }
}

pub struct LoginModifier<U: LoginSession> {
    login: Login<U>
}
impl <U: LoginSession> modifier::Modifier<Response> for LoginModifier<U> {
    fn modify(self, response: &mut Response) {
        response.set_cookie({
            let mut x = self.login.config.cookie_base.clone();
            x.value = self.login.session.map_or_else(|| "".to_owned(), |u| u.get_id());
            x
        });
    }
}

#[derive(Clone, Debug)]
pub struct Login<U: LoginSession> {
    pub session: Option<U>,
    pub config: Config,
}

impl <U: LoginSession> Login<U> {
    pub fn new(config: &Config, session: Option<U>) -> Login<U> {
        Login {
            session: session,
            config: config.clone(),
        }
    }
    
    pub fn cookie(&self) -> LoginModifier<U> {
        LoginModifier {
            login: (*self).clone()
        }
    }
}

pub trait LoginSession: Clone + Send + Sync + Sized {
    fn get_id(&self) -> String;
}