FROM golang:1.21-alpine

WORKDIR /app

COPY . .

RUN go mod init kiwi-sync && \
    go get golang.org/x/crypto/bcrypt && \
    go build -o kiwi-sync

FROM alpine:latest

WORKDIR /app

COPY --from=0 /app/kiwi-sync .

RUN mkdir -p /opt/kiwi/data /opt/kiwi/users && \
    chmod 755 /opt/kiwi/data /opt/kiwi/users

EXPOSE 8080

CMD ["./kiwi-sync"] 