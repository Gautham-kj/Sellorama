#!/bin/bash

sqlx migrate run
touch src/lib.rs
cargo run
