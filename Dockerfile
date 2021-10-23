FROM alpine:3.14

RUN apk update --no-cache
RUN apk upgrade --no-cache

RUN addgroup -S runner && adduser -S runner -G runner
RUN mkdir /app && chown -R runner:runner /app
WORKDIR /app

COPY target/x86_64-unknown-linux-musl/release/cookie-clicker-afk /usr/local/bin/cookie-clicker-afk
RUN chmod +x /usr/local/bin/cookie-clicker-afk

USER runner

CMD [ "/usr/local/bin/cookie-clicker-afk" ]
