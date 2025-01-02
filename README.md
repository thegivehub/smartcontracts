# The Give Hub Smart Contracts

This repository contains the Soroban smart contracts for The Give Hub platform, enabling secure and transparent crowdfunding for remote communities.

## Structure

- `contracts/`: Individual smart contracts
- `tests/`: Integration and unit tests
- `scripts/`: Deployment and testing scripts

## Prerequisites

- Rust and Cargo
- Soroban CLI
- Stellar account for deployment

## Setup

1. Install dependencies:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   cargo install soroban-cli
   ```

2. Build contracts:
   ```bash
   cargo build --release
   ```

3. Run tests:
   ```bash
   cargo test
   ```

## Deployment

1. Configure environment:
   ```bash
   cp .env.example .env
   # Edit .env with your credentials
   ```

2. Deploy contracts:
   ```bash
   ./scripts/deploy.sh
   ```

## Testing

Run integration tests:
```bash
cargo test --test test_full_flow
```

## Security

- All contracts are audited
- Multi-signature requirements for critical operations
- Comprehensive test coverage

## Contributing

1. Fork the repository
2. Create feature branch
3. Submit pull request
4. Ensure tests pass

## License

MIT License
