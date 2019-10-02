fn main() {
    tonic_build::compile_protos("proto/shardik.proto").unwrap();
}
