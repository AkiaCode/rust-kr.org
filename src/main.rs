//! [한국 러스트 사용자 그룹 홈페이지][rust-kr] 소스코드
//! --------
//!
//! ### 사용법
//! ```bash
//! git clone https://github.com/rust-kr/rust-kr.org.wiki.git docs
//! cargo run
//! # 웹 브라우저에서 http://localhost:8000 를 열어보세요
//! ```
//!
//! [rust-kr]: http://rust-kr.org

#[macro_use(itry)] extern crate iron;
#[macro_use(router)] extern crate router;
extern crate mount;
extern crate staticfile;
extern crate handlebars_iron as hbs;
extern crate pulldown_cmark as cmark;

use std::collections::BTreeMap;
use iron::prelude::*;
use iron::status;
use iron::middleware::AfterMiddleware;
use mount::Mount;
use staticfile::Static;
use hbs::{Template, HandlebarsEngine, DirectorySource};

//
// Configs
//
/// 서버가 소켓을 열 주소
const ADDR: &'static str = "localhost:8000";
/// 마크다운 문서가 담긴 주소
const DOCS_DIR: &'static str = "docs";

/// Entry point
fn main() {
    // Declarative definition of request handling
    let app = Iron::new({
        let mut c = Chain::new({
            // '/static/*'          -> static file serving
            // '/'                  -> Main page
            // '/pages/:name'       -> Specific documents
            // '/pages/_pages'      -> See all documents

            let mut m = Mount::new();
            m.mount("/static/", Static::new("static/"));
            m.mount("/", router! {
                get "/" => index,
                get "/pages/:name" => page,
                get "/pages/_pages" => all_docs,
            });
            m
        });
        // 404 page handler
        c.link_after(catch(|mut err: IronError| {
            if err.response.body.is_some() { return Err(err); }
            if err.response.status != Some(status::NotFound) { return Err(err); }

            // TODO: 좀더 내용 채우기
            let mut data = BTreeMap::new();
            data.insert("content", r#"
                <h1>404</h1>
                <p>This is not the web page you are looking for.</p>
            "#);
            err.response.set_mut(Template::new("default", data));
            Err(err)
        }));
        // Handlebar templating
        c.link_after({
            let mut hbr = HandlebarsEngine::new();
            hbr.add(Box::new(DirectorySource::new("templates", ".hbs")));
            if let Err(r) = hbr.reload() {
                use std::error::Error;
                panic!("{}", r.description());
            }
            hbr
        });
        c
    });

    println!("\nServer running at \x1b[33mhttp://{}\x1b[0m\n", ADDR);
    app.http(ADDR).unwrap();
}

/// Root page handler
fn index(_: &mut Request) -> IronResult<Response> {
    read_docs("Home")
}

/// Wiki page handler
fn page(req: &mut Request) -> IronResult<Response> {
    use router::Router;

    // Parse URL
    let name = req.extensions.get::<Router>().unwrap().find("name").unwrap_or("Home");
    read_docs(name)
}

/// "All documents" page handler
fn all_docs(_: &mut Request) -> IronResult<Response> {
    use std::fs::read_dir;

    let mut paths: Vec<_> = itry!(read_dir(DOCS_DIR))
        .filter_map(|f| f.ok())
        .map(|f| f.path())
        .filter(|p| p.is_file())
        .filter(|p| p.as_os_str().to_str()
            .map(|s| s.ends_with(".md"))
            .unwrap_or(false))
        .collect();
    paths.sort();

    let pages = paths.iter()
        .filter_map(|p| p.file_stem())
        .filter_map(|stem| stem.to_str());

    // TODO: 러스트로 이짓하지 말고 Handlebar로 대체
    let mut html = "<ul>".to_string();
    for page in pages {
        use std::fmt::Write;
        write!(&mut html, r#"<li><a href="/pages/{0}">{0}</a></li>"#, page).unwrap();
    }
    html.push_str("</ul>");

    // Fill in to the template
    let mut data = BTreeMap::new();
    data.insert("content", html);
    Ok(Response::with((status::Ok, Template::new("default", data))))
}

/// `name`을 인자로 받아, `docs/<name>.md` 파일을 렌더링하여 `IronResult<Response>`로 반환합니다.
fn read_docs(name: &str) -> IronResult<Response> {
    use std::fs::File;
    use std::io::Read;
    use cmark::Parser;
    use cmark::html::push_html;

    // Read file
    let path = format!("{}/{}.md", DOCS_DIR, name);
    let mut file = itry!(File::open(&path));
    let mut md = String::new();
    itry!(file.read_to_string(&mut md));

    // Parse markdown
    let parser = Parser::new(&md);
    let mut html = String::new();
    push_html(&mut html, parser);

    // Fill in to the template
    let mut data = BTreeMap::new();
    data.insert("content", html);
    Ok(Response::with((status::Ok, Template::new("default", data))))
}

/// Helper struct for handle 404 page of rust-kr
struct RustkrHandler<F> { handler: F }
/// Helper function for handle 404 page of rust-kr
fn catch<F>(handler: F) -> RustkrHandler<F> { RustkrHandler { handler: handler } }
impl<F> AfterMiddleware for RustkrHandler<F>
    where F: Send + Sync + 'static + Fn(IronError) -> IronResult<Response> {
    fn catch(&self, _: &mut Request, err: IronError) -> IronResult<Response> {
        (self.handler)(err)
    }
}
