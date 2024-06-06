#!/bin/bash

sqlx migrate run
cargo run --bin backend
