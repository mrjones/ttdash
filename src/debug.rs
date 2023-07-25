extern crate pretty_bytes;
extern crate querystring;
extern crate std;
extern crate tiny_http;

use crate::update;

pub fn run_debug_server(port: &str, local_png: Option<String>) {
    let server = tiny_http::Server::http(format!("0.0.0.0:{}", port)).expect("http server");
    debug!("Running debug HTTP server on port {}", port);

    for request in server.incoming_requests() {
        let url = request.url().clone();
        info!("Request: {}", url);
        if url == "/" {
            main_page(request, local_png.is_some())
        } else if url.starts_with("/dumplog") {
            dump_log(request)
        } else if url == "/current_image" {
            current_image(request, local_png.as_ref().map(String::as_str))
        } else {
            let response = tiny_http::Response::from_string(
                format!("Unknown URL: {}", url));
            request.respond(response);
        }

    }
}

fn which_log(request: &tiny_http::Request) -> Option<String> {
    let path_and_query = request.url();
    let mut path_and_query_parts = path_and_query.splitn(2, '?');
    let path = path_and_query_parts.next().unwrap();
    let query_string = path_and_query_parts.next().unwrap_or("");

    let params = querystring::querify(query_string);

    for (k, v) in params {
        if k == "log" && v.ends_with(".log") {
            return Some(v.to_string());
        }
    }

    return None;
}

fn current_image(request: tiny_http::Request, local_png: Option<&str>) {
    match local_png {
        None => {
            request.respond(
                tiny_http::Response::from_string("local_png not configured")).unwrap();
        }
        Some(local_png) => {
            match std::fs::read(local_png) {
                Err(err) => {
                    request.respond(
                        tiny_http::Response::from_string(
                            format!("Couldn't read {}: {:?}", local_png, err))).unwrap();
                },
                Ok(bytes) => {
                    request.respond(
                        tiny_http::Response::from_data(bytes)
                            .with_header(tiny_http::Header::from_bytes(
                                &b"Content-Type"[..], &b"image/png"[..]).unwrap()));
                }
            }
        },
    }
}

fn dump_log(request: tiny_http::Request) {
    let filename = which_log(&request);

    if filename.is_none() {
        request.respond(
            tiny_http::Response::from_string("filename missing")).unwrap();
        return;
    }

    let filename = filename.unwrap();

    match std::fs::read_to_string(filename) {
        Ok(contents) => {
            request.respond(
                tiny_http::Response::from_string(contents)).unwrap();
        },
        Err(err) => {
            request.respond(
                tiny_http::Response::from_string(
                    format!("ERROR: {:?}", err))).unwrap();
        }
    }
}

fn main_page(request: tiny_http::Request, has_local_png: bool)  {
    let mut body = format!("<html><body><h1>TTDash Debug Server</h1><div>Version {}</div>",
                           update::local_version()
                           .map(|v| v.to_string())
                           .unwrap_or("[unknown]".to_string()));

    if has_local_png {
        body.push_str("<div><h2>Current image</h2><img style='border: 1px solid black;' src='/current_image' /></div>");
    }

    body.push_str("<div><h2>Log files</h2><ul>");
    for entry in std::fs::read_dir("./").expect("fs.read_dir") {
        let entry = entry.expect("entry in main_page");
        if entry.path().to_string_lossy().ends_with(".log") {
            let filename = entry.path().file_name().unwrap().to_str().unwrap().to_string();
            body.push_str(&format!("<li><a href='/dumplog?log={}'>{}</a> [{}]</li>", filename, filename, pretty_bytes::converter::convert(entry.metadata().expect("query metadata").len() as f64)));
        }
    }
    body.push_str("</ul></div>");

    body.push_str("</body></html>");

    request.respond(
        tiny_http::Response::from_string(body)
            .with_header(tiny_http::Header::from_bytes(
                &b"Content-Type"[..], &b"text/html"[..]).unwrap()))
        .expect("send response");
}
