use std::{future::{ready, Ready}, rc::Rc, cell::RefCell};

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use futures_util::future::LocalBoxFuture;
use paris::info;

use crate::database::Database;

pub struct ClientSeenFactory {
    db: Rc<RefCell<Database>>
}

impl ClientSeenFactory {
    pub fn new(db: Database) -> Self {
        ClientSeenFactory { db: Rc::new(RefCell::new(db)) }
    }
}

impl<S, B> Transform<S, ServiceRequest> for ClientSeenFactory
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = ClientSeenMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ClientSeenMiddleware { service, db: self.db.clone() }))
    }
}

pub struct ClientSeenMiddleware<S> {
    service: S,
    db: Rc<RefCell<Database>>
}

impl<S, B> Service<ServiceRequest> for ClientSeenMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let binding = req.connection_info().clone();
        let ip = binding.peer_addr().unwrap();

        let mut db_mut = self.db.borrow_mut();
        db_mut.seen_client(ip);
        db_mut.flush();

        let client = db_mut.get_client_by_ip(ip);

        info!(
            "{} ({}) -- {} {}",
            ip,
            if client.is_some() {
                client.unwrap().name.clone()
            } else {
                String::from("")
            },
            req.method(),
            req.uri().path()
        );

        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;

            Ok(res)
        })
    }
}

