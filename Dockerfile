FROM rust:1-slim-bookworm AS backend
WORKDIR /app/backend
COPY backend/Cargo.toml backend/Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && cargo build --release && rm -r src
COPY backend/src src
RUN touch src/main.rs && cargo build --release

FROM oven/bun:1 AS frontend
WORKDIR /app/frontend
COPY frontend/package.json frontend/bun.lock ./
RUN bun install --frozen-lockfile
COPY frontend .
RUN bun run build

FROM gcr.io/distroless/cc-debian12
WORKDIR /app/backend
COPY --from=backend /app/backend/target/release/backend /usr/local/bin/backend
COPY --from=frontend /app/frontend/build /app/frontend/build
COPY backend/map.json map.json
EXPOSE 4400
CMD ["backend"]
