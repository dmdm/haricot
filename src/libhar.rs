#![allow(non_snake_case)]

use serde_json;
use std::cmp;
use std::error::Error;
use std::fs::File;
use std::path::Path;
use url::percent_encoding::percent_decode;
use url::Url;

#[derive(Serialize, Deserialize, Debug)]
pub struct Creator {
    pub name: String,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NameValue {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PostData {
    pub mimeType: String,
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Request {
    pub method: String,
    pub url: String,
    pub httpVersion: String,
    pub headers: Vec<NameValue>,
    pub queryString: Vec<NameValue>,
    pub cookies: serde_json::Value,
    pub headersSize: usize,
    pub bodySize: usize,
    #[serde(default)]
    pub postData: Option<PostData>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseContent {
    pub size: usize,
    pub mimeType: String,
    pub compression: isize,
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    pub status: u16,
    pub statusText: String,
    pub httpVersion: String,
    pub headers: Vec<NameValue>,
    pub cookies: serde_json::Value,
    pub content: ResponseContent,
    pub redirectURL: String,
    pub headersSize: usize,
    pub bodySize: usize,
    pub _transferSize: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Entry {
    pub startedDateTime: String,
    pub time: f64,
    pub request: Request,
    pub response: Response,
    pub cache: serde_json::Value,
    pub timings: serde_json::Value,
    pub serverIPAddress: serde_json::Value,
    pub connection: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Log {
    pub version: String,
    pub creator: Creator,
    pub pages: Vec<String>,
    pub entries: Vec<Entry>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Doc {
    pub log: Log,
}

pub fn read_file<P: AsRef<Path>>(path: P) -> Result<Doc, Box<Error>> {
    let f = File::open(path)?;
    let doc: Doc = serde_json::from_reader(f)?;
    Ok(doc)
}

fn print_name_vals(nvs: &Vec<NameValue>, excludes: Option<&Vec<&str>>) {
    let mut sorted: Vec<&NameValue> = nvs.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));
    for nv in sorted.iter() {
        let mut show = true;
        if let Some(excl) = excludes {
            if excl.contains(&nv.name.as_str()) {
                show = false;
            }
        }
        if show {
            println!("        {:20} {}", format!("{}{}", nv.name, ":"), nv.value);
        }
    }
}

// Need to return String, not &str. Otherwise temporary value would not live long enough.
fn cut_text(text: &str, maxlen: usize) -> String {
    text[..cmp::min(maxlen, text.len())]
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
        .trim()
        .to_string()
}

fn expand_privates_2(pr: serde_json::Value) -> Result<serde_json::Value, Box<Error>> {
    if let Some(s) = pr.as_str() {
        let decoded_s = percent_decode(s.as_bytes()).decode_utf8()?;
        return Ok(serde_json::from_str(&decoded_s)?);
    };
    Ok(pr)
}

fn expand_privates(text: &str) -> Result<serde_json::Value, Box<Error>> {
    let mut doc: serde_json::Value = serde_json::from_str(text)?;
    // TODO Scan for arbitrary positions of private data, not only in 'AddDevice.DevicePrivateData'.
    // TODO Instead of cloning, use pointer_mut()
    let pr = doc["AddDevice"]["DevicePrivateData"].clone();
    if pr != serde_json::Value::Null {
        doc["AddDevice"]["DevicePrivateData"] = expand_privates_2(pr)?;
        return Ok(doc);
    }
    let pr = doc["Resource"]["Device"]["DevicePrivateData"].clone();
    if pr != serde_json::Value::Null {
        doc["Resource"]["Device"]["DevicePrivateData"] = expand_privates_2(pr)?;
        return Ok(doc);
    }
    return Ok(doc);
}

pub fn print_body(doc: &Doc, num: usize, which: &str, ecs: bool) {
    let e = &doc.log.entries[num];
    match which {
        "req" => {
            match e.request.postData {
                Some(ref data) => {
                    if ecs {
                        let text = expand_privates(&data.text).unwrap();
                        println!("{}", text);
                    } else {
                        println!("{}", data.text);
                    }
                }
                None => {
                    println!("Request {} has no post data", num);
                }
            }
        }
        "resp" => {
            if ecs {
                let text = expand_privates(&e.response.content.text).unwrap();
                println!("{}", text);
            } else {
                println!("{}", e.response.content.text);
            }
        }
        _ => unreachable!()
    }
}

pub fn print_overview(
    doc: &Doc,
    short_url: bool,
    query_string_excludes: Option<&Vec<&str>>,
    headers_excludes: Option<&Vec<&str>>,
) -> Result<(), Box<Error>> {
    println!("{:?} entries", doc.log.entries.len());
    for (ix, e) in doc.log.entries.iter().enumerate() {
        let req = &e.request;
        let mut url = Url::parse(&req.url)?;
        if short_url {
            url.set_query(None);
        }
        println!("{}/ {} {}", ix, req.method, url);
        if req.queryString.len() > 0 {
            println!("    Query String:");
            print_name_vals(&req.queryString, query_string_excludes);
        }
        if req.headers.len() > 0 {
            println!("    Headers:");
            print_name_vals(&req.headers, headers_excludes);
        }
        if let Some(ref pd) = req.postData {
            println!("    Post Data:");
            println!("        Mime-Type:           {}", pd.mimeType);
            println!("        Length:              {}", pd.text.len());
            println!("        Text:                {}…", cut_text(&pd.text, 80));
        }

        let resp = &e.response;
        println!(
            "{}/ RESPONSE:                 {} {}",
            ix, resp.status, resp.statusText
        );
        if resp.headers.len() > 0 {
            println!("    Headers:");
            print_name_vals(&resp.headers, headers_excludes);
        }
        println!("    Content:");
        println!("        Mime-Type:           {}", resp.content.mimeType);
        println!("        Size:                {}", resp.content.size);
        println!(
            "        Text:                {}…",
            cut_text(&resp.content.text, 80)
        );

        println!("\n\n");
    }

    Ok(())
}
