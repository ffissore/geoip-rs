# geoip-rs

[![Latest version](https://img.shields.io/crates/v/geoip-rs.svg)](https://crates.io/crates/geoip-rs)
[![Build Status](https://travis-ci.com/ffissore/geoip-rs.svg?branch=master)](https://travis-ci.com/ffissore/geoip-rs)

geoip-rs is a geoip service: it provides geographical information about the calling or the specified IP address.

When called with no query params, it resolves the calling IP address. For example: https://api.geoip.rs

When called with the `ip` query param, it resolves the specified IP address. For example: https://api.geoip.rs/?ip=216.58.205.132

If the provided IP address is invalid, it falls back to the calling IP address.

### Example response

```json
{
  "ip_address": "46.51.179.90",
  "latitude": 53.3331,
  "longitude": -6.2489,
  "postal_code": "D02",
  "continent_code": "EU",
  "continent_name": "Europe",
  "country_code": "IE",
  "country_name": "Ireland",
  "region_code": "L",
  "region_name": "Leinster",
  "province_code": "",
  "province_name": "",
  "city_name": "Dublin",
  "timezone": "Europe/Dublin"
}
```

### Speed

On an 8 cores Intel i7, geoip.rs can serve between ~20K and ~50K requests/sec, depending on the requested ip address.

### Running

Once built with `cargo build --release`, run it with `./target/release/geoip-rs`. By default the english location dataset is loaded.

You can specify different datasets as command line arguments, for example
```bash
./target/release/geoip-rs data/GeoLite2-City-Blocks-IPv4.csv data/GeoLite2-City-Locations-ja.csv
```
or environment variables, for example
```bash
export GEOIP_RS_BLOCKS_FILE_PATH=./data/GeoLite2-City-Blocks-IPv4.csv
export GEOIP_RS_LOCATIONS_FILE_PATH=./data/GeoLite2-City-Locations-ja.csv
./target/release/geoip-rs
```
 
### Datasets

geoip-rs uses the free datasets provided by [maxmind](https://www.maxmind.com). They are not bundled: you have to download them separately.

Download "GeoLite2 City" dataset in CSV format from [here](https://dev.maxmind.com/geoip/geoip2/geolite2/#Downloads) and unzip it into the `data` folder.

### License

This project is licensed under the Apache License, Version 2.0
