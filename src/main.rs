// Copyright 2019 Federico Fissore
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[macro_use]
extern crate serde_derive;

use std::env;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::sync::Arc;

use actix_cors::Cors;
use actix_web::http::HeaderMap;
use actix_web::web;
use actix_web::App;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::HttpServer;
use maxminddb::geoip2::City;
use maxminddb::MaxMindDBError;
use maxminddb::Reader;
use memmap::Mmap;
use serde_json;

#[derive(Serialize)]
struct NonResolvedIPResponse<'a> {
    pub ip_address: &'a str,
}

#[derive(Serialize)]
struct ResolvedIPResponse<'a> {
    pub ip_address: &'a str,
    pub latitude: &'a f64,
    pub longitude: &'a f64,
    pub postal_code: &'a str,
    pub continent_code: &'a str,
    pub continent_name: &'a str,
    pub country_code: &'a str,
    pub country_name: &'a str,
    pub region_code: &'a str,
    pub region_name: &'a str,
    pub province_code: &'a str,
    pub province_name: &'a str,
    pub city_name: &'a str,
    pub timezone: &'a str,
}

#[derive(Deserialize, Debug)]
struct QueryParams {
    ip: Option<String>,
    lang: Option<String>,
    callback: Option<String>,
}

fn ip_address_to_resolve(
    query: &QueryParams,
    headers: &HeaderMap,
    remote_addr: Option<&str>,
) -> String {
    query
        .ip
        .as_ref()
        .filter(|ip_address| {
            ip_address.parse::<Ipv4Addr>().is_ok() || ip_address.parse::<Ipv6Addr>().is_ok()
        })
        .map(|s| s.to_owned())
        .or_else(|| {
            headers
                .get("X-Real-IP")
                .map(|s| s.to_str().unwrap().to_string())
        })
        .or_else(|| {
            remote_addr
                .map(|ip_port| ip_port.split(':').take(1).last().unwrap())
                .map(|ip| ip.to_string())
        })
        .expect("unable to find ip address to resolve")
}

fn get_language(query: &QueryParams) -> String {
    query
        .lang
        .as_ref()
        .map(|s| s.to_owned())
        .unwrap_or_else(|| String::from("en"))
}

struct Db {
    db: Arc<Reader<Mmap>>,
}

async fn index(req: HttpRequest, data: web::Data<Db>, web::Query(query): web::Query<QueryParams>) -> HttpResponse {
    //let query = Query::<QueryParams>::extract(&req).await.unwrap();

    let language = get_language(&query);
    let ip_address = ip_address_to_resolve(&query, req.headers(), req.connection_info().remote());

    let lookup: Result<City, MaxMindDBError> = data.db.lookup(ip_address.parse().unwrap());

    let geoip = match lookup {
        Ok(geoip) => {
            let region = geoip
                .subdivisions
                .as_ref()
                .filter(|subdivs| !subdivs.is_empty())
                .and_then(|subdivs| subdivs.get(0));

            let province = geoip
                .subdivisions
                .as_ref()
                .filter(|subdivs| subdivs.len() > 1)
                .and_then(|subdivs| subdivs.get(1));

            let res = ResolvedIPResponse {
                ip_address: &ip_address,
                latitude: geoip
                    .location
                    .as_ref()
                    .and_then(|loc| loc.latitude.as_ref())
                    .unwrap_or(&0.0),
                longitude: geoip
                    .location
                    .as_ref()
                    .and_then(|loc| loc.longitude.as_ref())
                    .unwrap_or(&0.0),
                postal_code: geoip
                    .postal
                    .as_ref()
                    .and_then(|postal| postal.code.as_ref())
                    .map(String::as_str)
                    .unwrap_or(""),
                continent_code: geoip
                    .continent
                    .as_ref()
                    .and_then(|cont| cont.code.as_ref())
                    .map(String::as_str)
                    .unwrap_or(""),
                continent_name: geoip
                    .continent
                    .as_ref()
                    .and_then(|cont| cont.names.as_ref())
                    .and_then(|names| names.get(&language))
                    .map(String::as_str)
                    .unwrap_or(""),
                country_code: geoip
                    .country
                    .as_ref()
                    .and_then(|country| country.iso_code.as_ref())
                    .map(String::as_str)
                    .unwrap_or(""),
                country_name: geoip
                    .country
                    .as_ref()
                    .and_then(|country| country.names.as_ref())
                    .and_then(|names| names.get(&language))
                    .map(String::as_str)
                    .unwrap_or(""),
                region_code: region
                    .and_then(|subdiv| subdiv.iso_code.as_ref())
                    .map(String::as_ref)
                    .unwrap_or(""),
                region_name: region
                    .and_then(|subdiv| subdiv.names.as_ref())
                    .and_then(|names| names.get(&language))
                    .map(String::as_ref)
                    .unwrap_or(""),
                province_code: province
                    .and_then(|subdiv| subdiv.iso_code.as_ref())
                    .map(String::as_ref)
                    .unwrap_or(""),
                province_name: province
                    .and_then(|subdiv| subdiv.names.as_ref())
                    .and_then(|names| names.get(&language))
                    .map(String::as_ref)
                    .unwrap_or(""),
                city_name: geoip
                    .city
                    .as_ref()
                    .and_then(|city| city.names.as_ref())
                    .and_then(|names| names.get(&language))
                    .map(String::as_str)
                    .unwrap_or(""),
                timezone: geoip
                    .location
                    .as_ref()
                    .and_then(|loc| loc.time_zone.as_ref())
                    .map(String::as_str)
                    .unwrap_or(""),
            };
            serde_json::to_string(&res).ok()
        }
        Err(_) => serde_json::to_string(&NonResolvedIPResponse {
            ip_address: &ip_address,
        })
        .ok(),
    }
    .unwrap();

    match &query.callback {
        Some(callback) => HttpResponse::Ok()
            .content_type("application/javascript; charset=utf-8")
            .body(format!(";{}({});", callback, geoip)),
        None => HttpResponse::Ok()
            .content_type("application/json; charset=utf-8")
            .body(geoip),
    }
}

fn db_file_path() -> String {
    let db_file_env_var = env::var("GEOIP_RS_DB_PATH");
    if db_file_env_var.is_ok() {
        return db_file_env_var.unwrap();
    }

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        return args[1].to_string();
    }

    panic!("You must specify the db path, either as a command line argument or as GEOIP_RS_DB_PATH env var");
}
#[actix_rt::main]
async fn main() {
    dotenv::from_path(".env").ok();

    let host = env::var("GEOIP_RS_HOST").unwrap_or_else(|_| String::from("127.0.0.1"));
    let port = env::var("GEOIP_RS_PORT").unwrap_or_else(|_| String::from("3000"));

    println!("Listening on http://{}:{}", host, port);

    let db = Arc::new(Reader::open_mmap(db_file_path()).unwrap());

    HttpServer::new(move || {
        App::new()
            .data(Db { db: db.clone() })
            .wrap(Cors::new().send_wildcard().finish())
            .route("/", web::route().to(index))
    })
    .bind(format!("{}:{}", host, port))
    .unwrap_or_else(|_| panic!("Can not bind to {}:{}", host, port))
    .run()
    .await
    .unwrap();
}
