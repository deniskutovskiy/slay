# --- Build Stage ---
FROM rust:1-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install trunk and wasm target
RUN cargo install --locked trunk
RUN rustup target add wasm32-unknown-unknown

WORKDIR /app

# 1. Copy only manifests to cache dependencies
COPY Cargo.toml Cargo.lock ./
COPY core/Cargo.toml core/Cargo.toml
COPY ui/Cargo.toml ui/Cargo.toml

# 2. Create dummy source files to compile dependencies
RUN mkdir -p core/src && touch core/src/lib.rs \
    && mkdir -p ui/src && echo "fn main() {}" > ui/src/main.rs

# 3. Build dependencies (this layer will be cached)
# We use cargo build first to specifically cache the wasm32 target
RUN cargo build --target wasm32-unknown-unknown --release -p slay-ui

# 4. Now copy real source code
COPY . .

# We need to 'touch' the files to ensure cargo notices the change 
# after the dummy build in the previous layer
RUN touch core/src/lib.rs ui/src/main.rs

# 5. Final build with Trunk
WORKDIR /app/ui
RUN trunk build --release

# --- Serve Stage ---
FROM nginx:alpine

# Copy custom nginx config
COPY deploy/nginx.conf /etc/nginx/conf.d/default.conf

# Copy build artifacts from builder stage
COPY --from=builder /app/ui/dist /usr/share/nginx/html

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
