extern crate querystring;
extern crate simple_server;
extern crate std;

pub fn run_debug_server(port: &str) {
    let server = simple_server::Server::new(|request, mut response| {
        match (request.method(), request.uri().path()) {
            (&simple_server::Method::GET, "/") => {
                list_logs(&mut response)
            }
            (&simple_server::Method::GET, "/dumplog") => {
                dump_log(&request, &mut response)
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

fn which_log(request: &simple_server::Request<Vec<u8>>) -> Option<String> {
    let query = request.uri().query().unwrap();

    let params = querystring::querify(query);

    for (k, v) in params {
        if k == "log" && v.ends_with(".log") {
            return Some(v.to_string());
        }
    }

    return None;
}



fn dump_log(request: &simple_server::Request<Vec<u8>>, response: &mut simple_server::ResponseBuilder) -> simple_server::ResponseResult {
    let filename = which_log(request);

    if filename.is_none() {
        return Err(simple_server::Error::Timeout);
    }

    let filename = filename.unwrap();

    match std::fs::read_to_string(filename) {
        Ok(contents) => return Ok(response.body(contents.as_bytes().to_vec())?),
        Err(_) => return Err(simple_server::Error::Timeout),
    }
}

fn list_logs(response: &mut simple_server::ResponseBuilder) -> simple_server::ResponseResult {
    let mut body = "<html><body><ul>".to_string();

    for entry in std::fs::read_dir("./")? {
        let entry = entry?;
        if entry.path().to_string_lossy().ends_with(".log") {
            let filename = entry.path().file_name().unwrap().to_str().unwrap().to_string();
            body.push_str(&format!("<li><a href='/dumplog?log={}'>{}</a></li>", filename, filename));
        }
    }

    body.push_str("</ul></body></html>");

    return Ok(response.body(body.as_bytes().to_vec())?)
}
