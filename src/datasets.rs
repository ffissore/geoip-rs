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

extern crate csv;

use std::io;

use csv::Reader;

use self::csv::Error;

#[derive(Debug, Deserialize)]
struct RawBlock {
    pub network: String,
    pub geoname_id: Option<u32>,
    pub postal_code: String,
    pub latitude: Option<f32>,
    pub longitude: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub network: String,
    pub geoname_id: u32,
    pub postal_code: String,
    pub latitude: f32,
    pub longitude: f32,
}

pub fn parse_blocks_csv<R: io::Read>(source: R) -> impl Iterator<Item=Block> {
    let reader = Reader::from_reader(source);

    reader.into_deserialize()
        .map(|result: Result<RawBlock, Error>| result.unwrap())
        .filter(|record| record.geoname_id.is_some() && record.latitude.is_some() && record.longitude.is_some())
        .map(|rawblock| Block {
            network: rawblock.network,
            geoname_id: rawblock.geoname_id.unwrap(),
            postal_code: rawblock.postal_code,
            latitude: rawblock.latitude.unwrap(),
            longitude: rawblock.longitude.unwrap(),
        })
}

#[derive(Debug, Deserialize)]
pub struct Location {
    pub geoname_id: u32,
    pub continent_code: String,
    pub continent_name: String,
    #[serde(rename = "country_iso_code")]
    pub country_code: String,
    pub country_name: String,
    #[serde(rename = "subdivision_1_iso_code")]
    pub region_code: String,
    #[serde(rename = "subdivision_1_name")]
    pub region_name: String,
    #[serde(rename = "subdivision_2_iso_code")]
    pub province_code: String,
    #[serde(rename = "subdivision_2_name")]
    pub province_name: String,
    pub city_name: String,
    #[serde(rename = "time_zone")]
    pub timezone: String,
}

pub fn parse_locations_csv<R: io::Read>(source: R) -> impl Iterator<Item=Location> {
    let reader = Reader::from_reader(source);

    reader.into_deserialize()
        .map(|record: Result<Location, Error>| record.unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_read_blocks_csv() {
        let data = "
network,geoname_id,registered_country_geoname_id,represented_country_geoname_id,is_anonymous_proxy,is_satellite_provider,postal_code,latitude,longitude,accuracy_radius
1.0.0.0/24,2077456,2077456,,0,0,,-33.4940,143.2104,1000
1.0.1.0/24,1811017,1814991,,0,0,,24.4798,118.0819,50
5.145.149.142/32,,6252001,,0,1,,,,";

        let blocks = parse_blocks_csv(data.as_bytes()).collect::<Vec<Block>>();
        assert_eq!(2, blocks.len());

        let block = blocks.get(0).unwrap();
        assert_eq!("1.0.0.0/24", block.network);
        assert_eq!(2077456, block.geoname_id);
        assert_eq!("", block.postal_code);
        assert_eq!(-33.4940, block.latitude);
        assert_eq!(143.2104, block.longitude);
    }

    #[test]
    fn can_read_locations_csv() {
        let data = "
geoname_id,locale_code,continent_code,continent_name,country_iso_code,country_name,subdivision_1_iso_code,subdivision_1_name,subdivision_2_iso_code,subdivision_2_name,city_name,metro_code,time_zone,is_in_european_union
5819,en,EU,Europe,CY,Cyprus,02,Limassol,,,Souni,,Asia/Nicosia,1
49518,en,AF,Africa,RW,Rwanda,,,,,,,Africa/Kigali,0
49747,en,AF,Africa,SO,Somalia,BK,Bakool,,,Oddur,,Africa/Mogadishu,0
51537,en,AF,Africa,SO,Somalia,,,,,,,Africa/Mogadishu,0";

        let locations = parse_locations_csv(data.as_bytes()).collect::<Vec<Location>>();
        assert_eq!(4, locations.len());

        let location = locations.get(0).unwrap();
        assert_eq!(5819, location.geoname_id);
        assert_eq!("EU", location.continent_code);
        assert_eq!("Europe", location.continent_name);
        assert_eq!("CY", location.country_code);
        assert_eq!("Cyprus", location.country_name);
        assert_eq!("02", location.region_code);
        assert_eq!("Limassol", location.region_name);
        assert_eq!("", location.province_code);
        assert_eq!("", location.province_name);
        assert_eq!("Souni", location.city_name);
        assert_eq!("Asia/Nicosia", location.timezone);
    }
}
