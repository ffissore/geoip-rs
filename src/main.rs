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
extern crate lazy_static;
extern crate regex;
#[macro_use]
extern crate serde_derive;

use std::env;
use std::fs::File;
use std::io;
use std::path::Path;

use iron::Handler;
use iron::headers::ContentType;
use iron::prelude::*;
use iron::status;
use iron_cors::CorsMiddleware;
use log::info;
use log::LevelFilter;
use regex::Regex;
use serde_json;
use urlencoded::UrlEncodedQuery;

use geoip_rs::GeoIPDB;

struct DatasetFiles {
    blocks: String,
    locations: String,
}

impl DatasetFiles {
    fn new() -> DatasetFiles {
        let args: Vec<String> = env::args().collect();

        let blocks_file_path_env = env::var("GEOIP_RS_BLOCKS_FILE_PATH");
        let blocks_file_path;
        if blocks_file_path_env.is_ok() {
            blocks_file_path = blocks_file_path_env.unwrap();
        } else if args.len() > 1 {
            blocks_file_path = args.get(1).unwrap().to_string();
        } else {
            blocks_file_path = String::from("./data/GeoLite2-City-Blocks-IPv4.csv");
        }

        let locations_file_path_env = env::var("GEOIP_RS_LOCATIONS_FILE_PATH");
        let locations_file_path;
        if locations_file_path_env.is_ok() {
            locations_file_path = locations_file_path_env.unwrap();
        } else if args.len() > 2 {
            locations_file_path = args.get(2).unwrap().to_string();
        } else {
            locations_file_path = String::from("./data/GeoLite2-City-Locations-en.csv");
        }

        DatasetFiles {
            blocks: blocks_file_path,
            locations: locations_file_path,
        }
    }
}

#[derive(Serialize)]
struct NonResolvedIPResponse<'a> {
    pub ip_address: &'a str,
}

#[derive(Serialize)]
struct ResolvedIPResponse<'a> {
    pub ip_address: &'a str,
    pub latitude: f32,
    pub longitude: f32,
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
    db: GeoIPDB,
}

impl ResolveIPHandler {
    fn get_query_param(req: &mut Request, param: &str) -> Option<String> {
        req.get_ref::<UrlEncodedQuery>().ok()
            .and_then(|query_params| {
                query_params.get(param)
            })
            .and_then(|params| {
                params.get(0)
            })
            .map(|param| {
                param.to_string()
            })
    }

    fn get_header_value(req: &Request, header_name: &str) -> Option<String> {
        req.headers.iter()
            .find(|header| header.name().eq(header_name))
            .map(|header| header.value_string())
    }

    fn ip_address_to_resolve(req: &mut Request) -> String {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$").unwrap();
        }

        ResolveIPHandler::get_query_param(req, "ip")
            .filter(|ipaddress| RE.is_match(ipaddress))
            .or_else(|| ResolveIPHandler::get_header_value(req, "X-Real-IP"))
            .unwrap_or(req.remote_addr.ip().to_string())
    }
}

impl Handler for ResolveIPHandler {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let ip_address = ResolveIPHandler::ip_address_to_resolve(req);

        let geoip = self.db.resolve(&ip_address)
            .map(|block| {
                let location = self.db.get_location(block.geoname_id);
                ResolvedIPResponse {
                    ip_address: &ip_address,
                    latitude: block.latitude,
                    longitude: block.longitude,
                    postal_code: &block.postal_code,
                    continent_code: &location.continent_code,
                    continent_name: &location.continent_name,
                    country_code: &location.country_code,
                    country_name: &location.country_name,
                    region_code: &location.region_code,
                    region_name: &location.region_name,
                    province_code: &location.province_code,
                    province_name: &location.province_name,
                    city_name: &location.city_name,
                    timezone: &location.timezone,
                }
            })
            .and_then(|geoip| serde_json::to_string(&geoip).ok())
            .or(serde_json::to_string(&NonResolvedIPResponse { ip_address: &ip_address }).ok())
            .unwrap();

        let res = match ResolveIPHandler::get_query_param(req, "callback") {
            Some(callback) => {
                let mut res = Response::with((status::Ok, format!("{}({})", callback, geoip)));
                res.headers.set(ContentType("application/javascript".parse().unwrap()));
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

fn main() {
    simple_logging::log_to(io::stdout(), LevelFilter::Info);

    let dataset_paths = DatasetFiles::new();

    info!("Loading datasets: IPV4 networks dataset from {} and locations from {}", &dataset_paths.blocks, &dataset_paths.locations);

    let geoipdb = GeoIPDB::new(
        File::open(Path::new(&dataset_paths.blocks)).unwrap(),
        File::open(Path::new(&dataset_paths.locations)).unwrap(),
    );

    let mut chain = Chain::new(ResolveIPHandler {
        db: geoipdb,
    });
    chain.link_around(CorsMiddleware::with_allow_any());

    let _server = Iron::new(chain).http("127.0.0.1:3000").unwrap();
    println!("On 3000");
}
