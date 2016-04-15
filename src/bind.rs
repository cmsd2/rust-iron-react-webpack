use iron::prelude::*;
use iron::Handler;

pub trait MethodHandler<T>: Send + Sync + 'static {
    fn handle(&self, &T, &mut Request) -> IronResult<Response>;
}

impl <T,F> MethodHandler<T> for F
where T: Send + Sync + 'static,
F: Send + Sync + 'static + Fn(&T, &mut Request) -> IronResult<Response> {
    fn handle(&self, state: &T, req: &mut Request) -> IronResult<Response> {
        (*self)(state, req)
    }
}

pub struct BoundMethodHandler<T> 
where T: Send + Sync + 'static
{
    state: T,
    handler: Box<MethodHandler<T>>
}

impl <T> Handler for BoundMethodHandler<T>
where T: Send + Sync + 'static
{
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        self.handler.handle(&self.state, req)
    }
}

pub fn bind<T, H>(h: H, t: T) -> Box<Handler>
where H: MethodHandler<T>,
T: Send + Sync + 'static
{
    Box::new(BoundMethodHandler {
        state: t,
        handler: Box::new(h)
    })
}