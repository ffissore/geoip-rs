# geoip-rs

[![Latest version](https://img.shields.io/crates/v/geoip-rs.svg)](https://crates.io/crates/geoip-rs)
[![Build Status](https://travis-ci.org/ffissore/geoip-rs.svg?branch=master)](https://travis-ci.org/ffissore/geoip-rs)

geoip-rs is a geoip service: it provides geographical information about the calling or the specified IP address. It supports both IPV4 and IPV6.

When called with no query params, it resolves the calling IP address. For example: https://api.geoip.rs

When called with the `ip` query param, it resolves the specified IP address. For example: https://api.geoip.rs/?ip=216.58.205.132

If the provided IP address is invalid, it falls back to the calling IP address.

By default, responses will be in english. An optional `lang` query param can be provided: if a translation is available, returned data will be in that language. Current list includes: `de`, `en`, `es`, `fr`, `ja`, `pt-BR`, `ru`, `zh-CH`.

### Example response

Valid ip address:
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

Not found (private) ip address:
```json
{
  "ip_address": "127.0.0.1"
}
```

### Speed

On an 8 cores Intel i7, geoip.rs can serve ~30K requests/sec.
 
### Dataset

geoip-rs uses the free dataset provided by [maxmind](https://www.maxmind.com). It's not bundled: you have to download it separately.

Download "GeoLite2 City" dataset in binary format from [here](https://dev.maxmind.com/geoip/geoip2/geolite2/#Downloads) and unzip it.

### Running

Install geoip.rs with 

```bash
cargo install geoip-rs
```

If you don't have `cargo`, install it with
 
```bash
curl https://sh.rustup.rs -sSf | sh
```

or [read the tutorial](https://doc.rust-lang.org/cargo/getting-started/installation.html) for additional instructions. 

You can specify the dataset location on the command line
```bash
geoip-rs /path/to/GeoLite2-City.mmdb
```
or via environment variable
```bash
export GEOIP_RS_DB_PATH=/path/to/GeoLite2-City.mmdb
geoip-rs
```

You can also customize the host and port geoip.rs will listen to
```bash
export GEOIP_RS_DB_PATH=/path/to/GeoLite2-City.mmdb
export GEOIP_RS_HOST=192.168.0.1
export GEOIP_RS_PORT=8080
geoip-rs
```

### License

This project is licensed under the Apache License, Version 2.0
