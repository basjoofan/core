# build stage use rust as base image
FROM rust:1.90-alpine AS builder
COPY ../ .
RUN apk add --no-cache musl-dev
RUN cargo build --features univ --release
# runtime stage use alpine as base image
FROM alpine:3.22.1
# copy compiled file from build stage
COPY --from=builder ./target/release/basjoofan /usr/bin