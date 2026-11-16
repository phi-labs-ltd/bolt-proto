mod generate;

use std::io::{Read, Write};

use prost::Message;
use prost_types::compiler::{code_generator_response, CodeGeneratorResponse};

/// The request fields we read. `proto_file` stays raw bytes: decoding it with
/// `prost_types` would drop the extension options we are here to read.
#[derive(Clone, PartialEq, Message)]
struct CodeGeneratorRequest {
    #[prost(string, repeated, tag = "1")]
    file_to_generate: Vec<String>,
    #[prost(bytes = "vec", repeated, tag = "15")]
    proto_file: Vec<Vec<u8>>,
}

const OUTPUT_FILE: &str = "bolt_ext.rs";

fn main() {
    let mut input = Vec::new();
    std::io::stdin()
        .read_to_end(&mut input)
        .expect("read CodeGeneratorRequest from stdin");

    let response = match run(&input) {
        Ok(content) => CodeGeneratorResponse {
            supported_features: Some(code_generator_response::Feature::Proto3Optional as u64),
            file: vec![code_generator_response::File {
                name: Some(OUTPUT_FILE.to_owned()),
                content: Some(content),
                ..Default::default()
            }],
            ..Default::default()
        },
        Err(error) => CodeGeneratorResponse {
            error: Some(error),
            ..Default::default()
        },
    };

    std::io::stdout()
        .write_all(&response.encode_to_vec())
        .expect("write CodeGeneratorResponse to stdout");
}

fn run(input: &[u8]) -> Result<String, String> {
    let request = CodeGeneratorRequest::decode(input).map_err(|e| e.to_string())?;
    let mut pool = prost_reflect::DescriptorPool::new();
    for file in &request.proto_file {
        pool.decode_file_descriptor_proto(file.as_slice())
            .map_err(|e| e.to_string())?;
    }
    generate::generate(&pool, &request.file_to_generate)
}
