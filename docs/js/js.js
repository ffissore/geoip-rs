"use strict";

const endpoint = "//api.geoip.rs";

fetch(endpoint)
  .then(res => Promise.all([res.url, res.json()]))
  .then(([url, obj]) => {
      document.querySelector(".simple_example").innerHTML = `curl ${url}\n\n${JSON.stringify(obj, null, 2)}`;
      return obj.ip_address;
  })
  .then(ip_address => {
      return Promise.all([
          fetch(`${endpoint}?ip=${ip_address}`)
            .then(res => Promise.all([res.url, res.json()]))
            .then(([url, obj]) => {
                document.querySelector(".specified_example").innerHTML = `curl ${url}\n\n${JSON.stringify(obj, null, 2)}`;
            }),
          fetch(`${endpoint}?ip=${ip_address}&callback=my_function`)
            .then(res => Promise.all([res.url, res.text()]))
            .then(([url, text]) => {
                document.querySelector(".jsonp_example").innerHTML = `curl ${url}\n\n`;
                eval(text);
            })
      ])
  });

function my_function(obj) {
    document.querySelector(".jsonp_example").append(`my_function(${JSON.stringify(obj, null, 2)})`);
}