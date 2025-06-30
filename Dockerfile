FROM messense/rust-musl-cross:x86_64-musl as builder
WORKDIR /proj
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM scratch
COPY --from=builder /proj/target/x86_64-unknown-linux-musl/release/api-deployment-example /api-deployment-example
ENTRYPOINT [ "/proj" ]
EXPOSE 8080