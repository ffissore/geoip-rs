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

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use ipaddress::IPAddress;
use log::info;

use crate::datasets::{Block, Location};

mod datasets;

#[derive(Debug)]
pub struct GeoIPDB {
    locations: HashMap<u32, Location>,
    blocks: HashMap<u32, Vec<Arc<Block>>>,
}

impl GeoIPDB {
    fn expand_network(network: &str) -> Vec<u32> {
        let netmask = u8::from_str_radix(network.split("/").last().unwrap(), 10).unwrap();

        let expanded_networks;
        if netmask < 16 {
            expanded_networks = IPAddress::parse(network).unwrap()
                .subnet(16).unwrap()
                .iter()
                .map(|network| GeoIPDB::ip_to_map_key(&network.to_string()))
                .collect();
        } else {
            expanded_networks = vec![GeoIPDB::ip_to_map_key(network)];
        }

        expanded_networks
    }

    fn ip_to_map_key(ip_address: &str) -> u32 {
        ip_address
            .split(".")
            .take(2)
            .map(|part| u32::from_str_radix(part, 10).unwrap())
            .scan(1_000, |state, value| {
                let res = *state * value;
                *state = *state / 1000;
                Some(res)
            })
            .sum()
    }

    pub fn new(blocks_csv_file: &str, locations_csv_file: &str) -> Self {
        info!("Loading IPV4 networks dataset from {}...", blocks_csv_file);
        let blocks_csv_file = File::open(Path::new(blocks_csv_file)).unwrap();
        let mut blocks = HashMap::new();

        datasets::parse_blocks_csv(blocks_csv_file)
            .map(|block| {
                let networks = GeoIPDB::expand_network(&block.network);

                let block = Arc::new(block);

                (block, networks)
            })
            .for_each(|(block, networks)| {
                networks.iter()
                    .for_each(|network| {
                        let geo_ips = blocks.entry(*network).or_insert(Vec::new());
                        geo_ips.push(Arc::clone(&block));
                    });
            });

        info!("Loading IP location dataset from {}...", locations_csv_file);
        let locations_csv_file = File::open(Path::new(locations_csv_file)).unwrap();
        let mut locations = HashMap::new();

        datasets::parse_locations_csv(locations_csv_file)
            .for_each(|location| {
                locations.insert(location.geoname_id, location);
            });

        GeoIPDB {
            locations,
            blocks,
        }
    }

    pub fn resolve(&self, ip_address: &str) -> Option<&Arc<Block>> {
        let candidates = self.blocks.get(&GeoIPDB::ip_to_map_key(ip_address));
        let ip_address = IPAddress::parse(ip_address).unwrap();

        candidates.and_then(|candidates| {
            candidates.iter()
                .find(|geo_ip| {
                    IPAddress::parse(geo_ip.network.as_str()).unwrap().includes(&ip_address)
                })
        })
    }

    pub fn get_location(&self, geoname_id: u32) -> &Location {
        self.locations.get(&geoname_id).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use ipaddress::IPAddress;

    use super::*;

    #[test]
    #[ignore]
    fn ipaddress_expansion() {
        let ip1 = IPAddress::parse("172.16.0.0/26").unwrap();
        assert_eq!(true, ip1.is_network());
        ip1.each(|ip| println!("{:?}", ip));
        let ip1 = IPAddress::parse("172.16.0.128/26").unwrap();
        assert_eq!(true, ip1.is_network());
        ip1.each(|ip| println!("{:?}", ip));
    }

    #[test]
    fn ip_to_number() {
        assert_eq!(255255, GeoIPDB::ip_to_map_key("255.255.255.12"));
        assert_eq!(1000, GeoIPDB::ip_to_map_key("1.0.0.1"));
        assert_eq!(81030, GeoIPDB::ip_to_map_key("81.30.9.30"));
    }
}
