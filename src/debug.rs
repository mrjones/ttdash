extern crate simple_server;

pub fn run_debug_server(port: &str) {
    let server = simple_server::Server::new(|request, mut response| {
        match (request.method(), request.uri().path()) {
            (&simple_server::Method::GET, "/hello") => {
                Ok(response.body("<h1>Hi!</h1><p>Hello Rust!</p>".as_bytes().to_vec())?)
            }
            (_, _) => {
                response.status(simple_server::StatusCode::NOT_FOUND);
                Ok(response.body("<h1>404</h1><p>Not found!<p>".as_bytes().to_vec())?)
            }
        }
    });

    debug!("Running debug HTTP server on port {}", port);
    server.listen("0.0.0.0", port);
}
