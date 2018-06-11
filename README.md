Simple Service - implemented in Rust
=====================================

For now a fairly simple service; `proto` folder contains domain
modelling of data and service. Running `cargo build` will create the `client`
and the `service`.

To run the service, have the following in the environment:

* MYSQL_URL : Url to the mysql instance and database, has the general form: `mysql://USERNAME:PASSWORD@hostname:3306/database`
* MAX_CACHE_SIZE : Number of items to cache internally (which it currently does not).

The `service` executable binds on IPv6 localhost interface [::1] on port 50051.

The `client` executable takes no environment variables but looks up the item with ident given by
the first commandline argument. It connects to IPv6 localhost interface [::1] port 50051.


Travis build status: [![Build Status](https://travis-ci.com/jakobdalsgaard/simple-tower-mysql-service.svg?branch=master)](https://travis-ci.com/jakobdalsgaard/simple-tower-mysql-service)
