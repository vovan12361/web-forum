########## BUILD STAGE ##########
FROM golang:1.23 AS builder

WORKDIR /src

# deps
COPY go.mod .
RUN go mod download

# source code
COPY . .

# compile
RUN mkdir -p .build
RUN CGO_ENABLED=0 GOOS=linux go build -o ./build/ cmd/post-service/main.go

########## RUN STAGE ##########
FROM alpine:latest

WORKDIR /app

COPY --from=builder /src/build .
COPY --from=builder /src/.env .
COPY --from=builder /src/migrations ./migrations

ENTRYPOINT [ "./main" ]