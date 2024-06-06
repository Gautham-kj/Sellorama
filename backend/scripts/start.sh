#!/bin/bash

sqlx migrate run
./target/release/backend
