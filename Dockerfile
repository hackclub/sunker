FROM rust:1-slim-bookworm AS backend
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && cargo build --release && rm -r src
COPY build.rs build.rs
COPY migrations migrations
COPY src src
RUN touch src/main.rs && cargo build --release

FROM oven/bun:1 AS frontend
WORKDIR /app/frontend
COPY frontend/package.json frontend/bun.lock ./
RUN bun install --frozen-lockfile
COPY frontend .
RUN bun run build

FROM gcr.io/distroless/cc-debian12
WORKDIR /app
COPY --from=backend /app/target/release/backend /usr/local/bin/backend
COPY --from=frontend /app/frontend/build /app/frontend/build
COPY map.json map.json
EXPOSE 4400
CMD ["backend"]
