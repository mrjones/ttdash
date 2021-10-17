extern crate pretty_bytes;
extern crate querystring;
extern crate simple_server;
extern crate std;

use crate::update;

pub fn run_debug_server(port: &str, local_png: Option<String>) {
    let server = simple_server::Server::new(move |request, mut response| {
        match (request.method(), request.uri().path()) {
            (&simple_server::Method::GET, "/") => {
                main_page(&mut response, local_png.is_some())
            }
            (&simple_server::Method::GET, "/dumplog") => {
                dump_log(&request, &mut response)
            }
            (&simple_server::Method::GET, "/current_image") => {
                current_image(&mut response, local_png.as_ref().map(String::as_str))
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

fn current_image(response: &mut simple_server::ResponseBuilder, local_png: Option<&str>) -> simple_server::ResponseResult {
    match local_png {
        None => {
            return Err(simple_server::Error::Timeout);
        }
        Some(local_png) => {
            match std::fs::read(local_png) {
                Err(_) => return Err(simple_server::Error::Timeout),
                Ok(bytes) => return Ok(response.body(bytes)?),
            }
        },
    }
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

fn main_page(response: &mut simple_server::ResponseBuilder, has_local_png: bool) -> simple_server::ResponseResult {
    let mut body = format!("<html><body><h1>TTDash Debug Server</h1><div>Version {}</div>",
                           update::local_version()
                           .map(|v| v.to_string())
                           .unwrap_or("[unknown]".to_string()));

    if has_local_png {
        body.push_str("<div><h2>Current image</h2><img style='border: 1px solid black;' src='/current_image' /></div>");
    }

    body.push_str("<div><h2>Log files</h2><ul>");
    for entry in std::fs::read_dir("./")? {
        let entry = entry?;
        if entry.path().to_string_lossy().ends_with(".log") {
            let filename = entry.path().file_name().unwrap().to_str().unwrap().to_string();
            body.push_str(&format!("<li><a href='/dumplog?log={}'>{}</a> [{}]</li>", filename, filename, pretty_bytes::converter::convert(entry.metadata()?.len() as f64)));
        }
    }
    body.push_str("</ul></div>");

    body.push_str("</body></html>");

    return Ok(response.body(body.as_bytes().to_vec())?)
}
