FROM messense/rust-musl-cross:x86_64-musl as builder

# Let openssl-sys know to build OpenSSL from source
ENV OPENSSL_STATIC=1
ENV OPENSSL_DIR=/usr/local
ENV OPENSSL_LIB_DIR=/usr/local/lib
ENV OPENSSL_INCLUDE_DIR=/usr/local/include
ENV PKG_CONFIG_ALLOW_CROSS=1

WORKDIR /proj
COPY . .

RUN cargo build --release --target x86_64-unknown-linux-musl

FROM scratch
COPY --from=builder /proj/target/x86_64-unknown-linux-musl/release/proj /proj
EXPOSE 8080
ENTRYPOINT ["/proj"]
