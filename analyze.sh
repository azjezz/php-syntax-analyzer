#!/bin/bash
set -e

echo "Building in release mode..."
cargo build --release

echo "Downloading packages 1000-1500..."
./target/release/php-syntax-analyzer --keyword using --min 1000 --max 1500 > /dev/null

echo "Analyzing for 'let' keyword..."
./target/release/php-syntax-analyzer --keyword let --skip-download > results/let.txt

echo "Analyzing for 'scope' keyword..."
./target/release/php-syntax-analyzer --keyword scope --skip-download > results/scope.txt

echo "Analyzing for 'using' keyword..."
./target/release/php-syntax-analyzer --keyword using --skip-download > results/using.txt

echo "Done! Results saved in results/"
