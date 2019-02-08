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
use std::net::Ipv4Addr;
use std::path::Path;

use ipnet::Ipv4Net;
use log::info;

use crate::datasets::{Block, Location};

mod datasets;

#[derive(Debug)]
pub struct GeoIPDB {
    locations: HashMap<u32, Location>,
    blocks: HashMap<u32, Vec<Block>>,
}

impl GeoIPDB {
    fn expand_network(network: &Ipv4Net) -> Vec<u32> {
        let prefix = network.prefix_len();

        let expanded_networks;
        if prefix < 16 {
            expanded_networks = network.subnets(16).unwrap()
                .map(|network| GeoIPDB::ipnet_to_map_key(&network))
                .collect();
        } else {
            expanded_networks = vec![GeoIPDB::ipnet_to_map_key(network)];
        }

        expanded_networks
    }

    fn ipnet_to_map_key(ip_address: &Ipv4Net) -> u32 {
        GeoIPDB::ipaddr_to_map_key(&ip_address.addr())
    }

    fn ipaddr_to_map_key(ip_address: &Ipv4Addr) -> u32 {
        ip_address.octets()[0..2].iter()
            .map(|n| u32::from(*n))
            .scan(1_000, |state, value| {
                let res = *state * value;
                *state = *state / 1000;
                Some(res)
            })
            .map(|n| u32::from(n))
            .sum()
    }

    pub fn new(blocks_csv_file: &str, locations_csv_file: &str) -> Self {
        info!("Loading IPV4 networks dataset from {}...", blocks_csv_file);
        let blocks_csv_file = File::open(Path::new(blocks_csv_file)).unwrap();
        let mut blocks = HashMap::new();

        datasets::parse_blocks_csv(blocks_csv_file)
            .map(|block| {
                let networks = GeoIPDB::expand_network(&block.network);

                (block, networks)
            })
            .for_each(|(block, networks)| {
                networks.iter()
                    .for_each(|network| {
                        let blocks = blocks.entry(*network).or_insert(Vec::new());
                        blocks.push(block.clone());
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

    pub fn resolve(&self, ip_address: &str) -> Option<&Block> {
        let ip_address = ip_address.parse::<Ipv4Addr>().unwrap();
        let candidates = self.blocks.get(&GeoIPDB::ipaddr_to_map_key(&ip_address));

        candidates.and_then(|candidates| {
            candidates.iter()
                .find(|block| {
                    block.network.contains(&ip_address)
                })
        })
    }

    pub fn get_location(&self, geoname_id: u32) -> &Location {
        self.locations.get(&geoname_id).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn ipaddress_expansion() {
        let ip1 = "172.16.0.0/26".parse::<Ipv4Net>().unwrap();
        ip1.subnets(16).unwrap().for_each(|ip| println!("{:?}", ip));
        let ip1 = "172.16.0.128/26".parse::<Ipv4Net>().unwrap();
        ip1.subnets(16).unwrap().for_each(|ip| println!("{:?}", ip));
    }

    #[test]
    fn ip_to_number() {
        assert_eq!(255255, GeoIPDB::ipnet_to_map_key(&"255.255.255.0/24".parse::<Ipv4Net>().unwrap()));
        assert_eq!(255255, GeoIPDB::ipaddr_to_map_key(&"255.255.255.12".parse::<Ipv4Addr>().unwrap()));
        assert_eq!(1000, GeoIPDB::ipaddr_to_map_key(&"1.0.0.1".parse::<Ipv4Addr>().unwrap()));
        assert_eq!(81030, GeoIPDB::ipaddr_to_map_key(&"81.30.9.30".parse::<Ipv4Addr>().unwrap()));
    }
}
