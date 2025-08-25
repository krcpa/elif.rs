use crate::errors::HttpResult;
use std::future::Future;
use std::pin::Pin;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait GenericHandler<T>: Clone + Send + Sync + 'static {
    type Response;

    fn call(&self, request: T) -> BoxFuture<'_, HttpResult<Self::Response>>;
}

pub trait IntoElifResponse {
    fn into_response(self) -> crate::response::ElifResponse;
}

pub trait RequestExtractor: Sized + Send + 'static {
    type Error;

    fn extract(request: &crate::request::ElifRequest) -> Result<Self, Self::Error>;
}
