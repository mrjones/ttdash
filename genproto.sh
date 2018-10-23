# 1. Install generic, Google protoc:
# - Best is to get a new version from: https://github.com/google/protobuf/releases
# - You can also do: sudo apt-get install protobuf-compiler  (But this is pretty old).
# 2. Install rust protoc plugin:
# - cargo install protobuf
~/downloads/proto/bin/protoc --proto_path ./proto/ --plugin ~/.cargo/bin/protoc-gen-rust --rust_out ./src/ proto/*.proto
