use std::collections::HashMap;

use iron::prelude::*;
use iron::status;
use rustc_serialize::json;

use result::*;

#[derive(RustcDecodable, RustcEncodable)]
pub struct TestStruct {
    data_int: u8,
    data_str: String,
    data_vector: Vec<u8>,
    data_map: HashMap<String, String>,
}

#[derive(RustcDecodable, RustcEncodable)]
pub struct SimpleItems {
    items: Vec<String>,
}

fn simple_data() -> Result<String> {
    let mut map = HashMap::new();
    map.insert("key1".to_owned(), "message1".to_owned());
    map.insert("key2".to_owned(), "message2".to_owned());
    
    let object = SimpleItems {
        items: vec!["item 1".to_owned(), "item 2".to_owned()]
    };
    
    let result = try!(json::encode(&object));
    
    Ok(result)
}

pub fn simple(_req: &mut Request) -> IronResult<Response> {
    
    
    Ok(Response::with((status::Ok, try!(simple_data()))))
}