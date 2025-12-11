# Dockerfile

FROM rust:1.82-slim as build

WORKDIR /usr/src/shrimpl
COPY . .
RUN cargo build --release --bin shrimpl

FROM debian:bookworm-slim
WORKDIR /opt/shrimpl
COPY --from=build /usr/src/shrimpl/target/release/shrimpl /usr/local/bin/shrimpl
COPY app.shr ./app.shr
COPY config ./config
EXPOSE 3000
ENV SHRIMPL_ENV=prod
CMD ["shrimpl", "--file", "app.shr", "run"]
