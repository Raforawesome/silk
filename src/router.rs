use std::collections::HashMap;

use crate::http::request::Request;

pub type Handler<'a> = dyn Fn(Request) + 'a;

pub enum RouteError {
    MalformedPath,
}

pub trait Router<'a> {
    fn add_route<R, F>(route: R, handler: F)
    where
        R: AsRef<[u8]>,
        F: Fn(Request) + 'a;

    fn get_route<R, F>(route: R) -> F
    where
        R: AsRef<[u8]>,
        F: Fn(Request) + 'a;
}

pub struct HashRouter<'a> {
    routes: HashMap<&'a [u8], Box<Handler<'a>>>,
}

impl<'a> HashRouter<'a> {
    pub fn push(&mut self, path: &'a [u8], handler: Box<Handler<'a>>) -> Option<Box<Handler<'a>>> {
        self.routes.insert(path, handler)
    }
}

impl<'a> Default for HashRouter<'a> {
    fn default() -> Self {
        let routes = HashMap::new();
        HashRouter { routes }
    }
}

impl<'a> Router<'a> for HashRouter<'a> {
    fn add_route<R, F>(route: R, handler: F)
    where
        R: AsRef<[u8]>,
        F: Fn(Request) + 'a,
    {
        todo!()
    }

    fn get_route<R, F>(route: R) -> F
    where
        R: AsRef<[u8]>,
        F: Fn(Request) + 'a,
    {
        todo!()
    }
}

// TODO: finish tree router
// Lifetime guide:
// 'a: lifetime of router
// 'b: lifetime of path as byte sequence
#[allow(clippy::type_complexity)]
struct RTreeNode<'a, 'b> {
    children: Vec<RTreeNode<'a, 'b>>,
    handler: Option<Box<dyn Fn(Request) + 'a>>,
    segment: &'b [u8],
}

pub struct TreeRouter<'a, 'b> {
    root: RTreeNode<'a, 'b>,
}

impl<'a, 'b> TreeRouter<'a, 'b> {
    pub fn from(path_bytes: &'b [u8]) -> Result<Self, RouteError> {
        let first_char = path_bytes.first();
        if first_char.is_none() || *first_char.unwrap() != b'/' {
            return Err(RouteError::MalformedPath);
        }

        let path_bytes = if *path_bytes.last().unwrap() == b'/' {
            &path_bytes[..path_bytes.len() - 1]
        } else {
            path_bytes
        };

        todo!()
    }
}
