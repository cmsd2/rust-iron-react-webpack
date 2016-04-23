use std::result;
use std::io;

use iron::prelude::*;
use iron::status;
use persistent;
use rustc_serialize::json::{DecoderError, EncoderError};

quick_error! {
    #[derive(Debug)]
    pub enum AppError {
        IoError(err: io::Error) {
            from()
            description("io error")
            display("I/O error: {}", err)
            cause(err)
        }
        
        NotImplemented {
            description("not implemented")
            display("Not implemented")
        }
        
        JsonEncoderError(err: EncoderError) {
            from()
            description("error encoding json")
            display("Error encoding json: {}", err)
            cause(err)
        }
        
        JsonDecoderError(err: DecoderError) {
            from()
            description("error decoding json")
            display("Error decoding json: {}", err)
            cause(err)
        }
        
        InvalidUsernameOrPassword {
            description("invalid username or password")
            display("Invalid username or password")
        }
        
        PersistenceError(err: persistent::PersistentError) {
            from()
            description("error persisting state")
            display("error persisting state")
            cause(err)
        }
        
        NoSessionLoaded {
            description("server session middleware not available")
            display("server session middleware not available")
        }
    }
}

pub fn error_status_code(err: &AppError) -> status::Status {
    match *err {
        _ => status::InternalServerError
    }
}

impl From<AppError> for IronError {
    fn from(err: AppError) -> IronError {
        let status_code = error_status_code(&err);
        
        IronError::new(err, status_code)
    }
}

pub type Result<T> = result::Result<T,AppError>;