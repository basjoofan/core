# build stage use rust as base image
FROM rust:1.91-alpine AS builder
COPY ../ .
RUN apk add --no-cache musl-dev
RUN cargo build --release
# runtime stage use alpine as base image
FROM alpine:3.23.0
# copy compiled file from build stage
COPY --from=builder ./target/release/basjoofan /usr/bin