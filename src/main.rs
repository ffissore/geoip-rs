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

extern crate iron;
#[macro_use]
extern crate serde_derive;

use std::env;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;

use iron::headers::ContentType;
use iron::prelude::*;
use iron::status;
use iron::Handler;
use iron_cors::CorsMiddleware;
use maxminddb::geoip2::City;
use maxminddb::MaxMindDBError;
use maxminddb::Reader;
use memmap::Mmap;
use serde_json;
use urlencoded::UrlEncodedQuery;

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

struct ResolveIPHandler {
    db: Reader<Mmap>,
}

impl ResolveIPHandler {
    fn get_query_param(req: &mut Request, param: &str) -> Option<String> {
        req.get_ref::<UrlEncodedQuery>()
            .ok()
            .and_then(|query_params| query_params.get(param))
            .and_then(|params| params.get(0))
            .map(|param| param.to_string())
    }

    fn get_header_value(req: &Request, header_name: &str) -> Option<String> {
        req.headers
            .iter()
            .find(|header| header.name().eq(header_name))
            .map(|header| header.value_string())
    }

    fn ip_address_to_resolve(req: &mut Request) -> String {
        Self::get_query_param(req, "ip")
            .filter(|ipaddress| {
                ipaddress.parse::<Ipv4Addr>().is_ok() || ipaddress.parse::<Ipv6Addr>().is_ok()
            })
            .or_else(|| Self::get_header_value(req, "X-Real-IP"))
            .unwrap_or(req.remote_addr.ip().to_string())
    }

    fn get_language(req: &mut Request) -> String {
        Self::get_query_param(req, "lang").unwrap_or(String::from("en"))
    }
}

impl Handler for ResolveIPHandler {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let language = Self::get_language(req);
        let ip_address = Self::ip_address_to_resolve(req);

        let lookup: Result<City, MaxMindDBError> = self.db.lookup(ip_address.parse().unwrap());
        let geoip = match lookup {
            Ok(geoip) => {
                let region = geoip
                    .subdivisions
                    .as_ref()
                    .filter(|subdivs| subdivs.len() > 0)
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

        let res = match Self::get_query_param(req, "callback") {
            Some(callback) => {
                let mut res = Response::with((status::Ok, format!("{}({})", callback, geoip)));
                res.headers
                    .set(ContentType("application/javascript".parse().unwrap()));
                res
            }
            None => {
                let mut res = Response::with((status::Ok, geoip));
                res.headers.set(ContentType::json());
                res
            }
        };

        Ok(res)
    }
}

fn db_file_path() -> String {
    let db_file_env_var = env::var("GEOIP_RS_DB_PATH");
    if db_file_env_var.is_ok() {
        return db_file_env_var.unwrap();
    }

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        return args.get(1).unwrap().to_string();
    }

    panic!("You must specify the db path, either as a command line argument or as GEOIP_RS_DB_PATH env var");
}

fn main() {
    let db_file = db_file_path();

    let db = Reader::open_mmap(db_file).unwrap();

    let mut chain = Chain::new(ResolveIPHandler { db });
    chain.link_around(CorsMiddleware::with_allow_any());

    let host = env::var("GEOIP_RS_HOST").unwrap_or(String::from("127.0.0.1"));
    let port = env::var("GEOIP_RS_PORT").unwrap_or(String::from("3000"));
    let _server = Iron::new(chain).http(format!("{}:{}", host, port)).unwrap();
    println!("Listening on {}:{}", host, port);
}
