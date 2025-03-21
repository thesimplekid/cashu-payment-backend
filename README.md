# Cashu Payment Backend

A backend service for generating and accepting [Cashu](https://cashu.space/) NUT-18 ecash payment requests.

## Overview

Cashu Payment Backend provides a simple HTTP API for merchants to generate and process Cashu token payments. It implements [NUT-18](https://github.com/cashubtc/nuts/blob/main/18.md) payment and supports multiple mints and currencies (SAT and USD), making it flexible for various use cases.

## Features

- Generate and accept payments from multiple Cashu mints
- Full implementation of the NUT-18 payment protocol
- Support for both SAT and USD denominations
- Simple REST API for payment request generation and processing
- Persistent storage of payment quotes and statuses
- Easy configuration via TOML config file

## Installation

### Prerequisites

- Rust toolchain (1.75.0+)
- Cargo

### Building from Source

```bash
# Clone the repository
git clone https://github.com/thesimplekid/cashu-payment-backend.git
cd cashu-payment-backend

# Build the project
cargo build --release
```

## Configuration

On first run, the application will create an example configuration file at `~/.cashu-payment/example.config.toml`. Copy this to `~/.cashu-payment/config.toml` and modify it according to your needs:

```toml
# Payment backend configuration
[pos]
# HTTP API server address
listen_host = "127.0.0.1"
listen_port = 3000
# Payment URL for the backend
payment_url = "https://your-pos-payment-url.com"
# List of accepted Cashu mint URLs
accepted_mints = [
  "https://mint1.example.com",
  "https://mint2.example.com"
]
```

## Usage

### Running the Server

```bash
./target/release/cashu-payment-backend
```

### API Endpoints

- `GET /create?amount=<amount>&unit=<unit>` - Generate a new NUT-18 payment request
- `GET /check/{id}` - Check the status of a payment request
- `POST /payment` - Process a Cashu NUT-18 payment

## Development

This project uses the Nix package manager for development environment setup. If you have Nix installed:

```bash
# Enter development shell
nix develop
```
