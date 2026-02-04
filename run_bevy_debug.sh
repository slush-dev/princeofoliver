#!/bin/bash

cd `dirname $0`/bevy
~/.cargo/bin/cargo run -- --gpu --labels
