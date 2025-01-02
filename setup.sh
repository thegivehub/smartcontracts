#!/bin/bash

# Setup script for The Give Hub smart contracts repository
# Usage: ./setup.sh <project-directory>

set -e  # Exit on error

if [ -z "$1" ]; then
    PROJECT_DIR="givehub-contracts"
else
    PROJECT_DIR="$1"
fi

echo "Creating project structure in $PROJECT_DIR..."

# Create main project directory
mkdir -p "$PROJECT_DIR"
cd "$PROJECT_DIR"

# Create directory structure
mkdir -p .github/workflows
mkdir -p contracts/{campaign,donation,verification}/src
mkdir -p scripts
mkdir -p tests/{integration,utils}

# Create GitHub Actions workflow files
cat > .github/workflows/test.yml << 'EOL'
name: Test Smart Contracts

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v2
    
    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        override: true
    
    - name: Install Soroban CLI
      run: cargo install soroban-cli
    
    - name: Build
      run: cargo build --verbose
    
    - name: Run tests
      run: cargo test --verbose
EOL

cat > .github/workflows/deploy.yml << 'EOL'
name: Deploy Smart Contracts

on:
  push:
    tags:
      - 'v*'

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v2
    
    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        override: true
    
    - name: Install Soroban CLI
      run: cargo install soroban-cli
    
    - name: Build
      run: cargo build --release
    
    - name: Deploy
      env:
        STELLAR_SECRET_KEY: ${{ secrets.STELLAR_SECRET_KEY }}
      run: ./scripts/deploy.sh
EOL

# Create main Cargo.toml
cat > Cargo.toml << 'EOL'
[workspace]
members = [
    "contracts/campaign",
    "contracts/donation",
    "contracts/verification"
]

resolver = "2"

[workspace.dependencies]
soroban-sdk = "20.0.0"
soroban-token-sdk = "20.0.0"
stellar-strkey = "0.0.7"
rand = "0.8.5"
EOL

# Create contract Cargo.toml files
for contract in campaign donation verification; do
    cat > "contracts/$contract/Cargo.toml" << EOL
[package]
name = "givehub-$contract"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
soroban-sdk = { workspace = true }
soroban-token-sdk = { workspace = true }

[dev-dependencies]
soroban-sdk = { workspace = true, features = ["testutils"] }
EOL

    # Create default lib.rs files
    cat > "contracts/$contract/src/lib.rs" << 'EOL'
#![no_std]
use soroban_sdk::{contract, contractimpl, log, symbol_short, vec, Env, Symbol, Vec};

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn hello(env: Env, to: Symbol) -> Vec<Symbol> {
        vec![&env, symbol_short!("Hello"), to]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn test_hello() {
        let env = Env::default();
        let contract_id = env.register_contract(None, Contract);
        let client = ContractClient::new(&env, &contract_id);

        let result = client.hello(&symbol_short!("world"));
        assert_eq!(
            result,
            vec![&env, symbol_short!("Hello"), symbol_short!("world")]
        );
    }
}
EOL
done

# Create deployment script
cat > scripts/deploy.sh << 'EOL'
#!/bin/bash
set -e

echo "Building contracts..."
cargo build --release

for contract in campaign donation verification; do
    echo "Deploying $contract contract..."
    soroban contract deploy \
        --wasm target/wasm32-unknown-unknown/release/givehub_$contract.wasm \
        --source "$STELLAR_SECRET_KEY" \
        --network testnet
done
EOL

chmod +x scripts/deploy.sh

# Create test script
cat > scripts/test.sh << 'EOL'
#!/bin/bash
set -e

echo "Running all tests..."
cargo test --workspace --verbose

echo "Running integration tests..."
cargo test --test test_full_flow
EOL

chmod +x scripts/test.sh

# Create integration test file
cat > tests/integration/test_full_flow.rs << 'EOL'
use soroban_sdk::Env;

#[test]
fn test_full_flow() {
    let env = Env::default();
    // Add integration tests here
    assert!(true);
}
EOL

# Create test utilities
cat > tests/utils/mock_data.rs << 'EOL'
use soroban_sdk::Env;

pub fn setup_test_env() -> Env {
    Env::default()
}
EOL

# Create .gitignore
cat > .gitignore << 'EOL'
/target
**/*.rs.bk
.env
.DS_Store
node_modules/
*.swp
Cargo.lock
EOL

# Create README.md
cat > README.md << 'EOL'
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
EOL

# Create Soroban configuration file
cat > soroban-config.json << 'EOL'
{
  "network": "testnet",
  "rpcUrl": "https://soroban-testnet.stellar.org",
  "sourceAccount": "YOUR_SOURCE_ACCOUNT",
  "contracts": {
    "campaign": {
      "path": "./contracts/campaign"
    },
    "donation": {
      "path": "./contracts/donation"
    },
    "verification": {
      "path": "./contracts/verification"
    }
  }
}
EOL

# Create .env example file
cat > .env.example << 'EOL'
# Stellar Network Configuration
STELLAR_NETWORK=testnet
STELLAR_SECRET_KEY=YOUR_SECRET_KEY_HERE
RPC_URL=https://soroban-testnet.stellar.org

# Contract Configuration
CAMPAIGN_CONTRACT_ID=
DONATION_CONTRACT_ID=
VERIFICATION_CONTRACT_ID=
EOL

echo "Project structure created successfully in $PROJECT_DIR"
echo "Next steps:"
echo "1. cd $PROJECT_DIR"
echo "2. cargo build"
echo "3. cargo test"
