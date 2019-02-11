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

//! geoip_rs is a library used to load geoip database provided by maxmind and associate ip addresses
//! with geographical information.
//!
//! In order to be fast, the whole dataset is loaded into memory: it consumes ~300 MB of RAM. This
//! allows the http server of the binary crate to serve ~50K requests/sec on an 8 cores Intel i7
#[macro_use]
extern crate serde_derive;

use std::collections::HashMap;
use std::io::Read;
use std::net::Ipv4Addr;

use ipnet::Ipv4Net;

use crate::datasets::{Block, Location};

mod datasets;

/// GeoIPDB is the struct holding both blocks (ip networks and their coordinates) and locations
/// (contintent, country, etc corresponding to some coordinates)
#[derive(Debug)]
pub struct GeoIPDB {
    locations: HashMap<u32, Location>,
    blocks: HashMap<u32, Vec<Block>>,
}

impl GeoIPDB {
    /// Given a V4 ip network with a prefix lower than 16, it will expand it the corresponding networks with prefix 16
    /// If instead the given network has a prefix greater or equal 16, no expansion occurs
    fn expand_network(network: &Ipv4Net) -> Vec<u32> {
        let prefix = network.prefix_len();

        let expanded_networks;
        if prefix < 16 {
            expanded_networks = network
                .subnets(16)
                .unwrap()
                .map(|network| GeoIPDB::ipnet_to_map_key(&network))
                .collect();
        } else {
            expanded_networks = vec![GeoIPDB::ipnet_to_map_key(network)];
        }

        expanded_networks
    }

    /// Translates a V4 ip network into a u32 suitable to be used as key in the hashmap held by GeoIPDB
    fn ipnet_to_map_key(ip_address: &Ipv4Net) -> u32 {
        GeoIPDB::ipaddr_to_map_key(&ip_address.addr())
    }

    /// Translates a V4 ip address into a u32 suitable to be used as key in the hashmap held by GeoIPDB
    fn ipaddr_to_map_key(ip_address: &Ipv4Addr) -> u32 {
        ip_address.octets()[0..2]
            .iter()
            .map(|n| u32::from(*n))
            .scan(1_000, |state, value| {
                let res = *state * value;
                *state = *state / 1000;
                Some(res)
            })
            .map(|n| u32::from(n))
            .sum()
    }

    /// Creates a new GeoIPDB by parsing and loading the contents of a blocks CSV file and a location CSV file
    pub fn new<R: Read + Sized>(blocks_csv_file: R, locations_csv_file: R) -> Self {
        let mut blocks = HashMap::new();

        datasets::parse_blocks_csv(blocks_csv_file)
            .map(|block| {
                let networks = GeoIPDB::expand_network(&block.network);

                (block, networks)
            })
            .for_each(|(block, networks)| {
                networks.iter().for_each(|network| {
                    let blocks = blocks.entry(*network).or_insert(Vec::new());
                    blocks.push(block.clone());
                });
            });

        let mut locations = HashMap::new();

        datasets::parse_locations_csv(locations_csv_file).for_each(|location| {
            locations.insert(location.geoname_id, location);
        });

        GeoIPDB { locations, blocks }
    }

    /// Looks for the given ip address in the db, returning the corresponding block, if any
    pub fn resolve(&self, ip_address: &str) -> Option<&Block> {
        let ip_address = ip_address.parse::<Ipv4Addr>().unwrap();
        let candidates = self.blocks.get(&GeoIPDB::ipaddr_to_map_key(&ip_address));

        candidates.and_then(|candidates| {
            candidates
                .iter()
                .find(|block| block.network.contains(&ip_address))
        })
    }

    /// Returns the location corresponding to the given id
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
        assert_eq!(
            255255,
            GeoIPDB::ipnet_to_map_key(&"255.255.255.0/24".parse::<Ipv4Net>().unwrap())
        );
        assert_eq!(
            255255,
            GeoIPDB::ipaddr_to_map_key(&"255.255.255.12".parse::<Ipv4Addr>().unwrap())
        );
        assert_eq!(
            1000,
            GeoIPDB::ipaddr_to_map_key(&"1.0.0.1".parse::<Ipv4Addr>().unwrap())
        );
        assert_eq!(
            81030,
            GeoIPDB::ipaddr_to_map_key(&"81.30.9.30".parse::<Ipv4Addr>().unwrap())
        );
    }

    #[test]
    fn can_resolve_ip() {
        let blocks = "network,geoname_id,registered_country_geoname_id,represented_country_geoname_id,is_anonymous_proxy,is_satellite_provider,postal_code,latitude,longitude,accuracy_radius
1.0.0.0/24,2077456,2077456,,0,0,,-33.4940,143.2104,1000
1.0.1.0/24,1811017,1814991,,0,0,,24.4798,118.0819,50
1.3.0.0/16,1809935,1814991,,0,0,,23.1167,113.2500,50";

        let locations = "geoname_id,locale_code,continent_code,continent_name,country_iso_code,country_name,subdivision_1_iso_code,subdivision_1_name,subdivision_2_iso_code,subdivision_2_name,city_name,metro_code,time_zone,is_in_european_union
1809935,en,AS,Asia,CN,China,GD,Guangdong,,,,,Asia/Shanghai,0
49518,en,AF,Africa,RW,Rwanda,,,,,,,Africa/Kigali,0";

        let geoip_db = GeoIPDB::new(blocks.as_bytes(), locations.as_bytes());

        let block = geoip_db.resolve("1.3.4.2").unwrap();
        assert_eq!("1.3.0.0/16", block.network.to_string());
        assert_eq!(1809935, block.geoname_id);
        assert_eq!("", block.postal_code);
        assert_eq!(23.1167, block.latitude);
        assert_eq!(113.25, block.longitude);

        let location = geoip_db.get_location(block.geoname_id);
        assert_eq!(1809935, location.geoname_id);
        assert_eq!("AS", location.continent_code);
        assert_eq!("Asia", location.continent_name);
        assert_eq!("CN", location.country_code);
        assert_eq!("China", location.country_name);
        assert_eq!("GD", location.region_code);
        assert_eq!("Guangdong", location.region_name);
        assert_eq!("", location.province_code);
        assert_eq!("", location.province_name);
        assert_eq!("", location.city_name);
        assert_eq!("Asia/Shanghai", location.timezone);
    }

    #[test]
    fn cannot_resolve_ip() {
        let geoip_db = GeoIPDB::new("".as_bytes(), "".as_bytes());
        assert_eq!(true, geoip_db.resolve("1.2.3.4").is_none());
    }
}
